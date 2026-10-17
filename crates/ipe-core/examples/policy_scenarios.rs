use ipe_core::policy_scenarios::ScenarioSuite;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: policy_scenarios SUITE.json [REPORT.json]")?;
    let bytes = std::fs::read(path)?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("suite exceeds 4 MiB".into());
    }
    let suite: ScenarioSuite = serde_json::from_slice(&bytes)?;
    let report = suite.run()?;
    let output = serde_json::to_string_pretty(&report)?;
    if let Some(path) = std::env::args().nth(2) {
        std::fs::write(path, &output)?;
    }
    println!("{output}");
    if !report.passed {
        std::process::exit(1);
    }
    Ok(())
}
