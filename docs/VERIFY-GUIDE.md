# Verify Guide

## Pluggable verify per project type

| Project type | Auto-detected by | Verify command |
|---|---|---|
| Rust | `Cargo.toml` | `cargo build --release` + `cargo test` |
| Node | `package.json` | `npm test` |
| Python | `requirements.txt` / `setup.py` / `pyproject.toml` | `pytest` |
| Go | `go.mod` | `go test ./...` |

## Custom verifier

```rust
#[async_trait]
impl Verify for MyVerify {
    fn name(&self) -> &str { "my-custom" }
    fn applies(&self, root: &Path) -> bool { root.join("Makefile").exists() }
    async fn run(&self, root: &Path) -> VerifyResult { /* ... */ }
}
```

Drop in `~/.rusty-whale/verify/` — auto-loaded next run.
