---
name: verify-process-depths
description: Use when a change touches src/parsing/depth.rs (process_depths, DepthStack), src/non_empty_vec.rs, or how src/parsing/rule/impls/list.rs, blockquote.rs or src/parsing/mod.rs (table of contents) feed or read depth lists. Checks that list/blockquote/TOC nesting never loses, reorders, mis-nests or mistypes a row.
---

# Verify `process_depths` list nesting

## What is promised, and what breaks if it is not

`process_depths` (`src/parsing/depth.rs`) turns the flat rows `(depth, list type, row)` that the list rule, the blockquote rule and the table-of-contents builder collect into nested `DepthItem::List` trees.

**Property.** For every input inside the bounds below:
1. it does not panic (`No depth to pop off!`, the `open layers remain` assert);
2. every row comes out exactly once, in input order;
3. a row's nesting depth in its tree equals its input depth (tree root = 0);
4. a row sits directly in a list of its own list type (so `*` and `#` rows never swap kinds);
5. no empty tree and no empty sub-list is returned.

**Harm.** A violation silently drops or reorders wiki list items, indents them wrongly, or turns a bulleted item into a numbered one, with no error.

## Run it

```sh
verification/process-depths/run.sh            # the check: expect "PASS", ~5 s after first build
verification/process-depths/run.sh mutants    # can-it-fail self-check: expect 5 "caught:" lines, ~25 s
```

Both run Cargo. `run.sh` uses its own target dir (`verification/process-depths/target`, git-ignored) and compiles the real `src/parsing/depth.rs` and `src/non_empty_vec.rs` by path. It is not a model or a copy. Bounds can be raised: `FTML_DEPTH_ROWS=7 FTML_DEPTH_MAX=3 FTML_DEPTH_TYPES=3 verification/process-depths/run.sh` (about 12x the work per extra row).

## Tool and why

Exhaustive enumeration: the test (`verification/process-depths/tests/exhaustive.rs`) feeds `process_depths` every row sequence up to the bounds and checks the property on each output. Kani (a Rust model checker) was tried first and abandoned: the recursive `Vec<DepthItem>` drop glue made CBMC run more than 9 minutes at only 3 rows, depth up to 2, 2 types. Enumeration covers 3,257,437 inputs in about 4 s on the same code.

## Scope: what a pass covers

- Verified: up to **6 rows**, depths **0..=3** (every jump up or down, including 0 straight to 3), **3 list types**, `process_depths` with the generic `L: Copy + PartialEq`, instantiated with `u8`. All inputs inside that box.
- Not covered: inputs larger than the box (a pass is evidence, not a proof for all sizes; the algorithm has one stack, so a break at larger size almost always shows at small size); the callers' own preprocessing (`retain_active_nested_list_types` in `list.rs`, `merge_adjacent_native_list_rows` in `blockquote.rs`); how `build_list_element`, `build_blockquote_element` and `build_toc_list_element` consume the trees; Wikidot parity of the resulting HTML (that is the parity corpus's job).
- Last full result when written: **PASS, no finding.** The current code satisfies all five points.

## Coverage record and sync commit

| Checked code | What the check relies on |
| --- | --- |
| `src/parsing/depth.rs`: `process_depths`, `DepthStack::{new, increase_depth, decrease_depth, new_list, push, push_item, last_type, finish_depth_list, into_trees}` | the whole file is compiled |
| `src/non_empty_vec.rs`: `NonEmptyVec` | compiled alongside |

Callers whose assumptions matter: `src/parsing/rule/impls/list.rs` (`process_depths(ListType::Generic, ..)`), `src/parsing/rule/impls/blockquote.rs` (`process_depths((), ..)`, depths are `depth - 1`), `src/parsing/mod.rs` (`process_depths((), table_of_contents_depths)`).

**Sync commit: `3f8be45c1`** (the commit at which this check was confirmed to describe the code).

Staleness test: `git diff 3f8be45c1 -- src/parsing/depth.rs src/non_empty_vec.rs`. If it prints anything, or a caller starts passing different inputs (for example a depth that is not "previous or smaller, or any larger", a new list type, a non-`Copy` list type), re-read the diff, update the harness if the property or signature changed, rerun both commands, and only then move the sync commit above to the new `HEAD`. Do not move it without a passing run of both commands.

## Reading a failure

A failure prints the smallest-first row sequence, for example `rows [(depth 1, type 0), (depth 0, type 0)]: row 1 came out at nesting depth 1 not 0`. Rows are listed in input order as `(depth, list type)`; "row N" is the 0-based position. To decide whether it is a real bug or a harness error:
- Replay it by hand: call `process_depths(top, vec![(1, 'a', 0), (0, 'a', 1)])` in a scratch test and print the tree.
- It is a **real bug** if the sequence can occur in the callers (`list.rs` and `blockquote.rs` emit any depth >= 0; blockquote depths are token count minus one; the TOC builder emits heading depths).
- It is a **harness error** if the property text no longer matches an intended change (for example you deliberately stopped splitting trees on a type change). Then edit the property in `tests/exhaustive.rs` and the list at the top of this file together, and say so in the PR. Never loosen the property or the bounds just to turn a real failure green; report the counterexample instead.

## Mutants: proof that the check can fail

`run.sh mutants` copies `depth.rs` to a temp directory and breaks it five ways: type change ignores the new list, one level not closed on a depth decrease, empty top-level trees kept, closed sublist dropped, new levels not opened on a depth increase. All five must be reported `caught:`. A `MUTANT SURVIVED` line means the check got weaker; fix `tests/exhaustive.rs`. If you edit `depth.rs` so a mutant's search text no longer matches, the script stops with an assertion; update the text in `run.sh` to match the new code.

## Prerequisites

Rust toolchain from `rust-toolchain.toml` and Python 3 (for the mutant script). No other tools. Cargo may resolve nothing from the network: the crate has no dependencies.
