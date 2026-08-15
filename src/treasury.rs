use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{Bps, EmberError, EmberResult, Units};

const FULL_BPS: u32 = 10_000;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapitalBand {
    Nominal,
    Watch,
    Guarded,
    Critical,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TreasuryPosition {
    pub asset: String,
    pub pool_available: Units,
    pub pool_reserved: Units,
    pub scheduled_rebates: Units,
    pub accumulated_penalties: Units,
    pub expected_epoch_inflow: Units,
    pub largest_reservation: Units,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TreasuryPolicy {
    pub liquidity_haircut_bps: Bps,
    pub reservation_shock_bps: Bps,
    pub rebate_shock_bps: Bps,
    pub penalty_addon_bps: Bps,
    pub concentration_addon_bps: Bps,
    pub minimum_coverage_bps: Bps,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TreasuryAssessment {
    pub asset: String,
    pub available_liquidity: Units,
    pub stressed_reservations: Units,
    pub stressed_rebates: Units,
    pub penalty_addon: Units,
    pub concentration_addon: Units,
    pub required_liquidity: Units,
    pub surplus: Units,
    pub shortfall: Units,
    pub coverage_bps: Bps,
    pub reservation_utilization_bps: Bps,
    pub concentration_bps: Bps,
    pub band: CapitalBand,
    pub signals: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortfolioAssessment {
    pub assets: Vec<TreasuryAssessment>,
    pub total_available_liquidity: Units,
    pub total_required_liquidity: Units,
    pub total_surplus: Units,
    pub total_shortfall: Units,
    pub critical_assets: usize,
}

impl Default for TreasuryPolicy {
    fn default() -> Self {
        Self {
            liquidity_haircut_bps: Bps::new(1_000).expect("static bps"),
            reservation_shock_bps: Bps::new(2_000).expect("static bps"),
            rebate_shock_bps: Bps::new(1_500).expect("static bps"),
            penalty_addon_bps: Bps::new(5_000).expect("static bps"),
            concentration_addon_bps: Bps::new(1_250).expect("static bps"),
            minimum_coverage_bps: Bps::new(11_500).expect("static bps"),
        }
    }
}

impl TreasuryPolicy {
    pub fn validate(&self) -> EmberResult<()> {
        let bounded = [
            ("liquidity haircut", self.liquidity_haircut_bps, FULL_BPS),
            ("reservation shock", self.reservation_shock_bps, 50_000),
            ("rebate shock", self.rebate_shock_bps, 50_000),
            ("penalty addon", self.penalty_addon_bps, FULL_BPS),
            (
                "concentration addon",
                self.concentration_addon_bps,
                FULL_BPS,
            ),
            ("minimum coverage", self.minimum_coverage_bps, 50_000),
        ];
        for (name, value, maximum) in bounded {
            if value.raw() > maximum {
                return Err(EmberError::Policy(format!("{name} outside treasury range")));
            }
        }
        Ok(())
    }
}

impl TreasuryPosition {
    pub fn validate(&self) -> EmberResult<()> {
        if self.asset.trim().is_empty() {
            return Err(EmberError::Policy("treasury asset is required".to_owned()));
        }
        if self.largest_reservation > self.pool_reserved {
            return Err(EmberError::Policy(
                "largest reservation exceeds reserved balance".to_owned(),
            ));
        }
        Ok(())
    }
}

pub fn assess_treasury(
    position: &TreasuryPosition,
    policy: &TreasuryPolicy,
) -> EmberResult<TreasuryAssessment> {
    position.validate()?;
    policy.validate()?;

    let gross_liquidity = position
        .pool_available
        .checked_add(position.expected_epoch_inflow)?;
    let available_liquidity = gross_liquidity.mul_bps(Bps::new(
        FULL_BPS.saturating_sub(policy.liquidity_haircut_bps.raw()),
    )?)?;
    let stressed_reservations = scale_ceil(
        position.pool_reserved,
        FULL_BPS.saturating_add(policy.reservation_shock_bps.raw()),
    )?;
    let stressed_rebates = scale_ceil(
        position.scheduled_rebates,
        FULL_BPS.saturating_add(policy.rebate_shock_bps.raw()),
    )?;
    let penalty_addon = scale_ceil(
        position.accumulated_penalties,
        FULL_BPS.saturating_add(policy.penalty_addon_bps.raw()),
    )?;
    let concentration_addon = scale_ceil(
        position.largest_reservation,
        policy.concentration_addon_bps.raw(),
    )?;
    let required_liquidity = stressed_reservations
        .checked_add(stressed_rebates)?
        .checked_add(penalty_addon)?
        .checked_add(concentration_addon)?;
    let surplus = available_liquidity.saturating_sub(required_liquidity);
    let shortfall = required_liquidity.saturating_sub(available_liquidity);
    let coverage_bps = ratio_or_max(available_liquidity, required_liquidity)?;
    let reservation_utilization_bps = position
        .pool_reserved
        .ratio_bps(gross_liquidity.max(Units::from(1u64)))?;
    let concentration_bps = position
        .largest_reservation
        .ratio_bps(position.pool_reserved.max(Units::from(1u64)))?;
    let band = classify(
        coverage_bps,
        required_liquidity,
        policy.minimum_coverage_bps,
    )?;
    let mut assessment = TreasuryAssessment {
        asset: position.asset.clone(),
        available_liquidity,
        stressed_reservations,
        stressed_rebates,
        penalty_addon,
        concentration_addon,
        required_liquidity,
        surplus,
        shortfall,
        coverage_bps,
        reservation_utilization_bps,
        concentration_bps,
        band,
        signals: Vec::new(),
    };
    assessment.signals = signals(&assessment);
    Ok(assessment)
}

pub fn assess_portfolio(
    positions: &[TreasuryPosition],
    policy: &TreasuryPolicy,
) -> EmberResult<PortfolioAssessment> {
    let mut seen = BTreeSet::new();
    let mut assets = Vec::with_capacity(positions.len());
    for position in positions {
        let canonical = position.asset.trim().to_ascii_lowercase();
        if !seen.insert(canonical) {
            return Err(EmberError::Policy(format!(
                "duplicate treasury asset {}",
                position.asset
            )));
        }
        assets.push(assess_treasury(position, policy)?);
    }
    assets.sort_by(|left, right| left.asset.cmp(&right.asset));
    let mut total_available_liquidity = Units::zero();
    let mut total_required_liquidity = Units::zero();
    let mut total_surplus = Units::zero();
    let mut total_shortfall = Units::zero();
    let mut critical_assets = 0usize;
    for assessment in &assets {
        total_available_liquidity =
            total_available_liquidity.checked_add(assessment.available_liquidity)?;
        total_required_liquidity =
            total_required_liquidity.checked_add(assessment.required_liquidity)?;
        total_surplus = total_surplus.checked_add(assessment.surplus)?;
        total_shortfall = total_shortfall.checked_add(assessment.shortfall)?;
        if assessment.band == CapitalBand::Critical {
            critical_assets = critical_assets.saturating_add(1);
        }
    }
    Ok(PortfolioAssessment {
        assets,
        total_available_liquidity,
        total_required_liquidity,
        total_surplus,
        total_shortfall,
        critical_assets,
    })
}

fn scale_ceil(value: Units, bps: u32) -> EmberResult<Units> {
    let product = value
        .raw()
        .checked_mul(u128::from(bps))
        .ok_or(EmberError::AmountOverflow)?;
    let denominator = u128::from(FULL_BPS);
    let quotient = product / denominator;
    let rounded = quotient.saturating_add(u128::from(product % denominator != 0));
    Units::new(rounded)
}

fn ratio_or_max(numerator: Units, denominator: Units) -> EmberResult<Bps> {
    if denominator.is_zero() {
        return Bps::new(if numerator.is_zero() {
            FULL_BPS
        } else {
            1_000_000
        });
    }
    numerator.ratio_bps_uncapped(denominator)
}

fn classify(coverage: Bps, required: Units, minimum: Bps) -> EmberResult<CapitalBand> {
    if required.is_zero() || coverage.raw() >= minimum.raw().saturating_add(2_000) {
        return Ok(CapitalBand::Nominal);
    }
    if coverage >= minimum {
        return Ok(CapitalBand::Watch);
    }
    if coverage >= Bps::new(9_000)? {
        return Ok(CapitalBand::Guarded);
    }
    Ok(CapitalBand::Critical)
}

fn signals(assessment: &TreasuryAssessment) -> Vec<String> {
    let mut result = Vec::new();
    if !assessment.shortfall.is_zero() {
        result.push("treasury_shortfall".to_owned());
    }
    if assessment.concentration_bps.raw() > 5_000 {
        result.push("reservation_concentration".to_owned());
    }
    if assessment.reservation_utilization_bps.raw() > 8_000 {
        result.push("reservation_utilization_high".to_owned());
    }
    if assessment.stressed_rebates > assessment.available_liquidity {
        result.push("rebates_above_available_liquidity".to_owned());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position() -> TreasuryPosition {
        TreasuryPosition {
            asset: "USDC".to_owned(),
            pool_available: Units::new(1_000_000).unwrap(),
            pool_reserved: Units::new(200_000).unwrap(),
            scheduled_rebates: Units::new(100_000).unwrap(),
            accumulated_penalties: Units::new(10_000).unwrap(),
            expected_epoch_inflow: Units::new(50_000).unwrap(),
            largest_reservation: Units::new(50_000).unwrap(),
        }
    }

    #[test]
    fn computes_stress_waterfall() {
        let result = assess_treasury(&position(), &TreasuryPolicy::default()).unwrap();
        assert_eq!(result.available_liquidity.raw(), 945_000);
        assert_eq!(result.stressed_reservations.raw(), 240_000);
        assert_eq!(result.stressed_rebates.raw(), 115_000);
        assert_eq!(result.penalty_addon.raw(), 15_000);
        assert_eq!(result.concentration_addon.raw(), 6_250);
        assert_eq!(result.required_liquidity.raw(), 376_250);
        assert_eq!(result.surplus.raw(), 568_750);
        assert_eq!(result.band, CapitalBand::Nominal);
    }

    #[test]
    fn uses_conservative_ceiling() {
        let mut item = position();
        item.pool_reserved = Units::new(1).unwrap();
        item.scheduled_rebates = Units::new(1).unwrap();
        item.accumulated_penalties = Units::new(1).unwrap();
        item.largest_reservation = Units::new(1).unwrap();
        let result = assess_treasury(&item, &TreasuryPolicy::default()).unwrap();
        assert_eq!(result.stressed_reservations.raw(), 2);
        assert_eq!(result.stressed_rebates.raw(), 2);
        assert_eq!(result.penalty_addon.raw(), 2);
        assert_eq!(result.concentration_addon.raw(), 1);
    }

    #[test]
    fn detects_shortfall() {
        let mut item = position();
        item.pool_available = Units::new(100_000).unwrap();
        item.expected_epoch_inflow = Units::zero();
        let result = assess_treasury(&item, &TreasuryPolicy::default()).unwrap();
        assert_eq!(result.band, CapitalBand::Critical);
        assert!(result.shortfall.raw() > 0);
        assert!(result.signals.contains(&"treasury_shortfall".to_owned()));
    }

    #[test]
    fn reports_reservation_concentration() {
        let mut item = position();
        item.largest_reservation = Units::new(150_000).unwrap();
        let result = assess_treasury(&item, &TreasuryPolicy::default()).unwrap();
        assert_eq!(result.concentration_bps.raw(), 7_500);
        assert!(
            result
                .signals
                .contains(&"reservation_concentration".to_owned())
        );
    }

    #[test]
    fn rejects_invalid_position() {
        let mut item = position();
        item.largest_reservation = Units::new(200_001).unwrap();
        assert!(assess_treasury(&item, &TreasuryPolicy::default()).is_err());
    }

    #[test]
    fn rejects_invalid_policy() {
        let policy = TreasuryPolicy {
            liquidity_haircut_bps: Bps::new(10_001).unwrap(),
            ..TreasuryPolicy::default()
        };
        assert!(assess_treasury(&position(), &policy).is_err());
    }

    #[test]
    fn classifies_watch_band() {
        let mut item = position();
        item.pool_available = Units::new(500_000).unwrap();
        item.expected_epoch_inflow = Units::zero();
        let result = assess_treasury(&item, &TreasuryPolicy::default()).unwrap();
        assert_eq!(result.band, CapitalBand::Watch);
    }

    #[test]
    fn classifies_guarded_band() {
        let mut item = position();
        item.pool_available = Units::new(420_000).unwrap();
        item.expected_epoch_inflow = Units::zero();
        let result = assess_treasury(&item, &TreasuryPolicy::default()).unwrap();
        assert_eq!(result.band, CapitalBand::Guarded);
    }

    #[test]
    fn empty_book_is_nominal() {
        let empty = TreasuryPosition {
            asset: "EURO".to_owned(),
            pool_available: Units::zero(),
            pool_reserved: Units::zero(),
            scheduled_rebates: Units::zero(),
            accumulated_penalties: Units::zero(),
            expected_epoch_inflow: Units::zero(),
            largest_reservation: Units::zero(),
        };
        let result = assess_treasury(&empty, &TreasuryPolicy::default()).unwrap();
        assert_eq!(result.band, CapitalBand::Nominal);
        assert_eq!(result.coverage_bps.raw(), FULL_BPS);
    }

    #[test]
    fn portfolio_is_sorted_and_aggregated() {
        let usdc = position();
        let mut euro = position();
        euro.asset = "EURO".to_owned();
        let result = assess_portfolio(&[usdc, euro], &TreasuryPolicy::default()).unwrap();
        assert_eq!(result.assets[0].asset, "EURO");
        assert_eq!(result.assets[1].asset, "USDC");
        assert_eq!(result.total_available_liquidity.raw(), 1_890_000);
        assert_eq!(result.critical_assets, 0);
    }

    #[test]
    fn portfolio_rejects_duplicate_asset() {
        let first = position();
        let mut second = position();
        second.asset = " usdc ".to_owned();
        assert!(assess_portfolio(&[first, second], &TreasuryPolicy::default()).is_err());
    }
}
