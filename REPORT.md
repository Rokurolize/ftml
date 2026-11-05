# SCP-JP real-page divergence sweep

Discovery-only branch: no library source (`src/`, `tests/`, `test/`, `examples/`) is changed.
Sweep tooling and data live under [`sweeps/scp-jp/`](sweeps/scp-jp/).

## STATUS

Stopped early on request. **No divergence families have been established yet**:
no page has been rendered locally, so there are no diffs and no minimized
repros. Treat this branch as the working pipeline plus groundwork findings.

| Stage | State | Notes |
|---|---|---|
| Candidate selection (ListPages by rating) | done | `sweeps/scp-jp/data/candidates.json`: 130 mainlist `scp-NNN-jp` + 80 `+tale +jp` pages, rating-ordered, with provenance |
| Wikitext fetch + decode (ViewSourceModule) | done, validated | Decoded length equals ListPages `%%size%%` exactly on scp-040-jp, scp-444-jp, scp-1111-jp |
| Live HTML fetch + `#page-content` extraction | done (code) | Balanced-div extraction; provenance includes the full-response SHA-256 |
| Capture of all candidates (source + live) | **partial** | Stopped at 65/210 into an ephemeral scratch cache; not committed, and formatting stats were not written back to `candidates.json` |
| Heavy-formatting selection (`fetch.py select`) | not run | Scoring is implemented: markup density × log(rating), with weights for ruby, anchors, and CJK-adjacent emphasis |
| Materialize pages + recursive include prefetch | not run | `fetch.py materialize` |
| Include expansion (`tools/include_expand`) | built, not exercised on real pages | Loops `ftml::include` passes like Wikidot (default cap 5) |
| Render with harness (`render.py`) | written, not run | `examples/render_html_jsonl`, `Layout::Wikidot`, `existing_pages` taken from live link state |
| DOM normalization, id/timestamp masking, diff | **not written** | Design below |
| Clustering into families, minimization, PagePreview confirmation | **not started** | |

### What remains, in order

1. Re-run `fetch.py capture` to completion, then `select` and `materialize`.
2. Run `render.py` (needs `cargo build --example render_html_jsonl` and the expander build).
3. Write `compare.py`: parse both sides with html5lib, mask per-render values, serialize canonically, and write unified diffs per page.
4. Cluster the hunks (rule-based classifiers, iterated by hand), minimize each family, and confirm the minimized sources via PagePreview. Use `scp-jp` when a family depends on locale; otherwise use the repo's usual `sandbox-for-codex`. Treat PagePreview-only verdicts as provisional, per CLAUDE.md.
5. Before probing live for any family, check it against existing pins and frozen captures (`tests/fixtures/wikidot-parity/references-*.jsonl`).

## Method (as built)

All access is anonymous and read-only, paced at about 1 request per second, with a generic User-Agent.

- **Selection.** `list/ListPagesModule` via `ajax-module-connector.php` (`order="rating desc"`), queried with tags `+scp +jp` (filtered to `^scp-\d+-jp$`) and `+tale +jp`.
- **Wikitext.** `viewsource/ViewSourceModule` with `page_id`, taken from `WIKIREQUEST.info.pageId` on the page HTML. The module body is presentation HTML. `wd.decode_view_source` undoes exactly these transforms:
  - `htmlspecialchars`;
  - an `<a>` link around each `[[include]]` target;
  - nl2br (`<br />` before each newline);
  - `&nbsp;` for runs of spaces;
  - the template's leading `\n\t` and trailing `\n`.

  A literal U+00A0 in the source survives, because only the `&nbsp;` entity is mapped to a space.
