use clap::{CommandFactory, Parser};
use ipe_integration::github_contract::{capture, CaptureOptions};
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(about = "Capture GitHub workflow and action declarations without executing them")]
struct Arguments {
    #[arg(long)]
    repo: String,
    #[arg(long = "ref", default_value = "main")]
    revision: String,
    #[arg(long)]
    workflow: String,
    /// Exact additional repositories whose referenced actions can be read.
    #[arg(long)]
    allow_repo: Vec<String>,
    #[arg(long, default_value = "target/github-contract")]
    out: PathBuf,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args = match Arguments::try_parse() {
        Ok(args) => args,
        Err(error) => {
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                let _ = Arguments::command().print_help();
                return ExitCode::SUCCESS;
            }
            eprintln!("invalid_command_arguments");
            return ExitCode::from(2);
        },
    };
    let options = CaptureOptions {
        repository: args.repo,
        revision: args.revision,
        workflow: args.workflow,
        allowed_repositories: args.allow_repo,
    };
    match capture(options).await {
        Ok(report) => {
            if let Err(code) = ipe_integration::github_contract::write_report(&args.out, &report) {
                eprintln!("{code}");
                return ExitCode::FAILURE;
            }
            println!(
                "capture_complete documents={} gaps={} coverage=partial",
                report["documents"].as_array().map_or(0, Vec::len),
                report["gaps"].as_array().map_or(0, Vec::len)
            );
            ExitCode::SUCCESS
        },
        Err(code) => {
            eprintln!("{code}");
            ExitCode::FAILURE
        },
    }
}
