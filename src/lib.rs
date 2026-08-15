#![forbid(unsafe_code)]

pub mod accounting;
pub mod amount;
pub mod asset;
pub mod error;
pub mod event;
pub mod ids;
pub mod ledger;
pub mod operator;
pub mod policy;
pub mod pool;
pub mod reference;
pub mod report;
pub mod risk;
pub mod route;
pub mod runtime;
pub mod scenario;
pub mod settlement;
pub mod treasury;

pub use accounting::{AccountingSnapshot, InvariantSet};
pub use amount::{Bps, Units};
pub use asset::{AssetBook, AssetSpec};
pub use error::{EmberError, EmberResult};
pub use event::{Event, EventKind};
pub use ids::{AccountId, AssetId, Digest, OperatorId, ReservationId, RouteId, SettlementId};
pub use ledger::{AccountState, Ledger, Posting};
pub use operator::{OperatorBook, OperatorProfile, OperatorTier};
pub use policy::{PolicyDecision, RebatePolicy, RebateQuote, SettlementPolicy};
pub use pool::{PoolAccount, PoolReservation, RebatePool};
pub use report::{PoolReport, ScenarioReport};
pub use risk::{Admission, RiskEnvelope, RiskSignal};
pub use route::{RouteLeg, RoutePlan, SettlementReceipt};
pub use scenario::{SCENARIO_NAMES, run_named_scenario};
pub use settlement::{EngineConfig, SettlementEngine};
pub use treasury::{
    CapitalBand, PortfolioAssessment, TreasuryAssessment, TreasuryPolicy, TreasuryPosition,
    assess_portfolio, assess_treasury,
};
