# Integration Test Compile Failure Diagnosis and Repair (`RESULTS.md`)

## Stage 1: Reproduction and Root-Cause Diagnosis

### Reproduction on `origin/main`

Reproduced on pristine `origin/main` (`f532738f0dfcef9814d6fe90e0a2590984f47631`) with `bash ~/.local/state/wikijump/muse-bhl/tools/heavy-build.sh cargo test --tests --no-run`.

The `integration` test target (`tests/integration.rs`) fails to compile with four `E0063` errors across two integration test modules:

```text
error[E0063]: missing field `wikidot_row_cut` in initializer of `ftml::tree::Tab<'_>`
   --> tests/coverage_public_api.rs:312:50
    |
312 |             Element::Partial(PartialElement::Tab(Tab {
    |                                                  ^^^ missing `wikidot_row_cut`

error[E0063]: missing field `wikidot_row_cut` in initializer of `ftml::tree::Tab<'_>`
   --> tests/security_regressions.rs:667:50
    |
667 |             Element::Partial(PartialElement::Tab(Tab {
    |                                                  ^^^ missing `wikidot_row_cut`

error[E0063]: missing field `wikidot_row_cut` in initializer of `ftml::tree::Tab<'_>`
   --> tests/security_regressions.rs:726:50
    |
726 |             Element::Partial(PartialElement::Tab(Tab {
    |                                                  ^^^ missing `wikidot_row_cut`

error[E0063]: missing field `wikidot_row_cut` in initializer of `ftml::tree::Tab<'_>`
   --> tests/security_regressions.rs:767:50
    |
767 |             Element::Partial(PartialElement::Tab(Tab {
    |                                                  ^^^ missing `wikidot_row_cut`

error: could not compile `ftml` (test "integration") due to 4 previous errors
```

### Root Cause

Commit `1e8f83b9a4125f78079138964d26e63919743541` (`fix(wikidot): carry the row-cut block kind to the tabview lowering`, merged via PR #1384 `d36137ca8` `fix(wikidot): collapsible-cut row tails in tabviews (collcut) (#1384)`) deliberately added the public field `pub wikidot_row_cut: Option<WikidotRowCut>` to `ftml::tree::Tab<'t>` in `src/tree/tab.rs`.

That commit updated the `Tab { ... }` struct literal initializers in the library unit tests (`src/render/html/element/tabs.rs` and `src/tree/partial.rs`) with `wikidot_row_cut: None`, matching how an earlier commit (`a41f618d89c7d5f48ccfc379fd0b3eeb576e7e55`, `fix(wikidot): wrap an open span around tab nav anchors`) had added `wikidot_label_spans: Vec::new()`, but omitted the four `Tab { ... }` struct literal initializers in the integration test suite (`tests/coverage_public_api.rs` line 312 and `tests/security_regressions.rs` lines 667, 726, and 767).

## Stage 2: Fix and Targeted Verification

### Changes Made

- `tests/coverage_public_api.rs`: Added `wikidot_row_cut: None` to the `PartialElement::Tab(Tab { ... })` initializer in `public_api_html_render_covers_fallback_branches_from_ast`.
- `tests/security_regressions.rs`: Added `wikidot_row_cut: None` to the three `PartialElement::Tab(Tab { ... })` initializers in `malformed_partial_elements_render_fallback_html` and `malformed_partial_elements_render_fallback_text`.

### Verification

- `bash ~/.local/state/wikijump/muse-bhl/tools/heavy-build.sh cargo test --tests --no-run`: Compiles both `unittests src/lib.rs` and `tests/integration.rs` with 0 errors.
- `bash ~/.local/state/wikijump/muse-bhl/tools/heavy-build.sh cargo test --test integration coverage_public_api`: 7 passed, 0 failed.
- `bash ~/.local/state/wikijump/muse-bhl/tools/heavy-build.sh cargo test --test integration security_regressions`: 59 passed, 0 failed.
- `rustfmt --check tests/coverage_public_api.rs tests/security_regressions.rs`: Clean (0 diffs).

### Unrelated Observations Outside Compile Breakage (Not Modified)

- `tests/super_linear_families.rs:202` emits a `dead_code` warning for `fn bracket_run_repair_probes_stay_bounded()` (missing `#[test]` or unused helper).
- `ftml` (`lib`) emits 17 compiler warnings (`unused_imports`, `unused_assignments`, `unused_mut`, `unused_variables`, `unreachable_patterns`, and `dead_code`) on `origin/main`.
- `cargo fmt --check` reports pre-existing formatting drift in unrelated `src/` and `tests/` files on `origin/main`.
