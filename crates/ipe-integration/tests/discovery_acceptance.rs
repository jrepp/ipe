#[allow(dead_code)]
#[path = "../examples/support/mod.rs"]
mod support;

/// Requires Node >=22.18 and OpenSSL, and runs real Rust binaries against the TS HTTPS facade.
#[tokio::test]
#[ignore = "requires Node >=22.18 and OpenSSL; exercised by integration CI"]
async fn standalone_discovery_acceptance() {
    let mut deployment = support::Deployment::start(
        std::path::Path::new(env!("CARGO_BIN_EXE_ipe-integration")),
        std::path::Path::new(env!("CARGO_BIN_EXE_integration-toolkit")),
    )
    .await
    .expect("local deployment starts");
    let report = deployment.acceptance().await.expect("black-box acceptance passes");
    println!("{report}");
}
