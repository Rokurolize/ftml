[<< Return to the README](../README.md)

## Wikidot Parity Evidence

Raw, provenance-backed Wikidot reference JSONL is the independent oracle. A tree-test `wikidot.html` file is derived FTML output for a local `Layout::Wikidot` regression; it must never be generated and then cited as evidence for itself.

The machine-readable stable corpus is `tests/fixtures/wikidot-parity/cases.jsonl`. It inventories every `test/**/input.ftml` and `tests/fixtures/**/*.ftml` source, its hash and origin, and one execution class:

- `saved-page-batch` and `page-preview-isolated` are context-free PagePreview cases that the FTML comparator can run.
- `wikijump-runtime` cases need site, page, actor, permission, query, import, file, or browser state and remain owned by Wikijump or another caller.
- `not-applicable` cases remain accounted for but are outside both PagePreview classes.

The dated `tests/fixtures/wikidot-parity/references-YYYYMMDD-NN.jsonl` files preserve the raw Wikidot responses and capture provenance. `tests/fixtures/wikidot-parity/bindings.json` binds each preview-compatible case to its reference hashes and comparator result. These files replace the old manual fixture table as the parity authority.

The current stable inventory has 500 cases: 196 saved-page-batch cases, 272 page-preview-isolated cases, 31 caller-runtime cases, and 1 not-applicable case. The 468 PagePreview-compatible cases have 468 bindings: 450 matches and 18 classified mismatches. The runtime and not-applicable rows are accounted for with explicit reasons and are not sent to the anonymous PagePreview capture lane.

Three reviewed lexical fixtures override the adjacent inventory generator's conservative runtime regexes: a plain email address uses deterministic Wikidot email obfuscation, a `mailto:` autolink is context-free, and a triple link with an external URL needs no page resolver. `tests/fixtures/wikidot-parity/classification-overrides.json` owns these decisions by exact fixture path and source hash. A source change invalidates its override instead of weakening the default runtime classification. Their exact inventory rows use the `reviewed-context-free-email-obfuscation`, `reviewed-context-free-mailto`, and `reviewed-context-free-external-triple-link` reasons and run as isolated PagePreview cases.

Each mismatch has one disposition: `intentional-security-divergence` means a frozen output difference that FTML deliberately preserves to enforce a security property; `caller-runtime` means a frozen output difference whose missing inputs belong to the caller runtime rather than FTML; `comparison-normalization` means a frozen raw-DOM difference with no demonstrated functional behavior difference; and `unresolved` remains allowed only for an active functional investigation. A classified mismatch remains visible as `status: "mismatch"` and does not become a match.

Ordinary FTML tests are offline. They validate inventory completeness, reference provenance and hashes, binding completeness, stable FTML output hashes, and local snapshot admission:

```sh
cargo test --test integration parity_index
```

Exact local FTML output hashing is skipped for sources containing `bibliography`, `embedvideo`, or `gallery` blocks because production rendering can generate random IDs. Their frozen Wikidot hashes, source hashes, binding status, and functional DOM and text checks remain enforced. The bibliography comparator canonicalizes only its generated suffix; caller-runtime mismatches remain mismatches and receive no snapshot.

## Discovery Backlog

The stable inventory is the local merge authority. To discover additional sources exercised only by Rust tests, record one full run and build a candidate inventory:

```sh
DISCOVERY_TMP=$(mktemp -d)
FTML_SOURCE_RECORD_PATH="$DISCOVERY_TMP/sources.jsonl" \
  cargo test --features test-source-recorder
node ../wikijump/install/local/wikidot-verification/scripts/build-ftml-recorded-live-pages.mjs \
  --records "$DISCOVERY_TMP/sources.jsonl" \
  --cases-output "$DISCOVERY_TMP/cases.jsonl" \
  --pages-output "$DISCOVERY_TMP/pages.jsonl" \
  --slug-prefix ftml-parity-discovery
```

This output is a discovery backlog, not evidence or a stable manifest. It includes intermediate stages and sources from generated tests, so runs can differ. Inspect each record origin and promote a useful source by adding or reusing a stable fixture before capture.

## Regenerate, Capture, Compare, Bind, Report, and Promote

Run capture and comparison from the FTML repository with the adjacent Wikijump repository at `../wikijump`. Use a fresh temporary directory because the external tools create outputs without replacement.

### 1. Regenerate the inventory

