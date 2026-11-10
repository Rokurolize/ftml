---
name: verify-ftml-render
description: Check ftml footnote and bibliography render stacks, legacy equation numbering, Wikidot tabview nav assignment and stray-closer lifts, MathML sanitizer emission, and syntax-highlight token coverage. Use when a change touches enter_footnote_ref, exit_footnote_ref, enter_bibliography_ref, exit_bibliography_ref, preregister_wikidot_equations, render_math_block, sanitize_mathml, simulate_tabview_navs, render_wikidot_tabview, flush_wikidot_tabview_lifts, tokenize, or highlight_code.
---

# Verify ftml render

## The promise

While a footnote or bibliography body is rendering, that ref sits on its stack, so a self-cite does not recurse, and every enter pairs with one exit. Legacy equation numbers are assigned when the preregister walk reaches the footnote reference, so an `eref` and the block agree. A nested Wikidot tabview takes the flat nav assignment, a second assignment of the same view is rejected, each outermost view restores its entry depth and emits its own queued `[[/tab]]` lifts, and a content-only outer does not emit a nested view's lift. `sanitize_mathml` emits only allowlisted tags and attributes, and a mismatched close stays escaped. `tokenize` returns, and its tokens stay on char boundaries and cover the preprocessed source.

**Harm when broken.** A self-cite overflows the stack, or a missed pop hides a later cite. An equation number swaps between the page and the footnote footer. A nested tabview shows the wrong tabs, a stray `[[/tab]]` leaks into the next view or is dropped, or depth stays high so the next view never lifts. MathML emits a tag outside the allowlist or closes the wrong element. Highlighting panics on a multibyte boundary or drops the tail of the source.

These checks walk an existing `SyntaxTree` or input bytes. They do not include a parse rule. The parser formal-verification program owns `verification/block-end`, `verification/include-expansion`, `verification/paragraph-stack`, `verification/process-depths`, and `verification/seam-typography`. RT-03, the parse/render clock, is out of this skill. RT-09 stays deferred, because those markers are stamped by parse rules and `omit_internal_attribute_marker` only drops `data-ftml-` keys. Do not extend this skill to parser or preprocessing machines, or to machines owned by another program.

## Run it

```bash
verification/render/walk/check.sh
verification/render/emission/check.sh
```

Run both. The walk check needs Java 11+ and Python 3. It downloads `tla2tools.jar` v1.7.4 to `~/.cache/tlaplus/1.7.4/` (pinned by sha256), or uses `$TLA2TOOLS_JAR` if set. Cargo goes through `$FTML_HEAVY_BUILD` when that wrapper exists, otherwise `cargo` on `PATH`. To keep the generated config and TLC logs, set `FTML_RENDER_WALK_WORK=<dir>`.

`verification/render/walk/check.sh mutations` flips one extracted constant at a time and requires the matching invariant to fail. `verification/render/walk/check.sh scratches` applies the same breaks to the Rust functions, runs the one tree or nav test that should catch each break, and restores the file from a copy. `verification/render/emission/check.sh mutations` does that for the sanitizer and the highlighter.

There is no Rust verifier in this repo. Walk order is TLC on a TLA+ model of the steps, plus generated trees through `HtmlRender`. Emission is proptest on the real private functions, included into the parent test module so those functions stay private.

Exit codes for the walk check:

- 0 means the pass invariants hold, the witnesses that must fail do fail, and the tree tests pass.
- 1 means TLC found a new violation, a witness that must fail did not fail, or a tree test failed.
- 2 means an anchor moved (`MODEL DRIFT`), TLC exited without a counterexample, or Java or Python is missing.

Exit codes for the emission check:

- 0 means the property tests passed.
- 1 means a property failed, or a mutation was still accepted.
- 2 means a mutation anchor is no longer unique.

`scratches` and `mutations` exit 0 when every broken variant is caught. They exit 1 when a broken variant still passes. They exit 2 when an anchor is missing. Both restore from the copy they took. Do not `git stash`, and do not `git checkout` a scratched file while the tree has other edits. If a run is killed, `git diff` the product files listed below and copy back anything that is not the `cfg(test)` include.

Read every TLC result with the pstack `/statechart` skill. For a violation, it turns the trace into the shortest path in plain words. For a pass, it draws how the system moves between states.

## Expected result at the sync commit

State counts are the stable part. Durations vary.

```
PASS  ref (713 distinct states found)
EXPECTED-FAIL ref-reenter (195 distinct states found)
Error: Invariant NeverReentered is violated.
PASS  eq (24 distinct states found)
EXPECTED-FAIL eq-number (6 distinct states found)
Error: Invariant NeverNumbered is violated.
PASS  nav (402 distinct states found)
PASS  lift (21 distinct states found)
EXPECTED-FAIL lift-emit (13 distinct states found)
Error: Invariant NeverEmitted is violated.
PASS  trees
PASS  emission
```

`ref-reenter`, `eq-number`, and `lift-emit` are witnesses. They fail because a self-cite, an equation number, and a lift are reachable, so the passes above are not empty. There is no finding file. Do not weaken an invariant to make a witness pass.

