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

Avoid large modules:

- Prefer adding new modules instead of growing existing ones.
- Target Rust modules under 500 LoC, excluding tests.
- If a file exceeds roughly 800 LoC, add new functionality in a new module instead of extending the existing file unless there is a strong documented reason not to.
- When extracting code from a large module, move the related tests and module/type docs toward the new implementation so the invariants stay close to the code that owns them.

## Validation expectations

Choose focused commands for the touched surface, then broaden before PR/merge when behavior is general:

```bash
cargo fmt --check
cargo test <focused-test-or-module>
cargo test
RUSTFLAGS='-D warnings' cargo clippy --tests --no-deps
```