```sh
PARITY_TMP=$(mktemp -d)
node ../wikijump/install/local/wikidot-verification/scripts/build-ftml-live-pages.mjs \
  --ftml-root . \
  --classification-overrides tests/fixtures/wikidot-parity/classification-overrides.json \
  --cases-output "$PARITY_TMP/cases.jsonl" \
  --pages-output "$PARITY_TMP/pages.jsonl" \
  --slug-prefix ftml-parity
diff -u tests/fixtures/wikidot-parity/cases.jsonl "$PARITY_TMP/cases.jsonl"
```

Review every inventory and classification change. If it is intentional, update `tests/fixtures/wikidot-parity/cases.jsonl` from the generated file. `pages.jsonl` is only a required planning output from the adjacent generator; it is not FTML evidence.

### 2. Capture raw Wikidot references

Split the preview-compatible inventory into fresh JSONL batches of at most 16 cases and verify the current 468-row selection:

```sh
mkdir "$PARITY_TMP/batches"
jq -c 'select(.execution_class == "saved-page-batch" or .execution_class == "page-preview-isolated")' \
  tests/fixtures/wikidot-parity/cases.jsonl | \
  split -l 16 -d -a 2 --additional-suffix=.jsonl - "$PARITY_TMP/batches/cases-"
test "$(wc -l "$PARITY_TMP"/batches/cases-*.jsonl | tail -n 1 | awk '{print $1}')" -eq 468
```

For each batch, choose a new dated no-replace output name and run:

```sh
BATCH="$PARITY_TMP/batches/cases-00.jsonl"
REFERENCE="tests/fixtures/wikidot-parity/references-$(date -u +%Y%m%d)-01.jsonl"
../wikijump/install/local/wikidot-verification/.venv/bin/python \
  ../wikijump/install/local/wikidot-verification/scripts/capture_wikidot_preview_references.py \
  --cases "$BATCH" \
  --output "$REFERENCE" \
  --site sandbox-for-codex \
  --batch-size 4 \
  --live-execution-class saved-page-batch \
  --live-execution-class page-preview-isolated
```

Change `BATCH` and the `REFERENCE` numeric suffix for each run. Both `--live-execution-class` options are required; otherwise the capture command defaults to only `page-preview-isolated`. The command is anonymous, read-only, and external to normal FTML tests. Never replace or edit a raw reference file to make a comparison pass. If a fixture source changes, keep its old row and add a new capture; the offline tools validate the old row as history and select the one whose source hash is current.

### 3. Compare FTML with each reference batch

Build the streaming renderer once, then create one no-replace verdict per newly captured current-source reference batch:

```sh
cargo build --example render_html_jsonl
node ../wikijump/install/local/wikidot-verification/scripts/run-syntax-differential.mjs \
  --references "tests/fixtures/wikidot-parity/references-YYYYMMDD-NN.jsonl" \
  --renderer target/debug/examples/render_html_jsonl \
  --output "$PARITY_TMP/verdict-YYYYMMDD-NN.json"
```

Repeat the comparator for every new reference file whose rows match the current fixture source hashes. It compares parsed DOM, DOM signature, and visible text, retains raw HTML diagnostics for mismatches, and exits nonzero for a mismatch or runner error. Keep historical reference files as `--references` inputs in later steps, but do not pass verdicts for their obsolete source rows.

### 4. Bind the complete result

Pass every dated reference file and the corresponding verdict for every current-source reference, including mismatches, with repeated options. Do not pass verdicts for historical source rows. Write to a temporary file first:

```sh
python3 scripts/wikidot_parity.py bind \
  --cases tests/fixtures/wikidot-parity/cases.jsonl \
  --references tests/fixtures/wikidot-parity/references-YYYYMMDD-01.jsonl \
  --references tests/fixtures/wikidot-parity/references-YYYYMMDD-02.jsonl \
  --verdict "$PARITY_TMP/verdict-YYYYMMDD-01.json" \
  --verdict "$PARITY_TMP/verdict-YYYYMMDD-02.json" \
  > "$PARITY_TMP/bindings.json"
```

Add all remaining repeated arguments in the same form. The command rejects missing, duplicate, extra, hash-invalid, or tier-incompatible evidence. Review the result before updating `tests/fixtures/wikidot-parity/bindings.json`.

Bind output is transient: `make_bindings` may emit `disposition: "unresolved"` with `reason: "Unreviewed mismatch."`. The discoverer must create an issue immediately and replace that reason with `Active functional investigation: issue #<positive-number>.` before committing `bindings.json`; committed bindings reject the placeholder.

