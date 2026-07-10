use serde::{Deserialize, Serialize};

use crate::{
    Bps, EmberError, EmberResult, OperatorProfile, PoolReservation, RoutePlan, SettlementReceipt,
    Units,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RebatePolicy {
    pub target_cost_bps: Bps,
    pub max_cost_bps: Bps,
    pub target_time_ms: u64,
    pub max_time_ms: u64,
    pub reserve_buffer_bps: Bps,
    pub max_rebate_share_bps: Bps,
    pub penalty_share_bps: Bps,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SettlementPolicy {
    pub network_id: u64,
    pub min_route_notional: Units,
    pub max_route_notional: Units,
    pub rebate: RebatePolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RebateQuote {
    pub gross_reserved_cost: Units,
    pub observed_cost: Units,
    pub adjusted_cost: Units,
    pub cost_score_bps: Bps,
    pub time_score_bps: Bps,
    pub combined_score_bps: Bps,
    pub operator_rebate: Units,
    pub pool_penalty: Units,
    pub released_to_pool: Units,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub admitted: bool,
    pub reserve_amount: Units,
    pub gross_cost_bps: Bps,
    pub adjusted_cost_bps: Bps,
    pub reason: String,
}

impl Default for RebatePolicy {
    fn default() -> Self {
        Self {
            target_cost_bps: Bps::new(80).expect("static bps"),
            max_cost_bps: Bps::new(240).expect("static bps"),
            target_time_ms: 5_000,
            max_time_ms: 18_000,
            reserve_buffer_bps: Bps::new(1_100).expect("static bps"),
            max_rebate_share_bps: Bps::new(8_500).expect("static bps"),
            penalty_share_bps: Bps::new(2_500).expect("static bps"),
        }
    }
}

impl Default for SettlementPolicy {
    fn default() -> Self {
        Self {
            network_id: 41_720,
            min_route_notional: Units::new(10_000).expect("static units"),
            max_route_notional: Units::new(50_000_000).expect("static units"),
            rebate: RebatePolicy::default(),
        }
    }
}

impl SettlementPolicy {
    pub fn decide_route(
        &self,
        route: &RoutePlan,
        operator: &OperatorProfile,
    ) -> EmberResult<PolicyDecision> {
        route.validate()?;
        if route.notional < self.min_route_notional || route.notional > self.max_route_notional {
            return Ok(PolicyDecision {
                admitted: false,
                reserve_amount: Units::zero(),
                gross_cost_bps: Bps::zero(),
                adjusted_cost_bps: Bps::zero(),
                reason: "notional outside policy range".to_owned(),
            });
        }
        let gross_cost = route.gross_cost();
        let reserve_buffer = self
            .rebate
            .reserve_buffer_bps
            .checked_add(Bps::new(operator.tier.cost_buffer_bps())?)?;
        let reserve_amount = gross_cost
            .checked_add(gross_cost.mul_bps(reserve_buffer.clamp_to_full())?)?
            .min(operator.reserve_limit());
        let gross_cost_bps = route.gross_cost_bps()?;
        let adjusted_cost_bps = route.planned_effective_cost_bps()?;
        let admitted = gross_cost_bps.raw() <= self.rebate.max_cost_bps.raw().saturating_mul(2);
        let reason = if admitted {
            "route admitted".to_owned()
        } else {
            "quoted cost outside route envelope".to_owned()
        };
        Ok(PolicyDecision {
            admitted,
            reserve_amount,
            gross_cost_bps,
            adjusted_cost_bps,
            reason,
        })
    }

    pub fn quote_rebate(
        &self,
        route: &RoutePlan,
        receipt: &SettlementReceipt,
        operator: &OperatorProfile,
        reservation: &PoolReservation,
    ) -> EmberResult<RebateQuote> {
        if route.id != receipt.route {
            return Err(EmberError::Policy("receipt route mismatch".to_owned()));
        }
        if route.operator != operator.id || receipt.operator != operator.id {
            return Err(EmberError::Policy("receipt operator mismatch".to_owned()));
        }
        let observed_cost = receipt.observed_gross_cost;
        let adjusted_cost = receipt.effective_cost();
        let cost_bps = adjusted_cost.ratio_bps_uncapped(route.notional)?;
        let cost_score_bps = self.cost_score(cost_bps)?;
        let time_score_bps = self.time_score(receipt.elapsed_ms)?;
        let tier_boost = Bps::new(operator.tier.base_rebate_bps())?;
        let trust_score = operator.trust_score()?;
        let score_numerator = u128::from(cost_score_bps.raw()) * 45
            + u128::from(time_score_bps.raw()) * 35
            + u128::from(tier_boost.raw()) * 10
            + u128::from(trust_score.raw()) * 10;
        let combined_score_bps = Bps::new((score_numerator / 100) as u32)?.clamp_to_full();
        let payout_basis = reservation
            .amount
            .mul_bps(self.rebate.max_rebate_share_bps)?;
        let operator_rebate = payout_basis.mul_bps(combined_score_bps)?;
        let pool_penalty = self.penalty_for(cost_bps, receipt.elapsed_ms, reservation.amount)?;
        let committed = operator_rebate.checked_add(pool_penalty)?;
        let released_to_pool = reservation
            .amount
            .checked_sub(committed.min(reservation.amount))?;
        Ok(RebateQuote {
            gross_reserved_cost: reservation.amount,
            observed_cost,
            adjusted_cost,
            cost_score_bps,
            time_score_bps,
            combined_score_bps,
            operator_rebate: operator_rebate.min(reservation.amount),
            pool_penalty,
            released_to_pool,
        })
    }

    fn cost_score(&self, cost_bps: Bps) -> EmberResult<Bps> {
        if cost_bps <= self.rebate.target_cost_bps {
            return Bps::new(10_000);
        }
        if cost_bps >= self.rebate.max_cost_bps {
            return Bps::new(2_000);
        }
        let span = self
            .rebate
            .max_cost_bps
            .raw()
            .saturating_sub(self.rebate.target_cost_bps.raw())
            .max(1);
        let over = cost_bps
            .raw()
            .saturating_sub(self.rebate.target_cost_bps.raw());
        let decay = (over.saturating_mul(8_000)) / span;
        Bps::new(10_000u32.saturating_sub(decay))
    }

    fn time_score(&self, elapsed_ms: u64) -> EmberResult<Bps> {
        if elapsed_ms <= self.rebate.target_time_ms {
            return Bps::new(10_000);
        }
        if elapsed_ms >= self.rebate.max_time_ms {
            return Bps::new(1_500);
        }
        let span = self
            .rebate
            .max_time_ms
            .saturating_sub(self.rebate.target_time_ms)
            .max(1);
        let over = elapsed_ms.saturating_sub(self.rebate.target_time_ms);
        let decay = ((over as u128) * 8_500u128 / (span as u128)) as u32;
        Bps::new(10_000u32.saturating_sub(decay))
    }

    fn penalty_for(
        &self,
        cost_bps: Bps,
        elapsed_ms: u64,
        reservation: Units,
    ) -> EmberResult<Units> {
        let mut penalty = Units::zero();
        if cost_bps > self.rebate.max_cost_bps {
            penalty = penalty.checked_add(reservation.mul_bps(self.rebate.penalty_share_bps)?)?;
        }
        if elapsed_ms > self.rebate.max_time_ms {
            penalty = penalty.checked_add(
                reservation.mul_bps(
                    self.rebate
                        .penalty_share_bps
                        .checked_sub(Bps::new(1_000)?)?,
                )?,
            )?;
        }
        Ok(penalty.min(reservation))
    }
}
