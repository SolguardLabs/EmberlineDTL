use crate::{
    Bps, EmberError, EmberResult, RouteLeg, RoutePlan, ScenarioReport, SettlementEngine, Units,
};

pub const SCENARIO_NAMES: &[&str] = &["normal", "rebate", "penalty", "pool", "crossflow"];

pub fn run_named_scenario(name: &str) -> EmberResult<ScenarioReport> {
    let mut engine = SettlementEngine::bootstrap()?;
    match name {
        "normal" => normal_execution(&mut engine)?,
        "rebate" => rebate_comparison(&mut engine)?,
        "penalty" => penalty_window(&mut engine)?,
        "pool" => pool_accounting(&mut engine)?,
        "crossflow" => crossflow_route(&mut engine)?,
        other => return Err(EmberError::UnknownScenario(other.to_owned())),
    }
    ScenarioReport::from_engine(&engine, name)
}

fn normal_execution(engine: &mut SettlementEngine) -> EmberResult<()> {
    let asset = engine.config.primary_asset;
    let operator = engine.operators.id_for("ember-solver")?;
    let route = RoutePlan::new(
        "normal-mainnet-euro",
        operator,
        asset,
        Units::new(1_000_000)?,
        engine.clock_ms,
        engine.clock_ms + 12_000,
        4,
    )
    .add_leg(
        RouteLeg::new(
            "mainnet-clear",
            "solana-mainnet",
            "emberline-eu",
            asset,
            Units::new(6_000)?,
            2_100,
        )?
        .with_compensation(Units::new(900)?)
        .with_external_fee(Units::new(350)?),
    )
    .add_leg(
        RouteLeg::new(
            "euro-final",
            "emberline-eu",
            "bank-eu",
            asset,
            Units::new(4_500)?,
            1_900,
        )?
        .with_compensation(Units::new(700)?)
        .with_liquidity_weight(Bps::new(6_500)?),
    );
    let route_id = route.id;
    engine.submit_route(route)?;
    engine.execute_route(route_id, Units::new(200)?, Units::new(100)?, 4_600)?;
    Ok(())
}

fn rebate_comparison(engine: &mut SettlementEngine) -> EmberResult<()> {
    let asset = engine.config.primary_asset;
    let prime = engine.operators.id_for("north-bridge")?;
    let standard = engine.operators.id_for("delta-runner")?;
    let prime_route = RoutePlan::new(
        "preferred-fast-lane",
        prime,
        asset,
        Units::new(2_400_000)?,
        engine.clock_ms,
        engine.clock_ms + 9_000,
        8,
    )
    .add_leg(
        RouteLeg::new(
            "prime-source",
            "solana-mainnet",
            "emberline-hub",
            asset,
            Units::new(11_000)?,
            1_200,
        )?
        .with_compensation(Units::new(4_800)?)
        .with_external_fee(Units::new(600)?),
    )
    .add_leg(
        RouteLeg::new(
            "prime-destination",
            "emberline-hub",
            "arbitrum-settlement",
            asset,
            Units::new(8_400)?,
            1_400,
        )?
        .with_compensation(Units::new(2_700)?),
    );
    let standard_route = RoutePlan::new(
        "standard-lane",
        standard,
        asset,
        Units::new(2_400_000)?,
        engine.clock_ms,
        engine.clock_ms + 13_000,
        3,
    )
    .add_leg(
        RouteLeg::new(
            "standard-source",
            "solana-mainnet",
            "emberline-us",
            asset,
            Units::new(10_500)?,
            2_600,
        )?
        .with_compensation(Units::new(800)?)
        .with_external_fee(Units::new(500)?),
    )
    .add_leg(
        RouteLeg::new(
            "standard-destination",
            "emberline-us",
            "arbitrum-settlement",
            asset,
            Units::new(8_600)?,
            2_900,
        )?
        .with_compensation(Units::new(500)?),
    );
    let prime_id = prime_route.id;
    let standard_id = standard_route.id;
    engine.submit_route(prime_route)?;
    engine.execute_route(prime_id, Units::new(100)?, Units::zero(), 3_100)?;
    engine.submit_route(standard_route)?;
    engine.execute_route(standard_id, Units::new(900)?, Units::zero(), 6_700)?;
    Ok(())
}

