mod support;
use clap::{CommandFactory, Parser};
use std::{path::PathBuf, process::ExitCode};
#[derive(Parser)]
#[command(name = "integration-demo", about = "Deploy and test the local integration stand-in")]
struct Arguments {
    #[arg(long)]
    binary: Option<PathBuf>,
    #[arg(long)]
    toolkit: Option<PathBuf>,
    #[arg(long)]
    report: Option<PathBuf>,
    #[arg(long, conflicts_with_all = ["acceptance", "external_agent"])]
    serve: bool,
    #[arg(long)]
    acceptance: bool,
    #[arg(long, conflicts_with_all = ["serve", "acceptance"])]
    external_agent: bool,
}
#[tokio::main]
async fn main() -> ExitCode {
    let args = match Arguments::try_parse() {
        Ok(args) => args,
        Err(error) => {
            if error.kind() == clap::error::ErrorKind::DisplayHelp {
                print!("{}", Arguments::command().render_long_help());
                return ExitCode::SUCCESS;
            }
            eprintln!("invalid_command_arguments");
            return ExitCode::from(2);
        },
    };
    if run(args).await.is_err() {
        eprintln!("local_integration_demo_failed");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn failure(code: &str, cleanup: &str) -> serde_json::Value {
    serde_json::json!({"passed":false,"validation":"automated external-client deployment validation","failure":{"code":code},"cleanup":cleanup})
}
fn write_report(path: &std::path::Path, result: &serde_json::Value) -> Result<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(result)?)?;
    Ok(())
}
async fn run(args: Arguments) -> Result<()> {
    let report = args.report.clone().or_else(|| {
        args.external_agent
            .then(|| PathBuf::from("target/external-agent-validation.json"))
    });
    if args.external_agent {
        // Invalidate any previous success before startup or subprocess work. An
        // interrupted validation must never leave an earlier run looking current.
        write_report(report.as_ref().unwrap(), &failure("validation_incomplete", "not_started"))?;
    }
    let result = match run_deployment(args).await {
        Ok(result) => result,
        Err(_) => failure("deployment_start_failed", "startup_cleanup_not_confirmed"),
    };
    // A passing persisted report includes cleanup, not merely successful probes.
    if let Some(path) = report {
        write_report(&path, &result)?;
    }
    println!("{}", serde_json::to_string_pretty(&result)?);
    if result["passed"] != true {
        return Err("local_validation_failed".into());
    }
    Ok(())
}
async fn run_deployment(args: Arguments) -> Result<serde_json::Value> {
    let directory = std::env::current_exe()?.parent().unwrap().parent().unwrap().to_path_buf();
    let binary = args.binary.unwrap_or_else(|| directory.join("ipe-integration"));
    let toolkit = args.toolkit.unwrap_or_else(|| directory.join("integration-toolkit"));
    let Arguments { serve, acceptance, external_agent, .. } = args;
    let mut deployment = support::Deployment::start(&binary, &toolkit).await?;
    let operation: Result<serde_json::Value> = async {
        let result = if external_agent {
            deployment.external_agent().await?
        } else if acceptance {
            deployment.acceptance().await?
        } else {
            deployment.smoke().await?;
            serde_json::json!({"passed":true,"checks":["native workload","TypeScript HTTPS facade","hosted WASM discovery","Rust CLI","Rust MCP"]})
        };
        if serve {
            deployment.serve().await?;
        }
        Ok(result)
    }.await;
    let cleanup = deployment.close().await;
    let mut result =
        operation.unwrap_or_else(|_| failure("validation_operation_failed", "pending"));
    if cleanup.is_err() {
        result["passed"] = serde_json::json!(false);
        result["cleanup"] = serde_json::json!("deployment_cleanup_failed");
    } else {
        result["cleanup"] = serde_json::json!("completed");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn failed_external_startup_overwrites_previous_success_report() {
        let directory = tempfile::tempdir().unwrap();
        let report = directory.path().join("report.json");
        std::fs::write(&report, r#"{"passed":true,"operations":["stale success"]}"#).unwrap();
        let result = run(Arguments {
            binary: Some(directory.path().join("missing-workload")),
            toolkit: Some(directory.path().join("missing-toolkit")),
            report: Some(report.clone()),
            serve: false,
            acceptance: false,
            external_agent: true,
        })
        .await;
        assert!(result.is_err());
        let result: serde_json::Value =
            serde_json::from_slice(&std::fs::read(report).unwrap()).unwrap();
        assert_eq!(result["passed"], false);
        assert_eq!(result["failure"]["code"], "deployment_start_failed");
        assert!(result.get("operations").is_none());
    }
    #[test]
    fn deployment_modes_are_mutually_exclusive() {
        for modes in [
            ["--acceptance", "--serve"],
            ["--external-agent", "--serve"],
            ["--external-agent", "--acceptance"],
        ] {
            let error = Arguments::try_parse_from(["integration-demo", modes[0], modes[1]])
                .err()
                .unwrap();
            assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
        }
        assert!(
            Arguments::try_parse_from(["integration-demo", "--external-agent"])
                .unwrap()
                .external_agent
        );
    }
}
