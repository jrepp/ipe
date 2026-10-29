use std::process::Command;

/// Uses real public GitHub files at pinned commits. No actions are executed.
#[test]
#[ignore = "requires GitHub network access; optional IPE_GITHUB_TOKEN avoids anonymous rate limits"]
fn public_workflow_and_nested_actions_produce_reviewable_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_github-contract"));
    command.args([
        "--repo",
        "jrepp/ipe",
        "--ref",
        "8346e53d0b428b701349817df6624f0ad33a1639",
        "--workflow",
        ".github/workflows/docs-site.yml",
    ]);
    for repo in [
        "actions/checkout",
        "actions/setup-python",
        "actions/setup-node",
        "actions/upload-artifact",
        "actions/upload-pages-artifact",
        "actions/configure-pages",
        "actions/deploy-pages",
    ] {
        command.args(["--allow-repo", repo]);
    }
    let output = command.arg("--out").arg(directory.path()).output().unwrap();
    assert!(
        output.status.success(),
        "capture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = std::fs::read(directory.path().join("capture.json")).unwrap();
    if let Ok(token) = std::env::var("IPE_GITHUB_TOKEN") {
        if !token.is_empty() {
            assert!(
                !String::from_utf8_lossy(&bytes).contains(&token),
                "credential leaked into report"
            );
            assert!(
                !String::from_utf8_lossy(&output.stdout).contains(&token),
                "credential leaked into stdout"
            );
            assert!(
                !String::from_utf8_lossy(&output.stderr).contains(&token),
                "credential leaked into stderr"
            );
        }
    }
    let report: serde_json::Value = ipe_json::parse(&bytes).unwrap();
    assert_eq!(report["schema"], "ipe.github-contract.v1");
    assert_eq!(report["publication"], false);
    assert_eq!(report["coverage"], "partial");
    assert_eq!(
        report["gaps"],
        serde_json::json!([]),
        "dependency reads must succeed for live acceptance"
    );
    let docs = report["documents"].as_array().unwrap();
    assert_eq!(docs[0]["source"]["commit"], "8346e53d0b428b701349817df6624f0ad33a1639");
    assert_eq!(docs[0]["projection"]["model"]["jobs"].as_array().unwrap().len(), 2);
    let packaging = docs
        .iter()
        .find(|d| d["source"]["repository"] == "actions/upload-pages-artifact")
        .unwrap();
    assert_eq!(packaging["projection"]["model"]["runs"]["using"], "composite");
    assert!(report["edges"].as_array().unwrap().iter().any(|e| e["from"] == packaging["id"]));
    let checkout = docs.iter().find(|d| d["source"]["repository"] == "actions/checkout").unwrap();
    assert!(checkout["projection"]["model"]["runs"].get("post").is_some());
    assert!(checkout["implementation"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["role"] == "author_source_clue" && e["status"] == "captured"));
    assert!(report["edges"].as_array().unwrap().iter().any(|e| e["bindings"]["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b["origin"] == "declared_default")));
    for document in docs {
        assert_eq!(document["source"]["commit"].as_str().unwrap().len(), 40);
        assert!(document["source"]["digest"].as_str().unwrap().starts_with("sha256:"));
        assert!(document["locations"].is_object());
    }
    assert!(directory.path().join("report.html").exists());
    println!(
        "live GitHub proof: {} definitions, {} dependency edges, {} requests",
        docs.len(),
        report["edges"].as_array().unwrap().len(),
        report["usage"]["requests"]
    );
}
