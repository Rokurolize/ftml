# ftml real-page divergence sweep: REPORT

Discovery only. No library source was changed. Sweep scripts, raw diffs, and evidence live beside this file.

## STATUS

**Done**
- Fetched 40 public scp-wiki articles that are not used anywhere in the repo. Both the source (view-source route) and the live `#page-content` HTML were fetched. The page list is in `data/`, and the excluded repo slugs are in `excluded-repo-slugs.txt`.
- Expanded `[[include]]` with ftml's own `ftml::include` through the sweep-side crate `expander/`. The crate fetches the component sources from the live wiki (`components/`, 30+ pages). The stock harness has no Includer, so expansion has to happen first.
- Rendered all 40 pages with the repo harness (`examples/render_html_jsonl`, Wikidot layout) and diffed them against live after normalization.
- **Result:** 275 hunks, and no page is hunk-free. Per-page diffs are in `out/diffs/`. All hunks are in `out/hunks.json`, per-page counts in `out/summary.json`, and per-family page lists in `out/family-pages.json`.
- Clustered the hunks into the families below, with evidence in `evidence/family-evidence.txt` and minimized repros in `repros/`.

**Partial / caveats**
- Six cross-site includes (e.g. `:scpsandbox2:clef`) were not resolved, so the pages that use them are inconclusive. They are listed in `unresolved-includes.json`.
- The sample is 40 pages (target was "40–60"), skewed toward articles numbered ≤3000. Fetches that returned 503 were skipped (`fetch-failures.json`).
- Minimized repros show the **local** output. They were **not** confirmed on live, because anonymous PagePreview is not available here. The live side of each family is the real page's bytes, not a minimized sample. Per `CLAUDE.md`, F1 to F3 still need a saved-page live probe before they count as pinned divergences.
- I did not check the families against `tests/fixtures/wikidot-parity` captures or run the repo's test suite.
- `normalize.py` masks `prw` ids, `time_<ts>` classes, unix timestamps and long hex ids. After normalization no masked-id differences remained in the diffs I reviewed, but I did not audit every hunk.

**Remaining**
- Live-probe F1 and F2 on a saved page, then pin them as corpus fixtures if confirmed.
- Resolve F3 by diffing `ftml::include` output against Wikidot's expansion for `author-label-source` and `iftags` components.
- Triage the ~20 low-count one-off hunks in `out/hunks.json`.
- Extend to 60 or more pages.

## Families

Counts are hunks / pages, taken from `out/hunks.json` and `out/family-pages.json`.

### F1. `[Word: text]` loses its brackets and the prefix (candidate real divergence)
- **Page:** scp-1994 (source `however [Note: this must be addressed before testing].`).
- **Live:** `…however [Note: this must be addressed before testing]. Unusually…` (literal).
- **ftml:** `…however this must be addressed before testing. Unusually…`.
- **Minimized:** `a [Note: x] b` → `<p>a x b</p>`. `a [Note:x] b` and `a [foo bar] b` stay literal, so the trigger looks like "word + colon + space inside single brackets".
- **Existing pins:** `test/link/single-wikidot-schemes` covers only `[scheme://…]`, so no existing pin covers this shape.
- **Evidence:** `evidence/family-evidence.txt` (F1).

### F2. `ftml::include` keeps trailing whitespace in argument values (candidate real divergence, library include path)
- **F2a, scp-1994:** component text is `[{$authorPage} ▸ More by this Author ◂]` and the argument is `|authorPage=http://www.scp-wiki.net/djkaktus[!-- … --]` on its own line.
  - **Live:** `<a href="http://www.scp-wiki.net/djkaktus">▸ More by this Author ◂</a>`.
  - **ftml:** `<p>[<a href="http://www.scp-wiki.net/djkaktus">http://www.scp-wiki.net/djkaktus</a></p><p>▸ More by this Author ◂]</p>`.
  - **Cause:** the expanded text is `[http://…djkaktus[!-- --]\n ▸ …◂]`, so the value carried its newline.
