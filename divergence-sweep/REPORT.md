# ftml real-page divergence sweep: REPORT (INCOMPLETE)

**This sweep did not reach the diff or clustering stage. There are no divergence families, no raw diffs, and no minimized repros in this report.** It records the tooling that was built, what was verified, and what remains. No library source was changed.

## STATUS

### Done
- **Source retrieval from the public scp-wiki, verified end to end.** Wikidot's `viewsource/ViewSourceModule` is called through `ajax-module-connector.php`, using the page id from the page HTML and a `wikidot_token7` value taken from the page's cookie. No credentials are needed. `fetch.py` decodes the response as follows:
  - `&nbsp;` becomes a space.
  - `<br />\n` becomes a newline.
  - `<a>` wrappers around include targets are stripped.
  - Other entities are unescaped.
  - Caveat: a literal tab in a source cannot be distinguished from view-source's own wrapper tab, so tab-bearing sources could decode slightly wrong. This was not checked.
- **Live HTML extraction.** `fetch.py` takes the inner HTML of `#page-content` by balancing `<div>` depth, because ad scripts follow the content and a fixed end marker was wrong. It was spot-checked on `scp-5133`.
- **Exclusion of articles already used in the repo.** `candidates.py` builds the candidate list from `scp-series` through `scp-series-7`. It excludes every slug mentioned in any tracked file (31 slugs, listed in `excluded-repo-slugs.txt`). The shuffle uses a fixed seed (20261105), and articles numbered 3000 or below come first.
- **Selection driver.** `select_and_fetch.py` runs a polite 1 s delay per request and caches results.
- **`render.py`.** It was written but **never run**. It feeds sources to `target/release/examples/render_html_jsonl` with `layout: wikidot`. The release build of that example did complete earlier in the session.
- **`normalize.py`.** It was written and smoke-tested on one small string only. Both sides tokenize to one tag or text run per line. Entity forms, attribute quoting, `<br>` forms and whitespace are canonicalized. Prefixed ids (`prw…`), `time_<ts>` classes, bare unix timestamps and long hex ids are masked.
- **No-include articles fetched**: `scp-1564` and `scp-4699` are in `data/`, with `source.ftml`, `live.html` and `meta.json`. They were neither rendered nor diffed.

### Partial
- **`expander/`** is a standalone crate with its own `[workspace]` and a path dependency on ftml (`../..`). It runs ftml's own `ftml::include` in a loop (at most 6 rounds) against fetched component sources and reports the pages it could not find. **It was not built or run.** The first build attempt used a wrong path, which I fixed, and the second attempt was interrupted. It is untested, so treat the code as a draft.

### Not done
1. Rendering any page and diffing it against live.
2. Per-page diffs.
3. Divergence families with minimized repros, and the per-family evidence (source, live bytes, ftml bytes, page).

## Finding that drove the scope change

The first plan was to keep only articles with no `[[include …]]`, because the harness has no Includer. That filter nearly emptied the sample: **about 46 articles were skipped for includes, 2 were kept, and 2 fetches failed with a 503** (the 46 skipped are in `skipped.json`; the 2 kept are `data/scp-1564` and `data/scp-4699`). Almost every article, old ones included, carries `component:image-block` and `license-box` includes, so a no-include sample is unrepresentative.

The expander was started for this reason. It would expand includes with the library's own code before the stock harness renders the text. Any include-expansion divergence would then show up in the diffs instead of being hidden.

## Expected noise (not yet confirmed)
`[[module Rate]]`, ListPages and other site-dependent modules should show up as a caller-runtime family. Under `CLAUDE.md`, caller-runtime differences are not FTML divergences.

## Remaining work, in order
1. Build the expander with `cd divergence-sweep/expander && CARGO_TARGET_DIR=../../target cargo build --release`.
2. Loop: run the expander, fetch the pages it reports missing (component sources via the same view-source route), and re-run until nothing is missing.
3. Run `select_and_fetch.py` for roughly 40–60 articles, with the include filter dropped once expansion works.
4. Render with `render.py`, normalize and diff to `diffs/<slug>.diff`, and cluster the hunks by tag-pair signature.
5. Minimize each family by ddmin over source lines. Before calling anything a divergence, check it against the existing pins and the frozen captures in `tests/fixtures/wikidot-parity`, as `CLAUDE.md` requires.

## Reproducing what exists
```sh
cd divergence-sweep
python3 candidates.py
python3 select_and_fetch.py "$PWD/data" 60
```
`data/*/page.raw.html` and `viewsource.json` are gitignored.