### 5. Report the corpus

```sh
python3 scripts/wikidot_parity.py report \
  --cases tests/fixtures/wikidot-parity/cases.jsonl \
  --references tests/fixtures/wikidot-parity/references-YYYYMMDD-01.jsonl \
  --references tests/fixtures/wikidot-parity/references-YYYYMMDD-02.jsonl \
  --bindings tests/fixtures/wikidot-parity/bindings.json \
  --repo .
```

Add every dated reference file as a repeated `--references` argument. The report names all mismatches, caller-owned runtime cases, not-applicable cases, missing references, missing bindings, and matched cases that still need a local snapshot. A nonzero mismatch count is an explicit result, not permission to hide or normalize the difference.

### Live note evidence

On 2026-08-15, anonymous `edit/PagePreviewModule` on `sandbox-for-codex.wikidot.com` was queried with `[[note class="custom"]]\nLive note probe.\n[[/note]]`. Wikidot returned literal markup:

```html
<p>[[note class=&quot;custom&quot;]]<br />
Live note probe.<br />
[[/note]]</p>
```

The response SHA-256 was `363e4f6b6139f479eb0b1fc80b0ccd4e8a2bafbbc681b77e912f94a9ea8c6d98`. The focused `test/note/wikidot-literal` case now compares this response's DOM tree, DOM signature, and visible text successfully. FTML therefore keeps the Wikijump-only `wj-note` feature while leaving the marker literal in `Layout::Wikidot`; issue #500 is resolved by this change.

### Live module alias evidence

On 2026-08-15, reference batches `references-20260815-11.jsonl` and `references-20260815-12.jsonl` captured seven anonymous `edit/PagePreviewModule` observations on `sandbox-for-codex.wikidot.com`. The Wikidot module closer rule refers to the observed behavior in which `module654` may open a module body but only `module` may close it. Wikidot rendered `[[/module654]]` and the remaining CSS body as page text for both `[[module CSS]]` and `[[module654 CSS]]`, while `[[module654 CSS]]` with `[[/module]]` consumed the CSS body normally.

Before the fix, the two alias-closer cases were confirmed parity mismatches across DOM tree, DOM signature, and visible text. Issue #503 records the FTML behavior defect. After the shared close-name fix, all seven observations in the two batches match across all three checks. Their exact source hashes, raw response hashes, timestamps, anonymous acquisition fields, and `wikidot.py` provenance remain frozen in the reference files.

### Registered block-name evidence

Registered block-name coverage refers to the state in which every configured block name and alias appears in the stable parity inventory. Reference batch `references-20260815-13.jsonl` closes the final three name gaps with `button`, `embedvideo`, and `gallery` sources. The invalid `[[button]]` case matches Wikidot across DOM tree, DOM signature, and visible text.

The other two rows are caller-runtime mismatches and remain visible as mismatches. Wikidot returns a provider no-match error for the inert `embedvideo` payload, while FTML emits a typed placeholder for caller resolution; issue #506 tracks that result. Wikidot gallery selection uses current-page and file-service state and returns a selection error for the empty case, while FTML emits a typed gallery request; issue #507 tracks that result. These rows are not normalized into matches.

Configured block-facet coverage refers to the state in which every configured star, score, and body-close branch appears in the stable parity inventory. Reference batch `references-20260815-14.jsonl` closes the four previously missing star branches for `anchor`, `checkbox`, `radio`, and `user`. Live Wikidot and `Layout::Wikidot` both keep all four starred forms literal, so every DOM tree, DOM signature, and visible text check matches. The parity report now fails locally if a configured name, star branch, score branch, or body-close branch lacks a stable source.

Ordered alias-pair coverage refers to the state in which a configured opener and closer name pair for a body-owning block appears together in a stable source. Batch `references-20260815-15.jsonl` added the two anchor cross-pairs and exposed issue #510. Batches `references-20260815-16.jsonl` through `references-20260815-18.jsonl` captured the remaining 48 pairs. All 48 match across DOM tree, DOM signature, and visible text, so the local report now lists no missing ordered alias pair.

Reference batch `references-20260815-19.jsonl` freezes the 12 context-free argument-grammar sources asserted by `tests/wikidot_block_argument_grammar.rs` that previously lacked per-source evidence. All 12 match across DOM tree, DOM signature, and visible text. The remaining whitespace-name `user` source is inventoried as caller runtime because reproducing its missing-user result requires the caller-owned user resolver used by that test.