- **Live render.** `GET https://scp-jp.wikidot.com/<slug>`, keeping the inner HTML of `<div id="page-content">`. Source and live HTML are fetched back-to-back so the revision matches.
- **Includes.** Wikidot expands includes before parsing. The render harness does not expand them, so `tools/include_expand` (a standalone crate with its own `[workspace]` and a path dependency on this checkout) calls the public `ftml::include` with a cache-backed `Includer`. It re-runs whole-source passes until nothing changes or the cap is reached. A missing include becomes a distinctive `SWEEP-MISSING-INCLUDE site:page` error-block, so it shows up in the diff; the real message is caller-owned and has not yet been observed live.
- **Caller-runtime inputs.** `existing_pages` comes from the live HTML: an internal `<a>` without class `newpage` names an existing page. The harness hard-codes `language: "en"` in `PageInfo`, so any locale-dependent strings will need classifying as caller-runtime, not as parser divergences.
- **Planned comparison.** ftml emits compact HTML (`<br>`, no inter-block newlines). Live emits newline-formatted markup (`<br />`). Comparison must therefore be DOM-based:
  - parse both sides with html5lib as a fragment;
  - sort attributes;
  - collapse whitespace in text, except in `pre` and `textarea`;
  - drop whitespace-only text at block and `<br>` boundaries;
  - make U+00A0 and U+3000 visible as markers, since they matter for CJK typography.

  Masks:
  - ftml: `wj-id-[A-Za-z0-9]{16}`, plus `(wiki-tabview|wj-button|wj-embed-video|wj-gallery|wj-social)-[0-9a-f]{32}` (the same set as `examples/corpus_profile.rs`).
  - live: `wiki-tabview-*` ids, `prw\d+` rate-widget classes, avatar `timestamp=` parameters, and `local--html/<page>/<hash>-<n>` iframe URLs.

## Groundwork observations (not yet divergence findings)

These come from probing a single page, `scp-040-jp` (page id 24180161, fetched 2026-11-05). They describe live Wikidot behavior only. None has been compared with ftml output yet, so none is a confirmed ftml divergence.

1. **Footnote footer title is English on the JP site.** Live `scp-040-jp` renders `<div class="footnotes-footer"><div class="title">Footnotes</div>`, even though `WIKIREQUEST.info.lang = "ja-corrections"`. Fixed strings like this may therefore match the harness's `en` default. Collapsible defaults and TOC titles still need checking per site.
2. **Unset include variables stay literal.** The live page contains `<div class="creditRate creditModule {$mode}">`. The `credit:start` component references `{$mode}`, the page does not pass it, and Wikidot leaves the token verbatim in a class attribute. ftml's `replace_variables` also substitutes only known names, so this should match; it is a good first check once rendering runs.
3. **The `[[*user]]` avatar URL is double-escaped live.** Live has `avatar.php?userid=1969312&amp;amp;size=small&amp;amp;timestamp=…`. The user id and timestamp are caller-runtime. The double escape is a Wikidot output quirk to classify, not mask, if ftml's user rendering differs.
4. **Heavy-formatting signals in the top-rated pool.** The source ListPages size `%%size%%` counts characters, not bytes. The largest top-rated pages are `scp-4000-jp` (71k chars), `scp-3000-jp` (63k), and `nostromo-last-game` (91k). These are the expected heavy-formatting stress pages once selection runs.

## Layout of `sweeps/scp-jp/`

```
scripts/wd.py          HTTP helpers, ViewSource decoder, page-content extraction, PagePreview call
scripts/fetch.py       candidates | capture | select | materialize
scripts/render.py      include expansion + harness rendering per page
tools/include_expand/  standalone crate: looped ftml::include over prefetched sources
data/candidates.json   rating-ordered candidate list with ListPages provenance
```

### Reproduce

```sh
cd sweeps/scp-jp
export SWEEP_CACHE=/path/to/scratch/cache          # raw full-page responses, not committed
python3 -I scripts/fetch.py candidates
python3 -I scripts/fetch.py capture
python3 -I scripts/fetch.py select
python3 -I scripts/fetch.py materialize
(cd tools/include_expand && CARGO_TARGET_DIR=/path/to/scratch/target-include-expand \
   CARGO_INCREMENTAL=0 cargo build --release)
(cd ../.. && cargo build --example render_html_jsonl)
python3 -I scripts/render.py --harness ../../target/debug/examples/render_html_jsonl \
  --expander /path/to/scratch/target-include-expand/release/scp-jp-include-expand
```