- **Minimized:** component `[{$u} label]` with `[[include component:t\n|u=http://x.net/d\n]]` expands to `[http://x.net/d\n label]` (`repros/pages.json`). With blank-line-separated arguments, `|a= EN\n\n|b=x\n\n` gives `EN\n\n` and `x\n\n` (leading space trimmed, trailing whitespace kept).
- **F2b, scp-1706:** `[[table class="scale [[#if EN\n\n | EN\n\n-base | ]]"]]`.
  - **Live:** `class="scale EN-base"`.
  - **ftml:** `class="scale EN -base"`.
  - **Also seen on this page, same component family (cause not isolated):** `CLASSIFIED` (live) vs `false` (ftml), and `2/1706` vs `2` + `/1706`.
- **Caveat:** `src/includes/parse.rs` trims trailing blanks in some places, so this may be a deliberate or pinned case. It needs adjudication against `test/include/*` and a live probe before it is called a divergence.

### F3. Leftover literals after include expansion (inconclusive)
- **scp-102:** ftml outputs `<p>[[/div]]<br>[[/div]]<br>[[/div]]</p>`, where live closes the three divs. The source is the `author-label-source` two-part include (`start=--` … `end=--]]`).
- **scp-039, scp-1305:** a literal `[[iftags -component-backend]]` / `[[/iftags]]` paragraph. scp-1305 expands to 19,404 lines, so it is likely an expansion artifact.
- **Disposition:** not attributed to the parser. Comment handling of `[!-- … --]` reproduces correctly in isolation (repro tested). Treat as an include-expansion or variable-substitution mismatch to investigate.

### F4. `<img>` attribute order and file host (cosmetic or caller)
- **Hunks / pages:** 24 hunks (17 plain `img` + 7 linked `img`) across the 17 pages in `out/family-pages.json`; the 16 avatar `img.small` hunks are counted under F5 (user links).
- **Live (scp-201):** `<img src="https://scp-wiki.wdfiles.com/local--files/scp-201/005.jpg" alt="005.jpg" class="image" />`.
- **ftml:** `<img src="https://scp-wiki.wjfiles.com/local--files/scp-201/005.jpg" class="image" alt="005.jpg">`.
- **Differences:** attribute order (`alt`,`class` vs `class`,`alt`) and file host (`wdfiles.com` vs `wjfiles.com`).
- **Disposition:** attribute order matches the repo's `comparison-normalization`. The file host is a file-service concern that belongs to the caller.

### F5. Caller-runtime, not FTML divergences
- **Rate module:** 39 pages. Live renders the widget, ftml emits `TODO: module Rate`.
- **ListPages / list-pages-box / backlinks:** 37 pages (e.g. license-box ListPages). ftml emits `TODO: module ListPages`.
- **`[[user]]` / `[[*user]]`:** 11 pages. Live uses `https://www.wikidot.com/user:info/<slug>`, real user ids, and display-name case. ftml uses `/user:info/<slug>`, `userInfo(0)`, and lowercased text.
- **Unresolved cross-site includes and `styleFrame` iframes:** 7 pages, including `error-block` where an include was unresolved.
- These need site, user, or page-database context and match the repo's `caller-runtime` disposition.

## Files
- `candidates.py`, `fetch.py`, `select_and_fetch.py`: selection and fetch. `pipeline.py`: fetch plus expansion. `expander/`: include expander. `render.py`: harness run. `normalize.py`, `diff_cluster.py`: diff and cluster. `repro.py`: single-case renderer.
- `data/<slug>/`: `source.ftml`, `expanded.ftml`, `live.html`, `ftml.html`, `*.norm`, `meta.json`.
- `out/`: `diffs/*.diff`, `hunks.json`, `summary.json`, `family-pages.json`. `evidence/`, `repros/`.

## Reproduce
```sh
cargo build --release --example render_html_jsonl
(cd divergence-sweep/expander && CARGO_TARGET_DIR=../../target cargo build --release)
cd divergence-sweep
python3 candidates.py
python3 pipeline.py "$PWD/data" "$PWD/components" 40
python3 render.py "$PWD/data" ../target/release/examples/render_html_jsonl
mkdir -p out && python3 diff_cluster.py "$PWD/data" "$PWD/out"
```
