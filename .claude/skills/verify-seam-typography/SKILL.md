---
name: verify-seam-typography
description: Use when a change touches src/delayed/seam.rs (rewrite_elements, flush_group, resolve_static_suppressions, resolve_bound_suppressions, dash and text seam edits) or wikidot_seam_edits / native_seam_edits / substitute_wikidot in src/preproc/typography.rs. Checks that removing a suppressed block between two text segments does not change the typography of the surrounding text.
---

# Verify seam transparency of Wikidot typography

## What is promised, and what breaks if it is not

The delayed pipeline (`src/delayed.rs`, `src/delayed/seam.rs`) typographically substitutes each authored text segment on its own, then removes suppressed blocks (`[[iftags]]` bodies that are hidden, comments, hidden generated values). Each removal leaves a **seam**. `wikidot_seam_edits` and `collect_dash_edits` then redo only the typography matches whose text crosses a seam, so that the result reads as if the suppressed block had never been in the source.

**Property.** For text segments separated by seams: substitute each segment alone with `substitute_wikidot`, resolve the seams with `resolve_static_suppressions(.., true)`, and join the text. That must equal `substitute_wikidot` of the plain concatenation of the raw segments. It must also not panic and leave no seam behind.

**Harm.** Quotes, `<<` `>>`, and `...` next to a hidden block come out different from the same text without the block: a straight quote where a curly one belongs, or the reverse.

## Run it

```sh
verification/seam-typography/run.sh            # the check, ~30 s once built; "MATCHES known finding classes" is a pass
verification/seam-typography/run.sh mutants    # can-it-fail self-check, ~6 min; expect 3 "caught:" lines
```

Both compile Rust. Under `docs/agents/herdr-build-approval.md` a non-controller agent requests build approval first. The check is the `#[ignore]` test `seams_are_transparent_to_wikidot_typography` in `src/delayed/seam_equivalence.rs` (compiled only under `cfg(test)`). `run.sh` compares the failure classes it finds with `verification/seam-typography/known-findings.txt`. `FTML_SEAM_TOKENS=6` is a 12x larger run (about 10 minutes) and finds at least one more class (a space-related one) that is not in the known list; it is for investigations, not routine use.

## Tool and why

Exhaustive enumeration through the real functions. Every sequence of up to 5 items from 10 text tokens (`"` `'` `a` space `.` `...` `<` `>` `<<` `>>` `é`) and a seam is built into text segments and seams (271,453 inputs), and compared with the oracle above. A failure is named by the characters on which the two strings disagree (sign only), so one root cause is one class.

## Current result: four known failure classes, backed by live Wikidot evidence

The classes share one cause: a segment is typographed **before** its left context is known, and the seam edits redo only matches that cross the seam.
- `<<` / `>>` chains: `"` seam `>>>>` gives `"»>>`, but the joined text gives `"»»`; `<` seam `<<<` gives `<«<` instead of `««`.
- `.` seam `...` gives `.…` where the joined text gives `….`; `.` seam `.....` gives `.…..` instead of `……`.
- An ordering class where the same characters come out in a different order (the ellipsis case above is its shortest member).

