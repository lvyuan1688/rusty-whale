# Contributing to rusty-whale

Thanks for your interest in rusty-whale! This is a community-driven, open-source
agent harness, and we welcome contributions of all sizes.

## Quick start

```bash
git clone https://github.com/lvyuan1688/rusty-whale
cd rusty-whale
cargo build
cargo test
```

You don't need an API key to run the skeleton — the stub provider returns a
hard-coded response so the CLI and examples work offline.

## Ways to contribute

- **Bugs**: open an issue with a minimal reproduction (OS, Rust version, command).
- **Features**: open an issue first to scope the work, then send a PR.
- **Providers**: add a new `LlmProvider` implementation in
  `crates/rusty-whale-provider/src/lib.rs`. Wire it into the CLI in
  `src/main.rs`.
- **Verifiers**: add a new `Verifier` implementation in
  `crates/rusty-whale-verify/src/lib.rs`. Useful for project-specific verify
  steps (e.g. `make test`, `pytest`, `go test ./...`).
- **Docs**: typos, clarifications, and new guides are all welcome.

## Pull request checklist

- [ ] `cargo fmt` is clean
- [ ] `cargo clippy -- -D warnings` is clean
- [ ] `cargo test` passes
- [ ] `CHANGELOG.md` updated (if user-visible)
- [ ] Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)

## Release cadence

Patch releases (`v0.1.x`) ship bug fixes and small improvements. Minor
releases (`v0.x.0`) ship new providers, verifiers, or breaking changes. See
`docs/RELEASING.md` for the maintainer checklist.

## Code of conduct

Be kind. Disagreements happen — address them constructively. Personal attacks,
harassment, or discriminatory behavior will not be tolerated.

## License

By contributing, you agree your contributions are licensed under the MIT
license (see `LICENSE`).
