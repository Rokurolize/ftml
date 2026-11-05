# Public API panic fuzzing report

## STATUS

**Stopped early at the user's request. No fuzzing campaign has run yet, so
no panics have been found, fixed, or covered by regression tests.**

| Item | State |
| --- | --- |
| Map the public parse/render entry points | Done |
| Review the existing panic guards (below) | Done (code reading only, not tested) |
| Stable-Rust fuzz driver (`fuzz/stable_driver/`) | **Partial**: written but **never compiled or run** |
| Fuzzing campaign | **Not started** |
| Minimal repros, root causes, fixes | **Not started** (no panics found yet) |
| Regression tests for fixed panics | **Not started** |
| Corpus run to confirm valid-input output is byte-identical | Not needed yet (no library code changed) |

This branch changes no library code. It adds this report and the uncompiled
driver. `cargo build --tests --examples` passed on the base revision
(`c1dc640`) before any changes.

## Scope

These public entry points were chosen as fuzz targets:

- `ftml::preprocess`, `ftml::preprocess_for_layout`
- `ftml::resolve_wikidot_parser_functions(_with_options)`, with both
  `WikidotZeroOperatorPolicy` values
- `ftml::include` with `DebugIncluder::{all_present, first_missing}` and
  `NullIncluder`
- `ftml::tokenize`, `ftml::parse` (every `WikitextMode`, both layouts,
  settings flags varied)
- `SyntaxTree::{page_references, user_references, to_owned}`, plus a serde
  JSON round trip
- `HtmlRender` (`render` and `render_with_page_existence`), `TextRender`,
  `DebugRender`
- The delayed List pipeline: `DelayedInput::new`, `parse_delayed_list`,
  `bind`, `render_html(_with_page_existence)`
- `Utf16IndexMap`, plus `ExtractedToken::to_utf16_indices` and
  `ParseError::to_utf16_indices` on every token and parse-error span

## Findings so far (code reading only, no testing)

No panics were found, because no fuzzing has run. These notes on existing
guards come from reading the code. They show where to aim the fuzzing.

1. **Deep nesting.** `ftml::parse` runs on a dedicated 64 MiB worker thread
   (`src/parsing/mod.rs`, `DEEP_PARSE_STACK_BYTES`). It retries with
   `DEEP_MAX_RECURSION_DEPTH` (1024) when the default limit (100) trips.
   Rendering and tree traversal move to a bounded stack when
   `tree_requires_bounded_tree_stack` says so (`src/tree/traversal.rs`,
   `src/render/html/mod.rs`). Parser functions are capped at 1,024 nesting
   levels and 65,536 candidates. A stack overflow aborts the process instead
   of panicking, so the driver's supervisor records worker deaths separately
   from caught panics.
2. **UTF-8 boundary slicing at the API surface.**
   - `DelayedInput::new` checks `is_char_boundary` on every segment edge
     before tokenizing (`src/delayed.rs`).
   - The wasm `Utf16IndexMap.get_index` uses `get_index_opt` and returns an
     error for an index inside a character (`src/wasm/utf16.rs`).
   - The host `Utf16IndexMap::get_index` is documented to panic on an index
     inside a character. That is a stated contract, and `get_index_opt` is
     the non-panicking variant.
   - **Unverified risk:** `ExtractedToken::to_utf16_indices` and
     `ParseError::to_utf16_indices` call the panicking `get_index`. They are
     panic-free only if every token and parse-error span lies on a character
     boundary. The wasm `tokenize`/`parse` bindings depend on this. The driver
     checks it on every run.
3. **Case-mapping and normalization length changes.** Some characters change
   byte length when their case changes or when they are normalized (`İ`, `Ⱥ`,
   `Ⱦ`, `ẞ`, `ﬃ`, the Kelvin sign, `ı`, `①`, `㌀`). They are a likely source of
   slice-index panics wherever code finds an index in a lowercased or
   normalized copy and then slices the original. The driver's `SHIFTY` set
   aims at these. No call site has been confirmed yet.

## The stable fuzz driver (`fuzz/stable_driver/`)

The existing `fuzz/` targets need `cargo-fuzz` and nightly. This environment
has only the pinned stable toolchain, so the driver is a separate,
self-contained crate. It declares its own `[workspace]` and uses a release
profile with `debug-assertions` and `overflow-checks` on, so `debug_assert!`
and arithmetic overflow panics are caught too. **It has never been compiled.**
The interrupted build had not finished, and expect compile errors on the first
build.

- `supervise <out> <workers> <secs> [seed]` runs workers under
  `ulimit -v 6000000`. It restarts any worker that dies, which covers stack
  overflow, OOM and other aborts, and saves that worker's last input as
  `abort-*.txt`. It kills a worker that makes no progress for 60 s and saves
  the input as `hang-*.txt`.
- `work` loads the corpus: every `*.ftml` under `test/`, `tests/fixtures/`,
  `verification/`, `misc/`, `benches/` and `conf/`. It mixes three kinds of
  input:
  - corpus mutations: char-safe insert, delete, replace, duplicate, splice
    and truncate; byte-level flips re-decoded lossily; CR/LF rewrites;
    long-token runs; block wrapping; case flips
  - synthesized adversarial shapes, covering every block name and alias,
    unterminated constructs, nested blocks, quotes, lists, tabviews,
    collapsibles and tables, lone-surrogate and out-of-range escapes
    (`&#xD800;`, `\uD800`, `[[char #xD800]]`, `&#x110000;`), NUL, mixed
    CR/LF/U+2028/U+0085, extreme `[[date]]` timestamps and formats,
    parser-function overflow expressions, and include syntax variants
  - corpus entries run unmodified

  Each case runs under one random configuration from a bitfield (`Cfg`).
  Caught panics are deduplicated by `file:line` and saved as `crash-*.txt`,
  with the smallest input kept. Each has a `.msg` file holding the stage,
  configuration and backtrace.
- `run <file> [cfg]` replays one case. With no configuration it tries the
  full layout × mode × include × parser-function × delayed matrix.
- `min <file> [cfg]` delta-minimizes the input while keeping the same panic
  location.

## Remaining work

1. Build the driver:
   `CARGO_TARGET_DIR=<scratch> CARGO_INCREMENTAL=0 cargo build --release` in
   `fuzz/stable_driver`. Fix any compile errors.
2. Run a campaign, for example `supervise <out> 4 3600`. Triage
   `crash-*`, `abort-*` and `hang-*`, and minimize each with `min`.
3. For each distinct panic, find the first wrong step and fix it there, not
   with `catch_unwind`. Add a regression test in the repo's existing style
   (see `tests/security_regressions.rs`,
   `doubled_block_score_marker_fails_closed_without_panicking`). Use a
   corpus fixture where the behavior is pinned to live Wikidot.
4. Run the full suite and corpus (`cargo test`) to confirm output on valid
   input is byte-identical. Then run `cargo fmt --check` and
   `RUSTFLAGS='-D warnings' cargo clippy --tests --no-deps`.
5. Update this report: list each panic with its repro, root cause and fix.
6. Decide whether to keep `fuzz/stable_driver/` in the repo or drop it once
   the findings are pinned as regression tests.
