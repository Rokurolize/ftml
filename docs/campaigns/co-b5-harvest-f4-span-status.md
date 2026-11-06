# co-b5 harvest F4-span (status note)

Campaign record paths:

- Family partition: `muse-bhl/co-b5/line152.json` key `F4-span` (27 ids)
- Frozen live bytes and baseline comparison columns: `muse-bhl/co-b5/b5-rerender.json` fields `src`, `n_live`, `n_ftml`
- Normaliser: `muse-bhl/co-b5/lib.py` function `norm`
- Render harness: `ftml` example binary `render_html_jsonl`, JSONL input schema `wikijump_syntax_differential.syntax_case.v1`, `layout: wikidot`

## Measurement on current trunk (informational)

At `origin/main` commit `0d876dc90` in worktree `/home/roku/.local/state/wikijump/wt-harvest-f4`, all 27 F4-span sources render with zero `ParseError` rows in `render_html_jsonl` output, and **none** of the 27 `norm(html)` strings equals the frozen `n_live` from `b5-rerender.json`.

Representative `d3-54-span-st-t` (from `b5-rerender.json`):

- Live (`n_live`): nested empty simple-table cell, bullet list with plain `i`, trailing paragraph whose text is the two pipe characters.
- Trunk FTML: different DOM (for example span wrappers and list item text shape), so the normalised strings still differ.

This file does not change parser behavior. It records the campaign baseline so follow-up work does not confuse trunk health with parity on this frozen capture set.
