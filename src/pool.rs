use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{AssetId, EmberError, EmberResult, ReservationId, RouteId, Units};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PoolReservation {
    pub id: ReservationId,
    pub route: RouteId,
    pub amount: Units,
    pub paid: Units,
    pub penalty: Units,
    pub released: Units,
    pub open: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PoolAccount {
    pub asset: AssetId,
    pub available: Units,
    pub reserved: Units,
    pub paid: Units,
    pub penalties: Units,
    pub released: Units,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RebatePool {
    account: PoolAccount,
    reservations: BTreeMap<ReservationId, PoolReservation>,
}

impl RebatePool {
    pub fn new(asset: AssetId) -> Self {
        Self {
            account: PoolAccount {
                asset,
                available: Units::zero(),
                reserved: Units::zero(),
                paid: Units::zero(),
                penalties: Units::zero(),
                released: Units::zero(),
            },
            reservations: BTreeMap::new(),
        }
    }

    pub fn fund(&mut self, amount: Units) -> EmberResult<()> {
        self.account.available = self.account.available.checked_add(amount)?;
        Ok(())
    }

    pub fn reserve(
        &mut self,
        id: ReservationId,
        route: RouteId,
        amount: Units,
    ) -> EmberResult<PoolReservation> {
        if self.reservations.contains_key(&id) {
            return Err(EmberError::ReservationAlreadyExists(id));
        }
        if self.account.available < amount {
            return Err(EmberError::InsufficientPoolLiquidity {
                available: self.account.available,
                required: amount,
            });
        }
        self.account.available = self.account.available.checked_sub(amount)?;
        self.account.reserved = self.account.reserved.checked_add(amount)?;
        let reservation = PoolReservation {
            id,
            route,
            amount,
            paid: Units::zero(),
            penalty: Units::zero(),
            released: Units::zero(),
            open: true,
        };
        self.reservations.insert(id, reservation.clone());
        Ok(reservation)
    }

    pub fn apply_execution(
        &mut self,
        id: ReservationId,
        rebate: Units,
        penalty: Units,
    ) -> EmberResult<PoolReservation> {
        let reservation = self
            .reservations
            .get_mut(&id)
            .ok_or(EmberError::ReservationNotFound(id))?;
        if !reservation.open {
            return Err(EmberError::Policy("reservation already closed".to_owned()));
        }
        let committed = rebate.checked_add(penalty)?;
        if committed > reservation.amount {
            return Err(EmberError::Policy(
                "reservation execution exceeds reserved budget".to_owned(),
            ));
        }
        let release = reservation.amount.checked_sub(committed)?;
        reservation.paid = rebate;
        reservation.penalty = penalty;
        reservation.released = release;
        reservation.open = false;
        self.account.reserved = self.account.reserved.checked_sub(reservation.amount)?;
        self.account.paid = self.account.paid.checked_add(rebate)?;
        self.account.penalties = self.account.penalties.checked_add(penalty)?;
        self.account.released = self.account.released.checked_add(release)?;
        self.account.available = self.account.available.checked_add(release)?;
        Ok(reservation.clone())
    }

    pub fn account(&self) -> &PoolAccount {
        &self.account
    }

    pub fn reservations(&self) -> impl Iterator<Item = &PoolReservation> {
        self.reservations.values()
    }

    pub fn reservation(&self, id: ReservationId) -> EmberResult<&PoolReservation> {
        self.reservations
            .get(&id)
            .ok_or(EmberError::ReservationNotFound(id))
    }

    pub fn conservation_holds(&self) -> bool {
        for reservation in self.reservations.values() {
            let Ok(closed_total) = reservation
                .paid
                .checked_add(reservation.penalty)
                .and_then(|value| value.checked_add(reservation.released))
            else {
                return false;
            };
            if !reservation.open && closed_total != reservation.amount {
                return false;
            }
        }
        true
    }
}
