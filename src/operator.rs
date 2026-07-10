use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{Bps, EmberError, EmberResult, OperatorId, Units};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum OperatorTier {
    Observer,
    Standard,
    Preferred,
    Prime,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OperatorProfile {
    pub id: OperatorId,
    pub label: String,
    pub tier: OperatorTier,
    pub account_label: String,
    pub reputation_bps: Bps,
    pub latency_floor_ms: u64,
    pub max_reserved_cost: Units,
    pub routes_executed: u64,
    pub rebates_earned: Units,
    pub penalties_paid: Units,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OperatorBook {
    operators: BTreeMap<OperatorId, OperatorProfile>,
    by_label: BTreeMap<String, OperatorId>,
}

impl OperatorTier {
    pub const fn base_rebate_bps(self) -> u32 {
        match self {
            Self::Observer => 1_500,
            Self::Standard => 2_500,
            Self::Preferred => 4_000,
            Self::Prime => 5_500,
        }
    }

    pub const fn cost_buffer_bps(self) -> u32 {
        match self {
            Self::Observer => 1_500,
            Self::Standard => 1_250,
            Self::Preferred => 1_000,
            Self::Prime => 850,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Observer => "observer",
            Self::Standard => "standard",
            Self::Preferred => "preferred",
            Self::Prime => "prime",
        }
    }
}

impl OperatorProfile {
    pub fn new(label: &str, tier: OperatorTier, max_reserved_cost: Units) -> EmberResult<Self> {
        Ok(Self {
            id: OperatorId::named(label),
            label: label.to_owned(),
            tier,
            account_label: format!("operator:{label}"),
            reputation_bps: Bps::new(match tier {
                OperatorTier::Observer => 6_500,
                OperatorTier::Standard => 7_500,
                OperatorTier::Preferred => 8_750,
                OperatorTier::Prime => 9_400,
            })?,
            latency_floor_ms: match tier {
                OperatorTier::Observer => 4_000,
                OperatorTier::Standard => 3_000,
                OperatorTier::Preferred => 2_250,
                OperatorTier::Prime => 1_750,
            },
            max_reserved_cost,
            routes_executed: 0,
            rebates_earned: Units::zero(),
            penalties_paid: Units::zero(),
        })
    }

    pub fn reserve_limit(&self) -> Units {
        self.max_reserved_cost
    }

    pub fn record_execution(&mut self, rebate: Units, penalty: Units) -> EmberResult<()> {
        self.routes_executed = self
            .routes_executed
            .checked_add(1)
            .ok_or(EmberError::Policy(
                "operator execution counter overflow".to_owned(),
            ))?;
        self.rebates_earned = self.rebates_earned.checked_add(rebate)?;
        self.penalties_paid = self.penalties_paid.checked_add(penalty)?;
        Ok(())
    }

    pub fn trust_score(&self) -> EmberResult<Bps> {
        let execution_bonus = (self.routes_executed.min(20) as u32) * 40;
        self.reputation_bps
            .checked_add(Bps::new(execution_bonus)?)
            .map(Bps::clamp_to_full)
    }
}

impl OperatorBook {
    pub fn with_defaults() -> EmberResult<Self> {
        let mut book = Self::default();
        book.insert(OperatorProfile::new(
            "north-bridge",
            OperatorTier::Prime,
            Units::new(3_000_000)?,
        )?)?;
        book.insert(OperatorProfile::new(
            "ember-solver",
            OperatorTier::Preferred,
            Units::new(2_500_000)?,
        )?)?;
        book.insert(OperatorProfile::new(
            "delta-runner",
            OperatorTier::Standard,
            Units::new(1_750_000)?,
        )?)?;
        book.insert(OperatorProfile::new(
            "audit-observer",
            OperatorTier::Observer,
            Units::new(750_000)?,
        )?)?;
        Ok(book)
    }

    pub fn insert(&mut self, operator: OperatorProfile) -> EmberResult<()> {
        if self.operators.contains_key(&operator.id) {
            return Err(EmberError::OperatorAlreadyExists(operator.id));
        }
        self.by_label.insert(operator.label.clone(), operator.id);
        self.operators.insert(operator.id, operator);
        Ok(())
    }

    pub fn get(&self, id: OperatorId) -> EmberResult<&OperatorProfile> {
        self.operators
            .get(&id)
            .ok_or(EmberError::OperatorNotFound(id))
    }

    pub fn get_mut(&mut self, id: OperatorId) -> EmberResult<&mut OperatorProfile> {
        self.operators
            .get_mut(&id)
            .ok_or(EmberError::OperatorNotFound(id))
    }

    pub fn id_for(&self, label: &str) -> EmberResult<OperatorId> {
        self.by_label
            .get(label)
            .copied()
            .ok_or_else(|| EmberError::Policy(format!("operator label not registered: {label}")))
    }

    pub fn values(&self) -> impl Iterator<Item = &OperatorProfile> {
        self.operators.values()
    }
}