`mutations` then prints `EXPECTED-FAIL` for `m-fn` (`NoDupFn`), `m-exit` (`NoUnder`), `m-bib` (`NoUnder`), `m-eq` (`NumbersAgree`), `m-nav` (`DupRejected`), `m-flush` (`LiftsConsumed`), and `m-depth` (`DepthRestored`). `scratches` prints `caught` for the footnote guard, the footnote exit, the bibliography reentry guard, the equation footnote walk, the duplicate tabview assignment, and the lift flush. Emission `mutations` prints `caught` for the mismatched MathML close and the highlight tail.

## What the model is

Four specs, plus tree tests and two property tests. `extract.py` reads the anchors on every walk run and writes the constants TLC uses. A renamed guard changes the constant and changes what TLC checks.

| Model element | Code |
| --- | --- |
| `FnContains` | `HtmlContext::enter_footnote_ref` returns false when `footnote_render_stack` already contains the index. |
| `FnExitOnlyElements` | `render_footnote` calls `exit_footnote_ref` only when `tooltip_text` is `None`. A tooltip never pushed, so it must not pop. |
| `BibLegacyReturns` | On the legacy layout, `render_bibcite` exits the bibliography stack and returns. The tail exit is the non-legacy path. |
| `Enter`, `Body`, `Exit` | One ref render. A body may start a child ref. `NoDupFn` and `NoDupBib` are the self-cite guards. `NoUnder` is the paired pop. `Balanced` requires both stacks empty when no frame is open. |
| `WalkFootnoteBodies` | `preregister_wikidot_equations` pushes the footnote body when it sees the reference. |
| `PreEref`, `RenderA`, `RenderP` | `[[eref]]` and `render_math_block` read the number that walk stored. `NumbersAgree` says the footnote equation is 1 and the page equation is 2 for the ref-first shape. |
| `AbortOnDuplicate` | `simulate_tabview_navs` returns `None` when `assignments[view_index]` is already `Some`. |
| `KnownPinned` | The outer view open, its tab, the inner view, and the following outer tab assign `<<0>>` and `<<1, 2>>`. |
| `CoversOrNone` | A short sequence assigns only tab indexes that appeared as tab-opens, or it assigns nothing. |
| `FlushOuter` | `flush_wikidot_tabview_lifts` emits one `<br>[[/tab]]` per queued lift taken from `take_wikidot_tabview_lifts`. |
| `RestoreDepth` | `render_wikidot_tabview` sets `wikidot_tabview_depth` back to the depth it captured on entry. |
| `Arm` | The item loop arms a lift only when the entry depth is 0. A content-only view returns before that loop, so it does not arm. |
| `assert_emitted_mathml` | `sanitize_mathml`. The tag and attribute lists are `known_mathml_tag` and `known_mathml_attribute`. A close that is not the stack top is escaped, not popped. |
| `assert_token_bounds` | `preprocess` then `tokenize`. The cursor advances only on a positive-width token and must finish at the end of the preprocessed text. |

## Bounds and assumptions (what a pass does not cover)

- **Ref stacks.** One thread, at most two starts, frame depth at most 3, ids `1..2`, labels `a` and `b`. The model does not render HTML. The tree tests cover one Wikijump self-cite for footnotes and one for bibliography.
- **Equations.** Four shapes, one footnote equation named `a` and one page block. The model checks the number map, not the HTML. The tree test checks the Wikidot legacy HTML, with `equation-2` before the footer and `equation-1` inside it.
- **Tab nav.** Every event sequence of length 0 through 3 over view-open 0 or 1, view-close, tab-open 0 through 2, and tab-close, plus the known nest and one duplicate view-open. The duplicate uses one view. Remote tabs are not in the TLA alphabet. The Rust test `remote_tab_keeps_hierarchical_navs` covers that abort. Row-tail `nested_tabview_lift` and a `||` suffix are a different machine and are not in `TabLift`.
- **Lifts.** Three schedules, which are one outer view, two outer views, and a content-only outer. Pending lifts are at most 2. The tree tests check two successive stray outers and a content-only outer wrapped around a nested stray.
- **MathML.** 48 generated sequences of up to 10 opcodes, one mismatched close, and one balanced fragment. Unclosed opens may remain. The check does not parse MathML. It reads the tags the sanitizer emits.
- **Highlight.** The 13 languages `table_for` knows. Fixed samples include an empty string and the multibyte sample `aあb`. Generated cases are 16 strings of at most 24 code points. Zero-width matches stay at the cursor. Frame-stack class restoration is not this promise.
- **Not checked.** Parser rules, preprocessing, RT-03, and RT-09. The include hooks are `cfg(test)` only. Product HTML is unchanged by this skill.

## Did my change touch the verified code?

It did if the diff touches any of these:

