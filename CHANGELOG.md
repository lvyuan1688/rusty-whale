# Changelog

All notable changes to rusty-whale are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.4] — 2026-08-20

### Added
- `src/main.rs` CLI entry point with `run` and `config` subcommands.
- `examples/basic-agent.rs` minimal agent using an echo provider.
- `examples/custom-verify.rs` showing how to plug in a custom `Verifier`.
- `CONTRIBUTING.md` with quick-start, PR checklist, and release cadence.
- `.github/ISSUE_TEMPLATE/{bug_report,feature_request}.md`.
- `.github/PULL_REQUEST_TEMPLATE.md`.
- `docs/demo.png` placeholder screenshot.

## [0.1.3] — 2026-08-15

### Added
- `docs/v0.1.3-patch-notes.md` describing the provider trait extension.

## [0.1.2] — 2026-08-13

### Added
- Stub `OllamaProvider` returning a skeleton response for offline examples.

## [0.1.1] — 2026-08-12

### Added
- Workspace split: `rusty-whale-core`, `-provider`, `-verify`, `-tui`.

## [0.1.0] — 2026-08-10

Initial public skeleton.
