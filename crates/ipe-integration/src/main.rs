use clap::{CommandFactory, Parser};
use std::{path::PathBuf, process::ExitCode};
#[derive(Parser)]
#[command(name = "ipe-integration", about = "Run the native Integration Workload")]
struct Arguments {
    /// Operator-owned connection and runtime configuration.
    config_path: PathBuf,
    /// Host agent-authored installations with scheduling and durable run history.
    #[arg(long)]
    host: bool,
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
    if let Err(code) = run(args).await {
        eprintln!("{code}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
async fn run(args: Arguments) -> Result<(), &'static str> {
    if std::fs::metadata(&args.config_path)
        .map_err(|_| "configuration_unavailable")?
        .len()
        > 1_048_576
    {
        return Err("configuration_limit");
    }
    let bytes = std::fs::read(args.config_path).map_err(|_| "configuration_unavailable")?;
    if args.host {
        let config = ipe_json::parse(&bytes).map_err(|_| "invalid_configuration")?;
        return ipe_integration::host::serve(config).await;
    }
    let config = serde_json::from_slice(&bytes).map_err(|_| "invalid_configuration")?;
    ipe_integration::serve(config).await
}
