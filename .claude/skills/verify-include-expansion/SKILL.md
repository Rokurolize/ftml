---
name: verify-include-expansion
description: Use when a change touches src/includes/mod.rs (`include`, `find_include_end`), src/includes/parse.rs, src/includes/quoted.rs, src/includes/include_ref.rs or the Includer trait. Checks that expanding include blocks never panics on odd text, asks for and returns the right pages, and pads the expanded text correctly.
---

# Verify `includes::include` expansion

## What is promised, and what breaks if it is not

`ftml::includes::include` (`src/includes/mod.rs`) scans page text for `[[include page | arg=val]]` blocks, asks an `Includer` for the pages, and splices their text into the source using byte offsets. It runs on untrusted page text before parsing. The repo's fuzz target (`fuzz/fuzz_targets/public_pipeline.rs`) never calls it, so nothing else exercises it on odd text.

**Property** (every input inside the bounds below, for four includers: missing page, empty page, marker page, page with variables):
1. `include` does not panic and finishes.
2. It returns exactly one `PageRef` per include block it asked the includer for, and the same blocks whichever includer is used.
3. When nothing is inserted (missing or empty page), the output is the input with text deleted: nothing is added.
4. Input with no `>` and no include asked for comes back unchanged.
5. An expanded page gets exactly one line ending on each side that has caller text, copying `\r\n` when the caller used it (`include_padding_follows_surrounding_line_endings`, a fixed grid of prefixes, suffixes and three include spellings).
6. A marker includer and a missing-page includer agree about which text is consumed, for inputs without line breaks.

**Harm.** A panic aborts rendering a page from page text alone; wrong consumption or padding silently eats or misplaces the author's text around an include.

Tight quote-prefixed includes (`>[[include x]]`) are consumed without being fetched on purpose (Wikidot parity; comment in `include`), so property 4 excludes input containing `>`.

## Run it

```sh
verification/include-expansion/run.sh            # expect "PASS", ~2 s after the first build
verification/include-expansion/run.sh mutants    # expect 4 "caught:" lines, ~3 min
```

Both compile Rust (the first run builds ftml into `verification/include-expansion/target`, git-ignored, several minutes). Raise the bound with `FTML_INCLUDE_TOKENS=6` (about 18x longer).

## Tool and why

Exhaustive enumeration through the public API. `tests/exhaustive.rs` is a standalone crate that depends on `ftml` by path and, for every string of up to 5 tokens from an 18-token alphabet (include opener and closer, `[[`, `]]`, spaces, `|`, `=`, `>`, `-- `, `é`, a 4-byte emoji, `\n`, `\r\n`, ...), runs `include` with four includers. That is 2,000,719 inputs per run. No source file is edited. Kani was not used (it did not scale on the recursive list code, see `verify-process-depths`); a model checker for ordering was not needed because `include` is a single pass over the text.

## Scope: what a pass covers

- Verified: the five properties above for inputs of up to 5 tokens from the alphabet, `WikitextMode::Page`, `Layout::Wikidot`.
- Not covered: longer inputs; characters outside the alphabet; **which** text counts as an include block (the check never states the grammar, so a parser that consumes the wrong range consistently passes); variable substitution results (`replace_variables`); `Layout::Wikijump`; `enable_page_syntax = false` (returns input unchanged); includers that fail or return mismatched pages (the `invalid_return` path).
- Result when written: **PASS, no finding** at 2,000,719 inputs.

## Coverage record and sync commit

| Checked code | Entry point |
| --- | --- |
| `src/includes/mod.rs`: `include`, `find_include_end` | `ftml::includes::include` |
| `src/includes/parse.rs`, `src/includes/quoted.rs`, `src/includes/include_ref.rs` | reached through `include` |

**Sync commit: `3f8be45c1`** (the check was confirmed against these files at that commit).

Staleness test: `git diff 3f8be45c1 -- src/includes`. If it shows anything, re-read it: new include syntax means adding tokens to `TOKENS`; a changed padding rule means updating property 5 in `tests/exhaustive.rs` and this file together; a changed `Includer` trait means updating the `Probe` includer. Rerun both commands, and only then move the sync commit to the new `HEAD`.

## Reading a failure

A failure prints the exact input as a Rust string literal (`"\r\n[[include é]]"`) and which property it broke. Replay it by calling `ftml::includes::include` on that string with `NullIncluder`. It is a real bug if the text can occur in a page. It is a harness error if the rule changed on purpose; then edit the property and this file together. Never shrink the alphabet or bounds to turn a real failure green.

## Mutants: proof that the check can fail

`run.sh mutants` copies the repo to a temp dir and breaks `src/includes/mod.rs` four ways: empty pages stop removing the include range; the page is not recorded; the byte index before the include is off by one; the byte index after is off by one. All four must be `caught:`. A `MUTANT SURVIVED` line means the check got weaker; if a mutant's search text no longer occurs once, the script says so and the text in `run.sh` needs updating.

## Prerequisites

Rust toolchain from `rust-toolchain.toml`, Python 3 (for the mutant script). Needs network or a warm Cargo cache the first time, to resolve ftml's dependencies for the standalone crate.
