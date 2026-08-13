//! Pluggable verify system — auto-verify after tool execution.
//!
//! Built-in verifiers: cargo, npm, pip, go test.
//! Custom verifiers implement the `Verify` trait.

use async_trait::async_trait;
use std::process::Stdio;
use tokio::process::Command;

/// Result of a verify run.
#[derive(Debug, Clone)]
pub struct VerifyResult {
    pub passed: bool,
    pub output: String,
    pub duration_ms: u64,
}

/// A verify strategy. Implement this to add a custom verifier.
#[async_trait]
pub trait Verify: Send + Sync {
    /// Name of the verifier (e.g. "cargo", "npm").
    fn name(&self) -> &str;

    /// Whether this verifier applies to the current project.
    /// Called before running — return false to skip.
    fn applies(&self, project_root: &std::path::Path) -> bool;

    /// Run the verification.
    async fn run(&self, project_root: &std::path::Path) -> VerifyResult;
}

/// Cargo verify — runs `cargo build` and optionally `cargo test`.
pub struct CargoVerify {
    pub run_tests: bool,
}

impl Default for CargoVerify {
    fn default() -> Self {
        Self { run_tests: true }
    }
}

#[async_trait]
impl Verify for CargoVerify {
    fn name(&self) -> &str {
        "cargo"
    }

    fn applies(&self, root: &std::path::Path) -> bool {
        root.join("Cargo.toml").exists()
    }

    async fn run(&self, root: &std::path::Path) -> VerifyResult {
        let start = std::time::Instant::now();

        // cargo build --release
        let build = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        let build_output = match build {
            Ok(out) => String::from_utf8_lossy(&out.stdout).to_string()
                + &String::from_utf8_lossy(&out.stderr),
            Err(e) => {
                return VerifyResult {
                    passed: false,
                    output: format!("cargo build failed to start: {}", e),
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            }
        };

        if !build_output.contains("Finished") && !build_output.contains("Compiling") {
            return VerifyResult {
                passed: false,
                output: build_output,
                duration_ms: start.elapsed().as_millis() as u64,
            };
        }

        // optionally cargo test
        if self.run_tests {
            let test = Command::new("cargo")
                .args(["test", "--release"])
                .current_dir(root)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await;

            return match test {
                Ok(out) => VerifyResult {
                    passed: out.status.success(),
                    output: String::from_utf8_lossy(&out.stdout).to_string()
                        + &String::from_utf8_lossy(&out.stderr),
                    duration_ms: start.elapsed().as_millis() as u64,
                },
                Err(e) => VerifyResult {
                    passed: false,
                    output: format!("cargo test failed to start: {}", e),
                    duration_ms: start.elapsed().as_millis() as u64,
                },
            };
        }

        VerifyResult {
            passed: true,
            output: build_output,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }
}

/// npm verify — runs `npm test`.
pub struct NpmVerify;

#[async_trait]
impl Verify for NpmVerify {
    fn name(&self) -> &str {
        "npm"
    }

    fn applies(&self, root: &std::path::Path) -> bool {
        root.join("package.json").exists()
    }

    async fn run(&self, root: &std::path::Path) -> VerifyResult {
        let start = std::time::Instant::now();
        let out = Command::new("npm")
            .args(["test"])
            .current_dir(root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        match out {
            Ok(o) => VerifyResult {
                passed: o.status.success(),
                output: String::from_utf8_lossy(&o.stdout).to_string()
                    + &String::from_utf8_lossy(&o.stderr),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => VerifyResult {
                passed: false,
                output: format!("npm test failed to start: {}", e),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }
}

/// pip verify — runs `pytest`.
pub struct PipVerify;

#[async_trait]
impl Verify for PipVerify {
    fn name(&self) -> &str {
        "pip"
    }

    fn applies(&self, root: &std::path::Path) -> bool {
        // requirements.txt or setup.py or pyproject.toml
        root.join("requirements.txt").exists()
            || root.join("setup.py").exists()
            || root.join("pyproject.toml").exists()
    }

    async fn run(&self, root: &std::path::Path) -> VerifyResult {
        let start = std::time::Instant::now();
        let out = Command::new("pytest")
            .current_dir(root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        match out {
            Ok(o) => VerifyResult {
                passed: o.status.success(),
                output: String::from_utf8_lossy(&o.stdout).to_string()
                    + &String::from_utf8_lossy(&o.stderr),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => VerifyResult {
                passed: false,
                output: format!("pytest failed to start: {}", e),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }
}

/// go verify — runs `go test ./...`.
pub struct GoVerify;

#[async_trait]
impl Verify for GoVerify {
    fn name(&self) -> &str {
        "go"
    }

    fn applies(&self, root: &std::path::Path) -> bool {
        root.join("go.mod").exists()
    }

    async fn run(&self, root: &std::path::Path) -> VerifyResult {
        let start = std::time::Instant::now();
        let out = Command::new("go")
            .args(["test", "./..."])
            .current_dir(root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        match out {
            Ok(o) => VerifyResult {
                passed: o.status.success(),
                output: String::from_utf8_lossy(&o.stdout).to_string()
                    + &String::from_utf8_lossy(&o.stderr),
                duration_ms: start.elapsed().as_millis() as u64,
            },
            Err(e) => VerifyResult {
                passed: false,
                output: format!("go test failed to start: {}", e),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }
}

/// Chain of verifiers. Runs the first one that `applies()`.
pub struct VerifyChain {
    verifiers: Vec<Box<dyn Verify>>,
}

impl VerifyChain {
    /// Create an empty chain.
    pub fn new() -> Self {
        Self { verifiers: Vec::new() }
    }

    /// Add a verifier to the chain.
    pub fn push(&mut self, v: Box<dyn Verify>) {
        self.verifiers.push(v);
    }

    /// Build a default chain with all 4 built-in verifiers.
    pub fn default_chain() -> Self {
        let mut chain = Self::new();
        chain.push(Box::new(CargoVerify::default()));
        chain.push(Box::new(NpmVerify));
        chain.push(Box::new(PipVerify));
        chain.push(Box::new(GoVerify));
        chain
    }

    /// Run the first applicable verifier. Returns None if no verifier applies.
    pub async fn run(&self, root: &std::path::Path) -> Option<VerifyResult> {
        for v in &self.verifiers {
            if v.applies(root) {
                return Some(v.run(root).await);
            }
        }
        None
    }
}

impl Default for VerifyChain {
    fn default() -> Self {
        Self::default_chain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn cargo_applies_when_cargo_toml_exists() {
        let v = CargoVerify::default();
        // this crate's own Cargo.toml is at ../../Cargo.toml from the test dir
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(v.applies(&root));
    }

    #[test]
    fn default_chain_has_all_verifiers() {
        let chain = VerifyChain::default_chain();
        assert_eq!(chain.verifiers.len(), 4);
    }
}
