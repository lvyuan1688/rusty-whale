//! Custom verify example: build a Verifier that always succeeds, run the
//! verify loop, and print the result. Run with
//! `cargo run --example custom-verify`.

use anyhow::Result;
use async_trait::async_trait;
use rusty_whale_verify::{FailAction, VerifyConfig, Verifier};

struct AlwaysTrue;

#[async_trait]
impl Verifier for AlwaysTrue {
    async fn verify(&self, _cfg: &VerifyConfig) -> Result<bool> {
        Ok(true)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = VerifyConfig {
        command: "echo verified".into(),
        fail_action: FailAction::Retry,
        max_retries: 1,
    };
    let v = AlwaysTrue;
    let ok = v.verify(&cfg).await?;
    println!("verify passed = {ok}");
    Ok(())
}
