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

Exhaustive enumeration through the real functions. Every sequence of up to 5 items from 11 text tokens (`"` `'` `a` space `.` `...` `<` `>` `<<` `>>` `,,` `é`) and a seam is built into text segments and seams (402,234 inputs), and compared with the oracle above. A failure is named by the characters on which the two strings disagree (sign only), so one root cause is one class.

## Current result: five known failure classes, none resolved

All five are the same shape: the oracle and the code disagree because a segment is typographed **before** its left context is known.
- `>>` and `<<` chains: `"` seam `>>>>` gives `"»>>` but the joined text gives `"»»` (the seam edit converts the one pair that crosses the seam; the next pair only converts because the first did).
- `.` seam `...` gives `.…` where the joined text gives `….`; `.` seam `.....` gives `.…..` instead of `……`.
- `,,` seam `,,''` gives `,,„”` where the joined text `,,,,''` is left alone.
- An ordering class where the same characters come out in a different order.

They are recorded as unresolved because the oracle may not match Wikidot. Wikidot types the raw source, including the hidden block, so for real Wikidot output neither "as if the block were absent" nor "as if the block were present" is established. By `CLAUDE.md`, live Wikidot evidence decides. Needed next: render the smallest example of each class on live Wikidot (for instance a hidden `[[iftags -x]]x[[/iftags]]` between the segments) and compare. If the code is wrong, fix `seam.rs`/`typography.rs`, then remove the class line from `known-findings.txt` in the same change. If the oracle is wrong, change the oracle in `seam_equivalence.rs` and this file together, with the evidence recorded. Do not hide a class by narrowing the alphabet.

## Scope: what a pass covers

- Verified: the failure classes above are the only ones for inputs of up to 5 tokens from the 11-token alphabet, `apply_typography = true` (Wikidot), `TypographyBoundary` seams, text elements only.
- Not covered: dash runs (`--`, `---`) across a seam (they become separate dash elements, not text, so the alphabet excludes them; `collect_dash_edits` is unchecked); the native layout (`apply_typography = false`, `native_seam_edits`); digit and unit spacing (`replace_number_spaces`, `replace_unit_spaces`); newlines and paragraph boundaries; bound suppressions with generated values (`resolve_bound_suppressions` error paths); non-text elements between segments; inputs longer than 5 tokens.
- A pass means "the failure classes are exactly the known ones", not "no differences".

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
