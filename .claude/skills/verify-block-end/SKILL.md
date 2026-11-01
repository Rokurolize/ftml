---
name: verify-block-end
description: Use when a change touches how a paired block finds its closer - verify_end_block and get_body_generic in src/parsing/rule/impls/block/parser.rs, src/wikidot_code.rs, the code/math/footnote/collapsible/bibliography block rules, or the preprocessing passes that move text around blocks (src/preproc/{whitespace,comments,compatibility}.rs). Checks that a closer followed by trailing blanks is accepted exactly like a bare one, and that nested same-name pairing stays as recorded.
---

# Verify block-end machinery

## What is promised, and what breaks if it is not

A paired block (`[[code]]`, `[[math]]`, `[[footnote]]`, `[[collapsible]]`, `[[bibliography]]`) scans forward for its closer. Three incidents in two days came from one bug class, the scan overrunning or missing its closer: math in footnote (#773), code closers with trailing whitespace (#776, one page's block swallowed following text), collapsible crossings in the preproc passes (#771). The same gate then missed `[[/math]] ` and `[[/bibliography]]  ` (#780), and the global backslash-newline join corrupted code and math bodies (#783).

**Properties** (for a block whose closer is followed only by spaces and tabs to the end of its line, in every enclosing context: top level, div, collapsible, footnote):

- P0 recognition: the block is parsed as a block, as with a bare closer.
- P1 termination: text after the closer is never inside the block.
- P2 closer equivalence: the parse summary equals the bare-closer summary.
- P3 pairing: for `OPEN A OPEN B CLOSE C CLOSE D` the nesting depth of A..D matches `verification/block-end/recorded-pairing.txt` (behavior bound by live-evidence tests, so a change here is a binding change, not a fix).

**Harm.** A closer rejected here turns a whole block into literal text, and an overrun swallows following content, and with it later code blocks and their hosted `/local--code/N` numbers.

## Run it

```sh
verification/block-end/run.sh            # the check, seconds once built
verification/block-end/run.sh mutants    # can-it-fail self-check, ~3 minutes of compiling; expect 3 "caught:" lines
```

The check is the `#[ignore]` tests in `src/block_end_closers.rs` (compiled only under `cfg(test)`). 5 block types x 4 contexts x 7 bodies x 5 blank variants x 3 line endings, about 1,980 parses on the real parser. Exit 0 means "findings equal `known-findings.txt` (currently none) and the pairing table equals the record".

## Tool and why

Exhaustive enumeration on the real parser with `catch_unwind`, no model: the reference for P0 to P2 is the bare closer, and Wikidot's acceptance of trailing blanks is established by live evidence (scp-9844 code, scp-099-fr math, scp-7451 bibliography). A hand model of the end-follower gates would not be checkable against the code.

## Scope: what a pass covers

- Verified: trailing space/tab variants (` `, `\t`, two spaces, space-tab, tab-space) after the closer, three line endings, bodies including a stray `[[/cod` and `a ]] b`, the four wrapping contexts.
- Not covered: closers followed by anything other than blanks (frozen live bindings such as `[[math]] A [[/math]] [[math]] ...` bind those); quote-prefixed (`>`) contexts; comments inside blocks; preprocessing passes (use the probes below); the Deepwell raw scan (below).

## Probes (need the shared Wikidot corpus; not part of run.sh)

All in `src/block_end_probes.rs`, driven by environment variables, run as `cargo test --lib <name> -- --ignored --nocapture`:

- `wikidot_html_counts`: with `BLOCK_END_PAGES` (file of `<lang>/pages/<slug>/source.wikidot.txt` paths) and `BLOCK_END_CORPUS`, prints Wikidot (retained `html.xmlrpc.html`) vs FTML counts of code blocks and collapsibles per page. This is the differential that found #781 and #783. Pages with `[[include]]`, runtime modules (`Members`, `Comments`, `ThemePreviewer`) or `[[iftags]]` are harness artifacts: confirm any suspect through Standing's `wikidot_page_preview` before believing it.
- `probe_code_minimize` (`BLOCK_END_PROBE`, optional `BLOCK_END_BALANCE_MATH` / `BLOCK_END_BALANCE_FOOTNOTE`): delta-minimizes a failing page to a reduction. A reduction is a guide only; fix against the full page. `probe_code_suffix_bisect` finds the nearest preceding line that breaks a block. `probe_render_file` renders one file (`BLOCK_END_HTML=1` prints the html).
- `raw_scan_vs_ftml_code_blocks` compares Deepwell's `scan_compat_code_blocks` (a verbatim copy in `src/compat_scan_probe.rs`, which goes stale if `deepwell/src/services/render/compat/fallback_code.rs` changes) against ftml's recognized code blocks.
- Always preprocess with `preprocess_for_layout(.., Layout::Wikidot)`. Plain `preprocess` is the native layout, silently skips every Wikidot compatibility rule, and makes a probe agree with itself while disagreeing with Deepwell.

## Coverage record and sync commit

| Checked code | What the check exercises |
| --- | --- |
| `src/parsing/rule/impls/block/parser.rs`: `verify_end_block` end-follower gate, `get_body_generic` nested math counting | closer-variant tests, pairing table |
| `src/wikidot_code.rs`: `candidates`, `code_closer_owns_physical_line` | code rows of the closer-variant test |
| `src/parsing/rule/impls/block/blocks/{code,math,footnote,collapsible,bibliography}.rs` | via the parser |

Sync commit: set when this branch is merged (see `git log -1 --format=%H -- verification/block-end`).

Staleness test: `git diff <sync commit> -- src/parsing/rule/impls/block/parser.rs src/wikidot_code.rs src/preproc`. A new end-follower gate, a new block with a line-owned closer, or a new pass that moves text across block boundaries needs a row in `BLOCKS` or a probe run before the sync commit moves.

## Reading a result

- `MATCHES known findings and recorded pairing`: done.
- `+ F|<block> in <context> body=...: P0/P1/P2 ...`: a new finding. Run `cargo test --lib block_end_closers -- --ignored --nocapture` and read the `MINIMAL-FAILURE ... source=` line for the shortest failing source. Establish Wikidot's behavior from retained html first (corpus search for the shape, then a render compare). Not every finding is a defect: a frozen live binding may require the behavior; then stop and report, never re-record.
- `+`/`-` on a `P|` row: the nested-pairing record changed. That is a binding change; do not edit `recorded-pairing.txt` to make it pass.
- `CHECK DID NOT RUN`: build error or a renamed test.
- `MUTANT SURVIVED`: the check got weaker, or a mutant's search text no longer occurs once.

## Mutants

`run.sh mutants` copies the repo to a temp dir and applies: (1) math and bibliography lose the blank-remainder exemption (the #780 bug); (2) the code closer scan rejects any suffix (the #776 bug); (3) math stops counting nested openers (changes the pairing table).

## Prerequisites

Rust toolchain from `rust-toolchain.toml`, Python 3 with `-I`.
