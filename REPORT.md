# Failed-div closer bridge (stray `[[/div]]` before an enclosing closer)

## Live characterization
Anonymous `edit/PagePreviewModule` on `sandbox-for-codex.wikidot.com`, masked tabview ids.

Live behavior: a stray `[[/div]]`, `[[/span]]` or `[[/footnote]]` inside an open block body stays literal text, and the enclosing closer (`[[/tab]]`, `[[/collapsible]]`, `[[/cell]]`, `[[/li]]`) still takes effect. The whitespace between the two closers is dropped.

Shapes confirmed (same line, tab-separated, two strays, text between, newline-then-closer, followed by a second tab):
- `x [[/div]] [[/tab]]` in a tab keeps the tabview and renders `<p>x [[/div]]</p>`.
- The same shape works for collapsible, table cell and ul/li, and for stray span/footnote closers.
- `x [[/div]] [[/div]] [[/tab]]` renders `x [<a href="/div]]">[[/div</a>]` and the tab still closes.
- `[[/tab]]\n[[/div]] [[/tabview]]` (stray after the tab closed) fails the whole view, literal. ftml already matched this.
- Top level, no enclosing body: `x [[/div]] [[/tab]]` forms the link. ftml already matched this.

## Root cause
`try_consume_wikidot_unmatched_inline_close_link` (src/parsing/consume.rs) turned `[[/div]] <label>]` into a link. It took the enclosing closer (`[[/tab`) as the label, so the body never closed and the tabview failed.

## Fix
`wikidot_label_region_has_body_closer`: the link is skipped when any `[[/` inside the label region is accepted by an enclosing body-end boundary. The stray closer then stays literal and the body closes.

## Fixtures (Wikidot layout)
Each input was compared whole against live PagePreview. The only difference was `style="display:none"` on non-first tabs, which ftml's renderer already omits without any stray closer.
- tabview/tab-failed-div-closer-bridge
- tabview/tab-failed-span-footnote-closer-bridge
- tabview/tab-failed-div-closer-after-tab-fails-view (negative)
- tabview/tab-failed-div-closer-toplevel-link (negative)
- collapsible/failed-div-closer-bridge
- table/failed-div-closer-bridge-cell
- list/failed-div-closer-bridge-li

## STATUS
**Done**
- Live matrix run and fix implemented.
- Seven fixtures generated, scoped to the new directories only, with no unrelated fixture rewrites.
- Wikidot-layout output matches live on all cases except the `display:none` artifact.
- WIP commit pushed earlier.

**Partial**
- The final validation was interrupted. `cargo fmt --check`, the full `cargo test` and clippy have not been confirmed after the fixtures were added. An earlier full `cargo test` ran with the fix but before the fixtures were complete, and its only failures were the fixture-loader errors for the half-created fixtures.
- Only the fixture names were checked in non-update mode, via the update run.

**Remaining**
- Run `cargo fmt --check`, `cargo test` and `RUSTFLAGS='-D warnings' cargo clippy --tests --no-deps`.
- Check that no fixture from other sections changed.
- `[[size]]` bodies: live closes at `[[/size]]` after a stray div, but ftml still links. Size is inline and not a body-end boundary, so this needs a separate fix.
- `[[html]]` bodies: live and ftml differ (live literal, ftml iframe). This is not caused by this change.
