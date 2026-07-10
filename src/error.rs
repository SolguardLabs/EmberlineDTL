use thiserror::Error;

use crate::{AccountId, AssetId, OperatorId, ReservationId, RouteId, SettlementId, Units};

pub type EmberResult<T> = Result<T, EmberError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EmberError {
    #[error("amount overflow")]
    AmountOverflow,

    #[error("amount underflow")]
    AmountUnderflow,

    #[error("amount {value} exceeds protocol max {max}")]
    AmountOutOfRange { value: u128, max: u128 },

    #[error("basis points value out of range: {value}")]
    BpsOutOfRange { value: u32 },

    #[error("division by zero")]
    DivisionByZero,

    #[error("asset not found: {0}")]
    AssetNotFound(AssetId),

    #[error("account not found: {0}")]
    AccountNotFound(AccountId),

    #[error("account already exists: {0}")]
    AccountAlreadyExists(AccountId),

    #[error("operator not found: {0}")]
    OperatorNotFound(OperatorId),

    #[error("operator already exists: {0}")]
    OperatorAlreadyExists(OperatorId),

    #[error("route not found: {0}")]
    RouteNotFound(RouteId),

    #[error("route already exists: {0}")]
    RouteAlreadyExists(RouteId),

    #[error("settlement not found: {0}")]
    SettlementNotFound(SettlementId),

    #[error("reservation not found: {0}")]
    ReservationNotFound(ReservationId),

    #[error("reservation already exists: {0}")]
    ReservationAlreadyExists(ReservationId),

    #[error("insufficient pool liquidity: available {available}, required {required}")]
    InsufficientPoolLiquidity { available: Units, required: Units },

    #[error("insufficient balance for {account}: available {available}, required {required}")]
    InsufficientBalance {
        account: AccountId,
        available: Units,
        required: Units,
    },

    #[error("route has no legs: {0}")]
    EmptyRoute(RouteId),

    #[error("route not admitted: {0}")]
    RouteNotAdmitted(RouteId),

    #[error("settlement already executed: {0}")]
    SettlementAlreadyExecuted(SettlementId),

    #[error("policy violation: {0}")]
    Policy(String),

    #[error("serialization failed: {0}")]
    Serialization(String),

    #[error("unknown scenario: {0}")]
    UnknownScenario(String),

    #[error("invalid command: {0}")]
    InvalidCommand(String),
}