Reference batch `references-20260815-20.jsonl` tests column-zero, space-prefixed, tab-prefixed, and text-prefixed ownership for `bibliography`, `code`, `gallery`, `math`, `module CSS`, and `toc`. It exposed issues #514 and #515: FTML accepted whitespace-prefixed physical-line blocks and joined a rejected gallery closer to the next line. After both fixes, five cases match all three comparison surfaces. Gallery line ownership and visible text also match; its remaining DOM mismatch is the caller-owned page/file rendering tracked by issue #507.

Configured head-shape coverage refers to the state in which every head shape required by a block's effective parser appears in the stable parity inventory. The local ratchet requires empty and non-assignment value tails for no-head and value-head blocks; empty, assignment-map, and non-assignment value tails for map-head blocks; and empty, value, map, and value-plus-map tails for value-plus-map blocks. It applies the effective `html` map, `tabview` value, and `raw` custom-head overrides. The parity report now lists `missing-head-shape 0`.

Reference batches `references-20260815-21.jsonl` through `references-20260815-24.jsonl` freeze 61 anonymous PagePreview head-shape observations; the remaining `file:none` and `module:map` sources are inventoried as caller runtime. The live comparisons exposed three FTML defects. Issue #517 covered standalone buttons missing Wikidot click handlers and incorrectly receiving a generated `id`; the fixed Wikidot output emits the live handlers without that ID. Issue #518 covered the localized host default for `[[date 0]]`; the fixed Wikidot default is the deterministic `01 Jan 1970 00:00`. Issue #519 covered a non-assignment gallery tail remaining literal instead of creating a gallery requirement. After the fixes, 59 of these 61 PagePreview cases match all three comparison surfaces. The `gallery:map` and `gallery:value` cases reach the correct gallery requirement but remain visible caller-runtime mismatches because current-page and file-service selection belongs to issue #507.

Reference batch `references-20260815-25.jsonl` adds the security-sensitive invalid apostrophe tag token for `set-tags`. Issue #520 recorded that Wikidot rejects the token with its missing-text error while FTML previously emitted a functional button. FTML now rejects the token while retaining JavaScript-string escaping as defense in depth, and the frozen case matches all three comparison surfaces.

Reference batches `references-20260815-26.jsonl` and `references-20260815-27.jsonl` freeze 25 button and date sources that were either missing live evidence for local regressions or added for the token boundaries in issue #521. The fixed Wikidot parser rejects quote, equals, ampersand, entity, and angle-bracket artifacts; treats ASCII tab, carriage return, line feed, vertical tab, and form feed as separators; removes NUL; and retains the observed broad punctuation and Unicode acceptance. Twenty-four cases match all three comparison surfaces. The live-accepted backslash case remains one visible `intentional-security-divergence`: FTML JavaScript-string escapes the backslash so authored tag data cannot change handler execution, while DOM signature and visible text still match.

Batch `references-20260815-27.jsonl` also exposed issue #522. For `[[date 0 format="%Y"]]`, Wikidot records `format_%25Y` in the class but renders the default `01 Jan 1970 00:00` text rather than `1970`. `Layout::Wikidot` now matches that behavior, while `Layout::Wikijump` retains explicit-format rendering.

Reference batches `references-20260815-28.jsonl` through `references-20260815-32.jsonl` freeze 76 anonymous PagePreview observations from a 77-source interaction set. The sources are grouped by shared parser branch rather than a Cartesian product: 8 lexical and malformed-block recovery cases, 12 quote-ownership cases, 9 native-list-ownership cases, 16 inline-delimiter cases, 16 math-boundary cases, and 16 table-ownership cases. The remaining `interaction-triple-link-span-close` source is inventoried as caller runtime, so all 77 remain accounted for.

These batches exposed three FTML defects. Issue #527 removed the functional trailing line break after a code block at the end of a footnote; its remaining root formatting whitespace differs only in the raw DOM, while DOM signature and visible text match, so the binding is `comparison-normalization`. Issue #528 made a malformed scored span opener with a residual `]` use Wikidot's root ownership. Issue #529 made a residual gallery opener bracket and the following line share Wikidot's paragraph ownership. The embedvideo residual-opener row remains a `caller-runtime` mismatch under issue #506 because provider matching belongs to the caller, and the gallery residual-opener row remains a `caller-runtime` mismatch under issue #507 because current-page file selection belongs to the caller.

