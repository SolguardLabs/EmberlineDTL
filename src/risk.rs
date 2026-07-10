use serde::{Deserialize, Serialize};

use crate::{Bps, EmberResult, OperatorProfile, RoutePlan, Units};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RiskSignal {
    pub label: String,
    pub score_bps: Bps,
    pub weight_bps: Bps,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RiskEnvelope {
    pub route_cost_bps: Bps,
    pub operator_trust_bps: Bps,
    pub route_depth: usize,
    pub expected_time_ms: u64,
    pub reserve_pressure_bps: Bps,
    pub signals: Vec<RiskSignal>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Admission {
    pub accepted: bool,
    pub score_bps: Bps,
    pub max_reserved_cost: Units,
    pub reason: String,
}

impl RiskEnvelope {
    pub fn from_route(
        route: &RoutePlan,
        operator: &OperatorProfile,
        reserved: Units,
    ) -> EmberResult<Self> {
        let route_cost_bps = route.gross_cost().ratio_bps_uncapped(route.notional)?;
        let operator_trust_bps = operator.trust_score()?;
        let reserve_pressure_bps = reserved.ratio_bps_uncapped(operator.reserve_limit())?;
        let signals = vec![
            RiskSignal {
                label: "cost-envelope".to_owned(),
                score_bps: Bps::new(10_000u32.saturating_sub(route_cost_bps.raw().min(10_000)))?,
                weight_bps: Bps::new(3_000)?,
            },
            RiskSignal {
                label: "operator-trust".to_owned(),
                score_bps: operator_trust_bps,
                weight_bps: Bps::new(4_000)?,
            },
            RiskSignal {
                label: "reserve-pressure".to_owned(),
                score_bps: Bps::new(
                    10_000u32.saturating_sub(reserve_pressure_bps.raw().min(10_000)),
                )?,
                weight_bps: Bps::new(2_000)?,
            },
            RiskSignal {
                label: "route-depth".to_owned(),
                score_bps: Bps::new(match route.legs.len() {
                    0 => 0,
                    1 => 9_500,
                    2 => 8_800,
                    3 => 8_000,
                    _ => 7_200,
                })?,
                weight_bps: Bps::new(1_000)?,
            },
        ];
        Ok(Self {
            route_cost_bps,
            operator_trust_bps,
            route_depth: route.legs.len(),
            expected_time_ms: route.expected_time_ms(),
            reserve_pressure_bps,
            signals,
        })
    }

    pub fn weighted_score(&self) -> EmberResult<Bps> {
        let mut weighted = 0u128;
        let mut weight_total = 0u128;
        for signal in &self.signals {
            weighted += u128::from(signal.score_bps.raw()) * u128::from(signal.weight_bps.raw());
            weight_total += u128::from(signal.weight_bps.raw());
        }
        if weight_total == 0 {
            return Bps::new(0);
        }
        Bps::new((weighted / weight_total) as u32)
    }

    pub fn admit(&self, max_reserved_cost: Units, reserved: Units) -> EmberResult<Admission> {
        let score = self.weighted_score()?;
        let accepted = score.raw() >= 5_500 && reserved <= max_reserved_cost;
        let reason = if accepted {
            "admitted".to_owned()
        } else if reserved > max_reserved_cost {
            "operator reserve limit exceeded".to_owned()
        } else {
            "risk score below policy floor".to_owned()
        };
        Ok(Admission {
            accepted,
            score_bps: score,
            max_reserved_cost,
            reason,
        })
    }
}