**Live Wikidot evidence** (`verification/seam-typography/evidence/wikidot-2026-11-01.json`, captured on http://sandbox-for-codex.wikidot.com/ with a run-owned page, since deleted): for all five sources Wikidot renders a hidden `[!-- c --]` comment or a false `[[iftags +zz-no-such-tag]]x[[/iftags]]` between two segments exactly as if the block were absent (`"»»`, `««`, `….`, `……`). So the oracle in `seam_equivalence.rs` is right for these classes.

**Plain FTML parse against the same evidence** (`examples/render_html_jsonl`, Layout::Wikidot): hidden comments match Wikidot in every case. A false `[[iftags]]` does not: `<` + iftags + `<<<` gives `<«<` (Wikidot `««`), `.` + iftags + `...` gives `.…` (Wikidot `….`), `.` + iftags + `.....` gives `.…..` (Wikidot `……`). The `"` + iftags + `>>>>` case matches. So at least the iftags half of this is a reachable parity difference, not only a delayed-pipeline detail. The check itself exercises only `resolve_static_suppressions`.

`,,` was dropped from the alphabet: it is subscript markup in Wikidot, so `,,` + `,,''` is not a typography case (live Wikidot and FTML both render `''` with or without a hidden block).

Next step: fix the seam logic (and the plain-parse iftags ordering) so the cases in the evidence file match Wikidot, then delete the fixed classes from `known-findings.txt` in the same change. If the classes are fixed, extend the evidence file with the new cases; do not narrow the alphabet to hide a class.

## Scope: what a pass covers

- Verified: the failure classes above are the only ones for inputs of up to 5 tokens from the 10-token alphabet, `apply_typography = true` (Wikidot), `TypographyBoundary` seams, text elements only.
- Not covered: dash runs (`--`, `---`) across a seam (they become separate dash elements, not text, so the alphabet excludes them; `collect_dash_edits` is unchecked); the native layout (`apply_typography = false`, `native_seam_edits`); digit and unit spacing (`replace_number_spaces`, `replace_unit_spaces`); newlines and paragraph boundaries; bound suppressions with generated values (`resolve_bound_suppressions` error paths); non-text elements between segments; inputs longer than 5 tokens.
- A pass means "the failure classes are exactly the known ones", not "no differences". The known ones are parity differences with live Wikidot, so a pass does not mean the code is right.

## Coverage record and sync commit

| Checked code | Entry point |
| --- | --- |
| `src/delayed/seam.rs`: `resolve_static_suppressions`, `rewrite_elements`, `flush_group`, `emit_source_range` | `resolve_static_suppressions` |
| `src/preproc/typography.rs`: `wikidot_seam_edits`, `substitute_wikidot` (as oracle) | via `flush_group` and the oracle |

Not exercised: `collect_dash_edits`, `range_crosses_seam` in `seam.rs` (dash path only), `native_seam_edits`, `resolve_bound_suppressions`.

**Sync commit: `68cf7e9ce`** (`seam.rs`, `typography.rs` and `delayed.rs` as of that commit; the check adds only test-only files).

Staleness test: `git diff <sync commit> -- src/delayed/seam.rs src/preproc/typography.rs src/delayed.rs`. If it shows changes to the seam rules, the typography regexes, or `TypographyBoundary` handling, rerun both commands, update `known-findings.txt` only for classes you understand, and only then move the sync commit to the new `HEAD`.

## Reading a result

- `MATCHES known finding classes`: unchanged. Done.
- `+ <class>|<len>|<count>` unknown class, or a known class with a different shortest length or count: behavior of the seam code changed. Run `cargo test --lib seam_equivalence -- --ignored --nocapture` and read the `CLASS` line: it ends with the shortest example, as `segments "a" | "b"` (`|` separates segments, a seam sits between them), what the code produced and what the joined text produces. Decide with live Wikidot output, not by editing the oracle.
- `- <class>`: a class went away. If you meant to fix it, delete the line. If you did not, a mutation or a rule change is hiding it.
- `CHECK DID NOT RUN`: build error or the test was renamed.

## Mutants: proof that the check can fail

`run.sh mutants` copies the repo to a temp dir and breaks `src/delayed/seam.rs` three ways: seam edits never applied; text after an edit emitted from the wrong place; seam positions not sorted and de-duplicated. Each must change the class list or the failure counts. A candidate that edits `range_crosses_seam` in `seam.rs` was dropped as equivalent here (that function serves the dash path only, outside the alphabet).

## Prerequisites

Rust toolchain from `rust-toolchain.toml`, Python 3 with `-I`. Uses the canonical `target/`; `run.sh mutants` uses a temporary target and deletes it.
