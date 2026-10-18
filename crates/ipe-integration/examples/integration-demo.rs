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
    #[arg(long, conflicts_with = "acceptance")]
    serve: bool,
    #[arg(long)]
    acceptance: bool,
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
async fn run(args: Arguments) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let directory = std::env::current_exe()?.parent().unwrap().parent().unwrap().to_path_buf();
    let binary = args.binary.unwrap_or_else(|| directory.join("ipe-integration"));
    let toolkit = args.toolkit.unwrap_or_else(|| directory.join("integration-toolkit"));
    let Arguments { report, serve, acceptance, .. } = args;
    let mut deployment = support::Deployment::start(&binary, &toolkit).await?;
    let operation: Result<(), Box<dyn std::error::Error + Send + Sync>> = async {
    let result = if acceptance {
        deployment.acceptance().await?
    } else {
        deployment.smoke().await?;
        serde_json::json!({"passed":true,"checks":["native workload","TypeScript HTTPS facade","hosted WASM discovery","Rust CLI","Rust MCP"]})
    };
    let output = serde_json::to_string_pretty(&result)?;
    if let Some(path) = report {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &output)?;
    }
    println!("{output}");
    if serve {
        deployment.serve().await?;
    }
    Ok(())
    }.await;
    let cleanup = deployment.close().await;
    operation?;
    cleanup?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn acceptance_and_interactive_serve_are_mutually_exclusive() {
        let error = Arguments::try_parse_from(["integration-demo", "--acceptance", "--serve"])
            .err()
            .unwrap();
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
}
