use crate::{EmberError, EmberResult, SCENARIO_NAMES, run_named_scenario};

pub fn run() -> EmberResult<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [] => print_scenario("normal"),
        [flag] if flag == "--list" || flag == "list" => {
            for scenario in SCENARIO_NAMES {
                println!("{scenario}");
            }
            Ok(())
        }
        [name] => print_scenario(name),
        [command, name] if command == "scenario" => print_scenario(name),
        [command, name] if command == "validate" => {
            let report = run_named_scenario(name)?;
            if !report.invariants.ok() {
                return Err(EmberError::Policy(format!(
                    "scenario {name} produced invalid invariants"
                )));
            }
            println!("ok {name}");
            Ok(())
        }
        _ => Err(EmberError::InvalidCommand(args.join(" "))),
    }
}

fn print_scenario(name: &str) -> EmberResult<()> {
    let report = run_named_scenario(name)?;
    let encoded = serde_json::to_string_pretty(&report)
        .map_err(|error| EmberError::Serialization(error.to_string()))?;
    println!("{encoded}");
    Ok(())
}