fn penalty_window(engine: &mut SettlementEngine) -> EmberResult<()> {
    let asset = engine.config.primary_asset;
    let operator = engine.operators.id_for("delta-runner")?;
    let route = RoutePlan::new(
        "delayed-cost-window",
        operator,
        asset,
        Units::new(1_800_000)?,
        engine.clock_ms,
        engine.clock_ms + 22_000,
        2,
    )
    .add_leg(
        RouteLeg::new(
            "congested-source",
            "solana-mainnet",
            "emberline-apac",
            asset,
            Units::new(24_000)?,
            7_500,
        )?
        .with_external_fee(Units::new(2_000)?),
    )
    .add_leg(
        RouteLeg::new(
            "congested-final",
            "emberline-apac",
            "external-custody",
            asset,
            Units::new(21_000)?,
            8_300,
        )?
        .with_external_fee(Units::new(1_500)?),
    );
    let route_id = route.id;
    engine.submit_route(route)?;
    engine.execute_route(route_id, Units::new(4_000)?, Units::zero(), 21_500)?;
    Ok(())
}

fn pool_accounting(engine: &mut SettlementEngine) -> EmberResult<()> {
    let asset = engine.config.primary_asset;
    let prime = engine.operators.id_for("north-bridge")?;
    let preferred = engine.operators.id_for("ember-solver")?;
    let standard = engine.operators.id_for("delta-runner")?;
    let route_a = RoutePlan::new(
        "pool-alpha",
        prime,
        asset,
        Units::new(1_250_000)?,
        engine.clock_ms,
        engine.clock_ms + 10_000,
        7,
    )
    .add_leg(
        RouteLeg::new(
            "alpha-hop",
            "solana-mainnet",
            "emberline-hub",
            asset,
            Units::new(7_500)?,
            1_400,
        )?
        .with_compensation(Units::new(2_000)?),
    );
    let route_b = RoutePlan::new(
        "pool-beta",
        preferred,
        asset,
        Units::new(1_750_000)?,
        engine.clock_ms,
        engine.clock_ms + 12_000,
        5,
    )
    .add_leg(
        RouteLeg::new(
            "beta-hop-one",
            "solana-mainnet",
            "emberline-eu",
            asset,
            Units::new(8_800)?,
            2_000,
        )?
        .with_compensation(Units::new(1_000)?),
    )
    .add_leg(
        RouteLeg::new(
            "beta-hop-two",
            "emberline-eu",
            "bank-eu",
            asset,
            Units::new(6_200)?,
            1_800,
        )?
        .with_compensation(Units::new(600)?),
    );
    let route_c = RoutePlan::new(
        "pool-gamma",
        standard,
        asset,
        Units::new(1_100_000)?,
        engine.clock_ms,
        engine.clock_ms + 19_000,
        2,
    )
    .add_leg(
        RouteLeg::new(
            "gamma-hop",
            "solana-mainnet",
            "external-custody",
            asset,
            Units::new(18_500)?,
            6_500,
        )?
        .with_external_fee(Units::new(1_200)?),
    );
    let a = route_a.id;
    let b = route_b.id;
    let c = route_c.id;
    engine.submit_route(route_a)?;
    engine.submit_route(route_b)?;
    engine.submit_route(route_c)?;
    engine.execute_route(a, Units::new(50)?, Units::zero(), 2_100)?;
    engine.execute_route(b, Units::new(300)?, Units::new(100)?, 4_500)?;
    engine.execute_route(c, Units::new(1_000)?, Units::zero(), 12_800)?;
    Ok(())
}

fn crossflow_route(engine: &mut SettlementEngine) -> EmberResult<()> {
    let asset = engine.config.primary_asset;
    let operator = engine.operators.id_for("north-bridge")?;
    let route = RoutePlan::new(
        "crossflow-mesh",
        operator,
        asset,
        Units::new(5_000_000)?,
        engine.clock_ms,
        engine.clock_ms + 8_000,
        9,
    )
    .add_leg(
        RouteLeg::new(
            "mesh-source",
            "solana-mainnet",
            "emberline-hub-a",
            asset,
            Units::new(32_000)?,
            1_100,
        )?
        .with_compensation(Units::new(28_000)?)
        .with_external_fee(Units::new(1_200)?)
        .with_liquidity_weight(Bps::new(7_500)?),
    )
    .add_leg(
        RouteLeg::new(
            "mesh-counter",
            "emberline-hub-b",
            "solana-mainnet",
            asset,
            Units::new(31_000)?,
            1_200,
        )?
        .with_compensation(Units::new(30_000)?)
        .with_external_fee(Units::new(1_100)?)
        .with_liquidity_weight(Bps::new(7_000)?),
    )
    .add_leg(
        RouteLeg::new(
            "mesh-final",
            "emberline-hub-a",
            "institutional-final",
            asset,
            Units::new(24_000)?,
            1_300,
        )?
        .with_compensation(Units::new(20_500)?)
        .with_external_fee(Units::new(800)?)
        .with_liquidity_weight(Bps::new(8_000)?),
    );
    let route_id = route.id;
    engine.submit_route(route)?;
    engine.execute_route(route_id, Units::new(500)?, Units::new(2_000)?, 3_900)?;
    Ok(())
}
