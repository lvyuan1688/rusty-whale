//! rusty-whale-verify: pluggable verify system (cargo/npm/pip/go).

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// What to do when verification fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailAction {
    Retry,
    Abort,
    Ask,
}

/// Verify configuration, typically loaded from `~/.rusty-whale/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyConfig {
    pub command: String,
    pub fail_action: FailAction,
    pub max_retries: u32,
}

impl Default for VerifyConfig {
    fn default() -> Self {
        Self {
            command: "cargo build --release".into(),
            fail_action: FailAction::Retry,
            max_retries: 3,
        }
    }
}

/// The verify trait. The default implementation shells out to the configured
/// command and treats a zero exit status as success.
#[async_trait]
pub trait Verifier: Send + Sync {
    async fn verify(&self, cfg: &VerifyConfig) -> Result<bool>;
}

/// A shell-based verifier that runs the configured command.
pub struct ShellVerifier;

#[async_trait]
impl Verifier for ShellVerifier {
    async fn verify(&self, cfg: &VerifyConfig) -> Result<bool> {
        let parts: Vec<&str> = cfg.command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(false);
        }
        let mut cmd = Command::new(parts[0]);
        cmd.args(&parts[1..]);
        let status = tokio::task::spawn_blocking(move || cmd.status()).await??;
        Ok(status.success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_cargo() {
        let c = VerifyConfig::default();
        assert_eq!(c.command, "cargo build --release");
        assert_eq!(c.fail_action, FailAction::Retry);
        assert_eq!(c.max_retries, 3);
    }
}
