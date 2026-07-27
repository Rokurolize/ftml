# FTML

## Product and documentation

- `README.md`, `docs/WikijumpBoundary.md`, the workspace manifests, and nearby parser/render fixtures.
- FTML provides deterministic parsing and rendering primitives. Behavior requiring a site, page database, actor, URL arguments, query, import state, file service, permissions, or browser runtime belongs in Wikijump or another caller.
- For Wikidot compatibility, live Wikidot evidence and provenance-backed corpus examples outrank assumptions and local Wikijump output.

## Development

- Search existing tests and helpers before adding parser or renderer behavior. Add focused positive and negative regression fixtures for changed syntax.
- Prefer simple local invariants over one-off abstraction. Split large modules when their responsibilities no longer fit clearly in one place.
- Remove task-owned branches, worktrees, target directories, and build artifacts when they are no longer useful.

## Validation and delivery

- Choose validation in proportion to the changed surface. Typical commands are `cargo fmt --check`, focused `cargo test`, the full suite for general parser or renderer changes, and `RUSTFLAGS='-D warnings' cargo clippy --tests --no-deps`.
