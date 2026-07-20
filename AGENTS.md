# FTML

## Read this first

- `README.md`: crate purpose and supported syntax scope.
- `docs/WikijumpBoundary.md`: pointer to the frozen FTML/Wikijump responsibility boundary contract and the FTML-side obligations it implies.
- `Cargo.toml` and workspace manifests: crate/test structure.
- Existing parser/render tests and fixtures: preferred source of behavior examples.
- Any repo-local docs or examples for block, inline, module, and HTML rendering behavior.

## Product language

FTML provides reusable parsing/rendering primitives for Wikijump and related tools. It should be deterministic, conservative, and test-driven.

FTML should not own site-runtime semantics. Behavior that depends on a Wikijump site, page database, actor, URL arguments, page query, import state, file service, or permissions belongs in Wikijump or the caller's runtime layer.

For Wikidot compatibility, real Wikidot evidence and provenance-backed corpus examples outrank assumptions. Local Wikijump output is not a source-of-truth oracle for FTML behavior.

## Architecture boundaries

- Preserve syntax structure when runtime data is required. Do not erase module syntax merely because FTML cannot evaluate it.
- `ListPages` and `CountPages` should remain delayed/preserved structures unless the caller supplies explicit runtime evaluation context. Runtime query semantics belong in Wikijump.
- Avoid broad compatibility shims that make unsupported syntax look supported. Fail closed, preserve literal structure, or expose an explicit representation.
- Treat raw HTML, escaping, and sanitization boundaries as security-sensitive. Do not weaken escaping to match a single fixture without evidence and tests.

## Implementation rules

Prefer small parser/rendering slices with focused fixtures. Search existing tests before adding new helpers. Do not add one-off helpers that obscure the grammar or are used only once unless they make a tricky invariant local and testable.

When changing parser or renderer behavior, add regression tests that describe the syntax and expected output. Include negative tests for unsupported or preserved behavior when that is the safety property.

Keep public APIs intentional. If a change affects consumers such as Wikijump, record the compatibility implication in the PR body or final report.

Size PRs by reviewability and risk, not by a line quota; do not split a coherent change merely to shrink the diff.

Give every worktree an owning task and remove it once its branch merges or its task closes; long-lived worktrees need a recorded owner. Create new worktrees under `~/wjlab/worktrees/ftml/<task-slug>` rather than scattering them across other locations.

Avoid large modules: target Rust modules under roughly 500 LoC excluding tests; past roughly 800 LoC, put new functionality in a new module and move the related tests and module docs with it.

## Validation expectations

Choose focused commands for the touched surface, then broaden before PR/merge when behavior is general:

```bash
cargo fmt --check
cargo test <focused-test-or-module>
cargo test
RUSTFLAGS='-D warnings' cargo clippy --tests --no-deps
```
