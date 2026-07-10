use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::{EmberError, EmberResult};

pub const MAX_PROTOCOL_UNITS: u128 = 1_000_000_000_000_000_000;
pub const BPS_DENOMINATOR: u32 = 10_000;

#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Units(u128);

#[derive(
    Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Bps(u32);

impl Units {
    pub const ZERO: Self = Self(0);

    pub fn new(value: u128) -> EmberResult<Self> {
        if value > MAX_PROTOCOL_UNITS {
            return Err(EmberError::AmountOutOfRange {
                value,
                max: MAX_PROTOCOL_UNITS,
            });
        }
        Ok(Self(value))
    }

    pub const fn zero() -> Self {
        Self::ZERO
    }

    pub const fn raw(self) -> u128 {
        self.0
    }

    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn checked_add(self, rhs: Self) -> EmberResult<Self> {
        let value = self
            .0
            .checked_add(rhs.0)
            .ok_or(EmberError::AmountOverflow)?;
        Self::new(value)
    }

    pub fn checked_sub(self, rhs: Self) -> EmberResult<Self> {
        let value = self
            .0
            .checked_sub(rhs.0)
            .ok_or(EmberError::AmountUnderflow)?;
        Self::new(value)
    }

    pub fn saturating_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    pub fn checked_mul(self, rhs: u128) -> EmberResult<Self> {
        let value = self.0.checked_mul(rhs).ok_or(EmberError::AmountOverflow)?;
        Self::new(value)
    }

    pub fn checked_div(self, rhs: u128) -> EmberResult<Self> {
        if rhs == 0 {
            return Err(EmberError::DivisionByZero);
        }
        Self::new(self.0 / rhs)
    }

    pub fn mul_bps(self, bps: Bps) -> EmberResult<Self> {
        let value = self
            .0
            .checked_mul(u128::from(bps.raw()))
            .ok_or(EmberError::AmountOverflow)?
            / u128::from(BPS_DENOMINATOR);
        Self::new(value)
    }

    pub fn ratio_bps(self, denominator: Self) -> EmberResult<Bps> {
        if denominator.is_zero() {
            return Err(EmberError::DivisionByZero);
        }
        let value = self
            .0
            .checked_mul(u128::from(BPS_DENOMINATOR))
            .ok_or(EmberError::AmountOverflow)?
            / denominator.0;
        Bps::new(value.min(u128::from(BPS_DENOMINATOR)) as u32)
    }

    pub fn ratio_bps_uncapped(self, denominator: Self) -> EmberResult<Bps> {
        if denominator.is_zero() {
            return Err(EmberError::DivisionByZero);
        }
        let value = self
            .0
            .checked_mul(u128::from(BPS_DENOMINATOR))
            .ok_or(EmberError::AmountOverflow)?
            / denominator.0;
        Bps::new(value.min(1_000_000) as u32)
    }

    pub fn min(self, rhs: Self) -> Self {
        if self <= rhs { self } else { rhs }
    }

    pub fn max(self, rhs: Self) -> Self {
        if self >= rhs { self } else { rhs }
    }
}

impl Bps {
    pub const ZERO: Self = Self(0);
    pub const FULL: Self = Self(BPS_DENOMINATOR);

    pub fn new(value: u32) -> EmberResult<Self> {
        if value > 1_000_000 {
            return Err(EmberError::BpsOutOfRange { value });
        }
        Ok(Self(value))
    }

    pub const fn zero() -> Self {
        Self::ZERO
    }

    pub const fn full() -> Self {
        Self::FULL
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    pub fn checked_add(self, rhs: Self) -> EmberResult<Self> {
        Self::new(
            self.0
                .checked_add(rhs.0)
                .ok_or(EmberError::BpsOutOfRange { value: u32::MAX })?,
        )
    }

    pub fn checked_sub(self, rhs: Self) -> EmberResult<Self> {
        Self::new(self.0.saturating_sub(rhs.0))
    }

    pub fn min(self, rhs: Self) -> Self {
        if self <= rhs { self } else { rhs }
    }

    pub fn max(self, rhs: Self) -> Self {
        if self >= rhs { self } else { rhs }
    }

    pub fn clamp_to_full(self) -> Self {
        Self(self.0.min(BPS_DENOMINATOR))
    }
}

impl Add for Units {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for Units {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for Units {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for Units {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl TryFrom<u128> for Units {
    type Error = EmberError;

    fn try_from(value: u128) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<u64> for Units {
    fn from(value: u64) -> Self {
        Self(u128::from(value))
    }
}

impl TryFrom<u32> for Bps {
    type Error = EmberError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl fmt::Display for Units {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

impl fmt::Display for Bps {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}
