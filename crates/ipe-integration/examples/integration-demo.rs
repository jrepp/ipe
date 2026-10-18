mod support;
use std::path::PathBuf;
#[tokio::main]
async fn main() {
    if run().await.is_err() {
        eprintln!("local_integration_demo_failed");
        std::process::exit(1);
    }
}
async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = std::env::args().skip(1);
    let directory = std::env::current_exe()?.parent().unwrap().parent().unwrap().to_path_buf();
    let mut binary = directory.join("ipe-integration");
    let mut toolkit = directory.join("integration-toolkit");
    let mut serve = false;
    let mut acceptance = false;
    let mut report: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
        "--binary" => binary = args.next().ok_or("missing binary")?.into(),
        "--toolkit" => toolkit = args.next().ok_or("missing toolkit")?.into(),
        "--report" => report = Some(args.next().ok_or("missing report")?.into()),
        "--serve" => serve = true,
        "--acceptance" => acceptance = true,
        _ => return Err("usage: integration-demo [--serve] [--acceptance] [--binary PATH] [--toolkit PATH] [--report PATH]".into()),
    }
    }
    let mut deployment = support::Deployment::start(&binary, &toolkit).await?;
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
}
