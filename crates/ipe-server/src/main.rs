mod runtime;
mod strict_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    runtime::run().await
}
