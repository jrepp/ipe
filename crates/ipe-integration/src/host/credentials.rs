//! Host-owned credential lookup. Never expose helper output or diagnostics.
use reqwest::Url;
use secrecy::SecretString;
use std::{process::Stdio, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};

type Result<T> = std::result::Result<T, &'static str>;
const OUTPUT_LIMIT: u64 = 16 * 1024;

fn validate_url(value: &str) -> Result<()> {
    let url = Url::parse(value).map_err(|_| "invalid_git_credential_url")?;
    if value.len() > 2048
        || value.chars().any(char::is_control)
        || url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid_git_credential_url");
    }
    Ok(())
}

fn parse(bytes: &[u8]) -> Result<SecretString> {
    let text = std::str::from_utf8(bytes).map_err(|_| "git_credential_invalid")?;
    let mut password = None;
    for line in text.lines().take_while(|line| !line.is_empty()) {
        let (key, value) = line.split_once('=').ok_or("git_credential_invalid")?;
        if key == "password" {
            if password.is_some()
                || value.is_empty()
                || value.len() > 8192
                || value.bytes().any(|b| b.is_ascii_control())
                || reqwest::header::HeaderValue::from_str(value).is_err()
            {
                return Err("git_credential_invalid");
            }
            password = Some(SecretString::from(value));
        }
    }
    password.ok_or("git_credential_missing")
}

pub(super) async fn git(url: &str) -> Result<SecretString> {
    let mut command = Command::new("git");
    // Git chooses configured helpers, including repository-local configuration.
    // The integration package cannot supply executable names or helper options.
    command.args(["-c", "credential.interactive=false", "credential", "fill"]);
    lookup(command, url, Duration::from_secs(5)).await
}

async fn lookup(mut command: Command, url: &str, deadline: Duration) -> Result<SecretString> {
    validate_url(url)?;
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "false")
        .env("SSH_ASKPASS", "false")
        .env("GCM_INTERACTIVE", "never")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().map_err(|_| "git_credential_unavailable")?;
    let result = tokio::time::timeout(deadline, async {
        let mut input = child.stdin.take().ok_or("git_credential_failed")?;
        input
            .write_all(format!("url={url}\n\n").as_bytes())
            .await
            .map_err(|_| "git_credential_failed")?;
        drop(input);
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or("git_credential_failed")?
            .take(OUTPUT_LIMIT + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| "git_credential_failed")?;
        if bytes.len() as u64 > OUTPUT_LIMIT {
            return Err("git_credential_output_limit");
        }
        if !child.wait().await.map_err(|_| "git_credential_failed")?.success() {
            return Err("git_credential_failed");
        }
        parse(&bytes)
    })
    .await
    .unwrap_or(Err("git_credential_timeout"));
    if result.is_err() {
        let _ = child.kill().await;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[test]
    fn protocol_rejects_injection_and_ambiguous_secrets() {
        for url in [
            "http://github.com",
            "https://user:secret@github.com",
            "https://github.com\npassword=secret",
            "https://github.com?secret=x",
        ] {
            assert!(validate_url(url).is_err());
        }
        assert!(validate_url("https://github.com/owner/repo").is_ok());
        assert!(parse(b"username=user\n").is_err());
        assert!(parse(b"password=a\npassword=b\n").is_err());
        assert!(parse(b"password=\n").is_err());
        assert_eq!(
            parse(b"username=user\npassword=test_token\n").unwrap().expose_secret(),
            "test_token"
        );
    }

    #[tokio::test]
    async fn configured_git_helper_receives_context_without_approving_or_storing() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("gitconfig");
        // Fake helper returns only synthetic credentials and checks the get action.
        std::fs::write(&config, "[credential]\n helper =\n helper = \"!f() { test \\\"$1\\\" = get || exit 1; grep -qx 'host=github.com' || exit 1; printf 'username=test\\\\npassword=synthetic_token\\\\n'; }; f\"\n").unwrap();
        let mut command = Command::new("git");
        command
            .current_dir(dir.path())
            .env("GIT_CONFIG_GLOBAL", &config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .args(["credential", "fill"]);
        let token = lookup(command, "https://github.com", Duration::from_secs(5)).await.unwrap();
        assert_eq!(token.expose_secret(), "synthetic_token");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn helper_errors_output_and_execution_are_bounded() {
        for (script, expected) in [
            ("echo sensitive >&2; exit 1", "git_credential_failed"),
            ("exec sleep 5", "git_credential_timeout"),
            ("printf 'password='; head -c 20000 /dev/zero", "git_credential_output_limit"),
        ] {
            let mut command = Command::new("sh");
            command.args(["-c", script]);
            assert_eq!(
                lookup(command, "https://github.com", Duration::from_millis(100)).await.err(),
                Some(expected)
            );
        }
    }
}
