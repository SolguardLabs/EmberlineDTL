use serde::{Deserialize, Serialize};

use crate::{
    AccountingSnapshot, Admission, AssetSpec, Digest, EmberResult, Event, InvariantSet,
    OperatorProfile, PoolReservation, RebatePool, RebateQuote, RoutePlan, SettlementEngine,
    SettlementReceipt, Units,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PoolReport {
    pub asset_symbol: String,
    pub available: Units,
    pub reserved: Units,
    pub paid: Units,
    pub penalties: Units,
    pub released: Units,
    pub reservation_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteReport {
    pub plan: RoutePlan,
    pub admission: Option<Admission>,
    pub reservation: Option<PoolReservation>,
    pub receipt: Option<SettlementReceipt>,
    pub quote: Option<RebateQuote>,
    pub executed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScenarioReport {
    pub lab: String,
    pub scenario: String,
    pub network_id: u64,
    pub clock_ms: u64,
    pub state_digest: Digest,
    pub assets: Vec<AssetSpec>,
    pub operators: Vec<OperatorProfile>,
    pub routes: Vec<RouteReport>,
    pub pool: PoolReport,
    pub accounting: AccountingSnapshot,
    pub invariants: InvariantSet,
    pub events: Vec<Event>,
    pub reference_controls: usize,
    pub reference_digest: Digest,
    pub notes: Vec<String>,
}

impl PoolReport {
    pub fn from_pool(pool: &RebatePool, asset_symbol: String) -> Self {
        let account = pool.account();
        Self {
            asset_symbol,
            available: account.available,
            reserved: account.reserved,
            paid: account.paid,
            penalties: account.penalties,
            released: account.released,
            reservation_count: pool.reservations().count(),
        }
    }
}

impl ScenarioReport {
    pub fn from_engine(engine: &SettlementEngine, scenario: &str) -> EmberResult<Self> {
        let asset = engine.assets.get(engine.config.primary_asset)?;
        let routes = engine
            .routes
            .values()
            .map(|state| RouteReport {
                plan: state.plan.clone(),
                admission: state.admission.clone(),
                reservation: state
                    .reservation
                    .and_then(|id| engine.pool.reservation(id).ok().cloned()),
                receipt: state.receipt.clone(),
                quote: state.quote.clone(),
                executed: state.executed,
            })
            .collect::<Vec<_>>();
        let accounting = engine.accounting_snapshot();
        let invariants = engine.invariants();
        let reference_digest = crate::reference::reference_digest()?;
        let mut report = Self {
            lab: "EmberlineDTL".to_owned(),
            scenario: scenario.to_owned(),
            network_id: engine.config.network_id,
            clock_ms: engine.clock_ms,
            state_digest: Digest::zero(),
            assets: engine.assets.values().cloned().collect(),
            operators: engine.operators.values().cloned().collect(),
            routes,
            pool: PoolReport::from_pool(&engine.pool, asset.symbol.clone()),
            accounting,
            invariants,
            events: engine.events.clone(),
            reference_controls: crate::reference::CONTROL_MATRIX.len(),
            reference_digest,
            notes: vec![
                "deterministic-local-simulation".to_owned(),
                "rebate-policy-v1".to_owned(),
            ],
        };
        report.state_digest = Digest::from_serializable(
            "emberline-report-state",
            &(
                &report.scenario,
                report.clock_ms,
                &report.pool,
                &report.accounting,
                &report.invariants,
                &report.routes,
            ),
        )?;
        Ok(report)
    }
}
