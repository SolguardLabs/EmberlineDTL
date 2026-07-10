use serde::{Deserialize, Serialize};

use crate::{AssetId, Bps, EmberError, EmberResult, OperatorId, RouteId, SettlementId, Units};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteLeg {
    pub label: String,
    pub source_domain: String,
    pub target_domain: String,
    pub asset: AssetId,
    pub quoted_cost: Units,
    pub internal_compensation: Units,
    pub external_fee: Units,
    pub expected_time_ms: u64,
    pub liquidity_weight: Bps,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoutePlan {
    pub id: RouteId,
    pub label: String,
    pub operator: OperatorId,
    pub asset: AssetId,
    pub notional: Units,
    pub submitted_at_ms: u64,
    pub expires_at_ms: u64,
    pub priority: u8,
    pub legs: Vec<RouteLeg>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SettlementReceipt {
    pub id: SettlementId,
    pub route: RouteId,
    pub operator: OperatorId,
    pub observed_gross_cost: Units,
    pub observed_internal_compensation: Units,
    pub elapsed_ms: u64,
    pub finalized_at_ms: u64,
    pub output_notional: Units,
}

impl RouteLeg {
    pub fn new(
        label: &str,
        source_domain: &str,
        target_domain: &str,
        asset: AssetId,
        quoted_cost: Units,
        expected_time_ms: u64,
    ) -> EmberResult<Self> {
        Ok(Self {
            label: label.to_owned(),
            source_domain: source_domain.to_owned(),
            target_domain: target_domain.to_owned(),
            asset,
            quoted_cost,
            internal_compensation: Units::zero(),
            external_fee: Units::zero(),
            expected_time_ms,
            liquidity_weight: Bps::new(5_000)?,
        })
    }

    pub fn with_compensation(mut self, amount: Units) -> Self {
        self.internal_compensation = amount;
        self
    }

    pub fn with_external_fee(mut self, amount: Units) -> Self {
        self.external_fee = amount;
        self
    }

    pub fn with_liquidity_weight(mut self, bps: Bps) -> Self {
        self.liquidity_weight = bps;
        self
    }

    pub fn settlement_cost(&self) -> EmberResult<Units> {
        self.quoted_cost.checked_add(self.external_fee)
    }
}

impl RoutePlan {
    pub fn new(
        label: &str,
        operator: OperatorId,
        asset: AssetId,
        notional: Units,
        submitted_at_ms: u64,
        expires_at_ms: u64,
        priority: u8,
    ) -> Self {
        Self {
            id: RouteId::named(label),
            label: label.to_owned(),
            operator,
            asset,
            notional,
            submitted_at_ms,
            expires_at_ms,
            priority,
            legs: Vec::new(),
        }
    }

    pub fn add_leg(mut self, leg: RouteLeg) -> Self {
        self.legs.push(leg);
        self
    }

    pub fn gross_cost(&self) -> Units {
        self.legs.iter().fold(Units::zero(), |acc, leg| {
            acc.checked_add(leg.settlement_cost().unwrap_or_else(|_| Units::zero()))
                .unwrap_or_else(|_| Units::zero())
        })
    }

    pub fn internal_compensation(&self) -> Units {
        self.legs.iter().fold(Units::zero(), |acc, leg| {
            acc.checked_add(leg.internal_compensation)
                .unwrap_or_else(|_| Units::zero())
        })
    }

    pub fn expected_time_ms(&self) -> u64 {
        self.legs
            .iter()
            .map(|leg| leg.expected_time_ms)
            .sum::<u64>()
            .saturating_add((self.legs.len() as u64).saturating_sub(1) * 120)
    }

    pub fn planned_effective_cost(&self) -> Units {
        self.gross_cost()
            .saturating_sub(self.internal_compensation())
    }

    pub fn gross_cost_bps(&self) -> EmberResult<Bps> {
        self.gross_cost().ratio_bps_uncapped(self.notional)
    }

    pub fn planned_effective_cost_bps(&self) -> EmberResult<Bps> {
        self.planned_effective_cost()
            .ratio_bps_uncapped(self.notional)
    }

    pub fn validate(&self) -> EmberResult<()> {
        if self.legs.is_empty() {
            return Err(EmberError::EmptyRoute(self.id));
        }
        if self.notional.is_zero() {
            return Err(EmberError::Policy(
                "route notional must be positive".to_owned(),
            ));
        }
        for leg in &self.legs {
            if leg.asset != self.asset {
                return Err(EmberError::Policy(format!(
                    "route leg {} has mismatched asset",
                    leg.label
                )));
            }
        }
        Ok(())
    }
}

impl SettlementReceipt {
    pub fn from_route(
        route: &RoutePlan,
        nonce: u64,
        gross_delta: Units,
        compensation_delta: Units,
        elapsed_ms: u64,
        finalized_at_ms: u64,
    ) -> EmberResult<Self> {
        Ok(Self {
            id: SettlementId::derive(route.id, nonce),
            route: route.id,
            operator: route.operator,
            observed_gross_cost: route.gross_cost().checked_add(gross_delta)?,
            observed_internal_compensation: route
                .internal_compensation()
                .checked_add(compensation_delta)?,
            elapsed_ms,
            finalized_at_ms,
            output_notional: route
                .notional
                .checked_sub(route.planned_effective_cost().min(route.notional))?,
        })
    }

    pub fn effective_cost(&self) -> Units {
        self.observed_gross_cost
            .saturating_sub(self.observed_internal_compensation)
    }

    pub fn effective_cost_bps(&self, notional: Units) -> EmberResult<Bps> {
        self.effective_cost().ratio_bps_uncapped(notional)
    }
}
