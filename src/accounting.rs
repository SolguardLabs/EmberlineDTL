use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{AssetId, PoolAccount, Units};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AccountingSnapshot {
    pub asset: AssetId,
    pub pool_available: Units,
    pub pool_reserved: Units,
    pub pool_paid: Units,
    pub pool_penalties: Units,
    pub pool_released: Units,
    pub operator_rebates: Units,
    pub operator_penalties: Units,
    pub account_balances: BTreeMap<String, Units>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InvariantSet {
    pub pool_non_negative: bool,
    pub reservations_conserved: bool,
    pub operators_non_negative: bool,
    pub route_links_valid: bool,
    pub reports_deterministic: bool,
}

impl AccountingSnapshot {
    pub fn from_pool(pool: &PoolAccount, account_balances: BTreeMap<String, Units>) -> Self {
        Self {
            asset: pool.asset,
            pool_available: pool.available,
            pool_reserved: pool.reserved,
            pool_paid: pool.paid,
            pool_penalties: pool.penalties,
            pool_released: pool.released,
            operator_rebates: pool.paid,
            operator_penalties: pool.penalties,
            account_balances,
        }
    }
}

impl InvariantSet {
    pub fn all_ok() -> Self {
        Self {
            pool_non_negative: true,
            reservations_conserved: true,
            operators_non_negative: true,
            route_links_valid: true,
            reports_deterministic: true,
        }
    }

    pub fn ok(&self) -> bool {
        self.pool_non_negative
            && self.reservations_conserved
            && self.operators_non_negative
            && self.route_links_valid
            && self.reports_deterministic
    }
}