Reference batch `references-20260815-33.jsonl` freezes 16 anonymous, non-mutating PagePreview observations for top-level parser functions and preprocessor ownership seams. Ten sources cover `#if`, `#ifexpr`, and `#expr` truth, error, case, newline, and residual-close branches. Six sources cover quote removal, crossed center/collapsible ownership, crossed center/div ownership, inline bold/size correction, and orphan quoted tab closers. The crossed quoted center/collapsible source exposed issue #534: FTML incorrectly kept text after the collapsible close inside the unfolded body. The fixed `Layout::Wikidot` renderer matches all 16 sources across DOM tree, DOM signature, and visible text; `Layout::Wikijump` keeps its existing crossed-closer behavior.

Reference batch `references-20260815-35.jsonl` freezes 13 anonymous PagePreview branch witnesses selected from the remaining interaction backlog. Two crossed Color/Container directions exercise both shared-span shell roles. Three inline triples, together with the existing no-equality triple, exercise each FIFO tag-equality relation. Four quote cases exercise depth decrease, blank and empty quoted-row ownership, and virtual heading start. Four list cases exercise indented-first rejection, depth one, a skipped depth, and tab expansion to depth four. All 13 match Wikidot across DOM tree, DOM signature, and visible text. These witnesses close the relevant parser branches; equivalent ordered combinations do not need Cartesian capture.

Reference batch `references-20260815-34.jsonl` freezes 16 anonymous, non-mutating PagePreview observations for autolink termination and ownership seams: punctuation, whitespace, Unicode, email, `mailto:`, protocol-relative text, external triple links, code, raw text, comments, and tables. The current `Layout::Wikidot` renderer at engine commit `bd1e6bfb6859011a62b2cf2173c0764b279abf00` matches all 16 across DOM tree, DOM signature, and visible text, so this batch exposed no functional discrepancy.

Reference batch `references-20260815-36.jsonl` freezes 10 anonymous, non-mutating PagePreview observations for advanced-table closer selection, residual closers, casing, malformed names, and heading, quote, and list ownership. Nine cases match Wikidot across DOM tree, DOM signature, and visible text and have reviewed local snapshots. In `table-residual-hcell-closer`, PagePreview's standard leading root `\n\n` is merged into the residual text node; DOM signature and visible text match. Issue #537 reviewed this as `comparison-normalization`, not FTML behavior, so that row remains a visible classified mismatch and has no snapshot.

Reference batches `references-20260815-37.jsonl` and `references-20260815-38.jsonl` freeze 17 anonymous, non-mutating PagePreview observations for the remaining context-free automatic-link and single-link recovery branches. The witnesses cover paired wrapper removal, terminal-period splitting, bare `www.` rejection, single-link target and label comment elision, empty and invalid target dispositions, empty-label and end-of-input recovery, residual closers, raw, span, footnote, and bibliography ownership, and case-sensitive and security-sensitive scheme handling. All 17 match Wikidot across DOM tree, DOM signature, and visible text on FTML engine commit `81d60e5db932b83c5101442e4534bf5ef13212f7`.

Reference batch `references-20260815-39.jsonl` freezes 16 anonymous, non-mutating PagePreview observations for heading content and recovery. The witnesses cover empty TOC and no-TOC headings, comment, raw, inline-format, span, Color, footnote-first and next-line footnote ownership, adjacent heading retries, nested line ownership, and the trailing-underscore guard. All 16 match Wikidot across DOM tree, DOM signature, and visible text on FTML engine commit `926303b2931c4420c1ed3557399f74cc4375ef67`.

### 6. Promote matched cases

For a new case, add `test/<group>/<case>/wikidot.html` only when its current binding has `status: "match"`. Create the empty file, run the existing FTML update path, and inspect every generated byte:

```sh
touch test/<group>/<case>/wikidot.html
FTML_UPDATE_TESTS=1 cargo test --lib test::ast::ast -- --exact --nocapture
git diff --no-index -- /dev/null test/<group>/<case>/wikidot.html || true
cargo test --test integration parity_index
```

Update mode deliberately fails after writing. It generates a derived local snapshot; by itself it is never evidence. Existing snapshots with recorded mismatch bindings remain visible historical regressions, but a new or changed snapshot must not be promoted from a mismatch.
