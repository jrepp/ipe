#[tokio::main]
async fn main() {
    if let Err(code) = run().await {
        eprintln!("{code}");
        std::process::exit(1);
    }
}
async fn run() -> Result<(), &'static str> {
    let path = std::env::args().nth(1).ok_or("usage: ipe-integration CONFIG_PATH")?;
    if std::fs::metadata(&path).map_err(|_| "configuration_unavailable")?.len() > 1_048_576 {
        return Err("configuration_limit");
    }
    let bytes = std::fs::read(path).map_err(|_| "configuration_unavailable")?;
    let config = serde_json::from_slice(&bytes).map_err(|_| "invalid_configuration")?;
    ipe_integration::serve(config).await
}
