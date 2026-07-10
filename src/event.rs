use serde::{Deserialize, Serialize};

use crate::{AccountId, AssetId, OperatorId, ReservationId, RouteId, SettlementId, Units};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum EventKind {
    AccountOpened,
    PoolFunded,
    PoolReserved,
    RouteSubmitted,
    RouteAdmitted,
    SettlementExecuted,
    RebatePaid,
    PenaltyApplied,
    ReservationReleased,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub kind: EventKind,
    pub clock_ms: u64,
    pub route: Option<RouteId>,
    pub settlement: Option<SettlementId>,
    pub reservation: Option<ReservationId>,
    pub operator: Option<OperatorId>,
    pub account: Option<AccountId>,
    pub asset: Option<AssetId>,
    pub amount: Units,
    pub detail: String,
}

impl Event {
    pub fn new(kind: EventKind, clock_ms: u64, detail: impl Into<String>) -> Self {
        Self {
            kind,
            clock_ms,
            route: None,
            settlement: None,
            reservation: None,
            operator: None,
            account: None,
            asset: None,
            amount: Units::zero(),
            detail: detail.into(),
        }
    }

    pub fn route(mut self, route: RouteId) -> Self {
        self.route = Some(route);
        self
    }

    pub fn settlement(mut self, settlement: SettlementId) -> Self {
        self.settlement = Some(settlement);
        self
    }

    pub fn reservation(mut self, reservation: ReservationId) -> Self {
        self.reservation = Some(reservation);
        self
    }

    pub fn operator(mut self, operator: OperatorId) -> Self {
        self.operator = Some(operator);
        self
    }

    pub fn account(mut self, account: AccountId) -> Self {
        self.account = Some(account);
        self
    }

    pub fn asset(mut self, asset: AssetId) -> Self {
        self.asset = Some(asset);
        self
    }

    pub fn amount(mut self, amount: Units) -> Self {
        self.amount = amount;
        self
    }
}
