# FTML

## Product and documentation

- `README.md`, `docs/WikijumpBoundary.md`, the workspace manifests, and nearby parser/render fixtures.
- FTML provides deterministic parsing and rendering primitives. Behavior requiring a site, page database, actor, URL arguments, query, import state, file service, permissions, or browser runtime belongs in Wikijump or another caller.
- For Wikidot compatibility, live Wikidot evidence and provenance-backed corpus examples outrank assumptions and local Wikijump output.

## Development

- Search existing tests and helpers before adding parser or renderer behavior. Add focused positive and negative regression fixtures for changed syntax.
- Prefer simple local invariants over one-off abstraction. Split large modules when their responsibilities no longer fit clearly in one place.
- Remove task-owned branches, worktrees, target directories, and build artifacts when they are no longer useful.
- Cargo targets: normal development uses the canonical `target/` and compact profiles in `Cargo.toml`; candidate and one-shot CI builds use revision/role-specific `CARGO_TARGET_DIR` with `CARGO_INCREMENTAL=0`. Read `docs/development/cargo-target-policy.md` before changing build or cleanup behavior.

## Validation and delivery

- Choose validation in proportion to the changed surface. Typical commands are `cargo fmt --check`, focused `cargo test`, the full suite for general parser or renderer changes, and `RUSTFLAGS='-D warnings' cargo clippy --tests --no-deps`.

## Agent skills

### Issue tracker

Issues live in GitHub Issues and use the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Use the default five triage labels. See `docs/agents/triage-labels.md`.

### Domain docs

This is a single-context repo. See `docs/agents/domain.md`.

### Agent build approval

Every non-controller agent and background process must request controller approval before memory-intensive Rust commands. The controller serializes those commands through one build slot. See `docs/agents/herdr-build-approval.md`.
