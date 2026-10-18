//! CLI argument handling must not disclose user-supplied values or contaminate MCP stdout.
use std::process::{Command, Output};
fn toolkit(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_integration-toolkit"))
        .args(args)
        .env_clear()
        .env("IPE_INTEGRATION_URL", "https://secret.internal")
        .output()
        .unwrap()
}
#[test]
fn toolkit_help_and_errors_never_echo_inputs() {
    for args in [vec!["--help"], vec!["mcp", "--help"]] {
        let output = toolkit(&args);
        assert!(output.status.success());
        let all = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(all.contains("integration-toolkit"));
        assert!(!all.contains("secret.internal"));
        if args[0] == "mcp" {
            assert!(output.stdout.is_empty());
        }
    }
    for args in [
        vec!["mcp", "--credential=secret.internal"],
        vec!["discover"],
        vec!["secret.internal"],
        vec!["mcp", "extra-secret.internal"],
    ] {
        let output = toolkit(&args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, b"invalid_command_arguments\n");
    }
}
#[test]
fn global_flags_and_subcommands_are_typed() {
    for args in [
        vec!["--token-env", "ABSENT_TEST_TOKEN", "capabilities"],
        vec!["check", "conn_test", "--token-env", "ABSENT_TEST_TOKEN"],
        vec!["discover", "conn_test", "--url", "http://127.0.0.1:1"],
    ] {
        let output = toolkit(&args);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stderr.is_empty());
        let body: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(body["error"]["code"], "missing_or_invalid_workload_credential");
    }
}
#[test]
fn workload_requires_one_config_path_and_redacts_arguments() {
    for args in [
        vec![],
        vec!["config-secret.internal", "extra-secret.internal"],
        vec!["--secret.internal"],
    ] {
        let output =
            Command::new(env!("CARGO_BIN_EXE_ipe-integration")).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, b"invalid_command_arguments\n");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_ipe-integration"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("CONFIG_PATH"));
}

#[test]
fn explicit_url_overrides_environment() {
    for (args, expected) in [
        (vec!["capabilities"], "invalid_workload_endpoint"),
        (
            vec!["--url", "http://127.0.0.1:1", "capabilities"],
            "missing_or_invalid_workload_credential",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_integration-toolkit"))
            .args(args)
            .env_clear()
            .env("IPE_INTEGRATION_URL", "http://192.0.2.1")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
            expected
        );
        assert!(output.stderr.is_empty());
    }
}