- `src/render/html/context.rs`, the footnote, bibliography, equation, and tabview stack methods
- `src/render/html/element/footnotes.rs`, `render_footnote`
- `src/render/html/element/bibliography.rs`, `render_bibcite`
- `src/render/html/element/math.rs`, `preregister_wikidot_equations`, `render_math_block`, `sanitize_mathml`
- `src/render/html/element/tabs.rs`, `simulate_tabview_navs`, `render_wikidot_tabview`, `flush_wikidot_tabview_lifts`
- `src/render/html/element/highlight.rs`, `tokenize`, `highlight_code`
- `src/render/html/mod.rs`, only the `verify_walks` test module
- `verification/render/`

To list the covered files changed since the last sync:

```bash
git diff --stat "$(git log -1 --format=%h -- .claude/skills/verify-ftml-render/SKILL.md)" -- \
  src/render/html/context.rs \
  src/render/html/element/footnotes.rs \
  src/render/html/element/bibliography.rs \
  src/render/html/element/math.rs \
  src/render/html/element/tabs.rs \
  src/render/html/element/highlight.rs \
  src/render/html/mod.rs \
  verification/render
```

If this lists anything, run both checks.

## Reading a result

- **A new FAIL on `ref`.** `FnContains` or `FnExitOnlyElements` no longer matches the code, or a pop runs on an empty stack. A self-cite recurses, or a tooltip pops a ref it never pushed. Fix the code, not the model.
- **A new FAIL on `eq`.** The footnote body is no longer numbered at the reference, so the page block and the footer disagree. Fix the code, not the model.
- **A new FAIL on `nav`.** The known nest is not `<<0>>` and `<<1, 2>>`, a duplicate view assignment returned a function, or an assigned tab index never appeared as a tab-open. Fix the code, not the model. If a TLC step cannot happen in `simulate_tabview_navs`, the operator is wrong. Fix the spec and say so.
- **A new FAIL on `lift`.** An outermost view finished with a queued lift, a content-only view emitted one, depth was not back at 0, or the second outer view did not emit its own lift. Fix the code, not the model.
- **A new FAIL on `trees` or `emission`.** The real function broke the promise the model names. Fix the code, not the assertion.
- **`ref-reenter`, `eq-number`, or `lift-emit` starts passing.** That step is no longer reachable. The pass above may be empty. Find out why before trusting it.
- **MODEL DRIFT.** An anchor was renamed or duplicated. Open the named file. If the behavior is unchanged and only the text moved, update the anchor in `extract.py` and the matching scratch string. If the behavior changed, extend the model before trusting any result.
- **`/statechart`.** Use it on the TLC log. Do not copy its command lines into this skill. They move between releases.

Counterexamples in the code's words:

- **Footnote reentry.** A footnote frame starts, the body cites the same index, and `enter_footnote_ref` pushes again. `NoDupFn` fails. With the guard present, that second enter sets `reentered` and drops the frame instead.
- **Footnote exit.** A tooltip frame never pushed. The exit still pops. `underflow` becomes true.
- **Legacy bibliography.** The legacy frame pops, then pops again because it did not return. The second pop underflows.
- **Equation order.** The footnote body is not walked at the reference. The page block takes 1. `NumbersAgree` fails.
- **Duplicate nav.** The same view-open is assigned twice and the result is still a function. `DupRejected` fails.
- **Lift.** The outer view queues a stray closer and then skips the flush. `pending` is still 1 at `done`.
- **Depth.** The first view leaves `depth` at 1. The second view's entry is 1, so it does not arm, and it does not emit.
- **MathML.** `<mrow>...<mi>a</mi></mfrac>` emits `</mfrac>` instead of `&lt;/mfrac&gt;`.
- **Highlight.** The last token ends at `pos` rather than `len`, so the cursor never reaches the end of the preprocessed source.

## Updating the verification on an intended change

1. Change the model, the extractor, or the property so it matches the new code. Keep the promises as they are.
2. Prove that it can still fail. Run `verification/render/walk/check.sh scratches` and `verification/render/emission/check.sh mutations`. Each broken variant must be caught. The scratches are the footnote contains-check replaced by `false`, the footnote `exit_footnote_ref` call removed, the bibliography `enter_bibliography_ref` guard replaced by `false`, the equation footnote `stack.push` removed, the duplicate-view `return None` removed, the lift take replaced by `0`, the MathML close arm changed to `if true`, and the highlight final `emit` end changed to `pos`.
3. Confirm both plain checks match the expected result above. The scratch runners restore from a copy. If you scratch by hand, restore from that copy too.
4. `walk/check.sh mutations` repeats the stack, equation, nav, flush, and depth breaks as constant overrides, without editing Rust. Use it when you cannot edit the source tree. It does not replace the Rust scratches.
5. Move the sync commit forward by committing an edit to this record, but only after you have confirmed that the verification describes the code at the new commit.

Bring the verification back in line with `/maintain-formal-verification-skill` when the covered code changed and this check was not rerun.

## Coverage record

- **Sync commit:** the last commit that changed this file, given by `git log -1 --format=%H -- .claude/skills/verify-ftml-render/SKILL.md`. The checks were last confirmed against the code at that commit.
- **Scratch breaks seen to fail:** the six Rust walk scratches and the two emission scratches named above. `walk/check.sh mutations` repeats the seven constant overrides.
