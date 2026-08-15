[<< Return to the README](../README.md)

## Wikidot Parity Evidence

Raw, provenance-backed Wikidot reference JSONL is the independent oracle. A tree-test `wikidot.html` file is derived FTML output for a local `Layout::Wikidot` regression; it must never be generated and then cited as evidence for itself.

The machine-readable stable corpus is `tests/fixtures/wikidot-parity/cases.jsonl`. It inventories every `test/**/input.ftml` and `tests/fixtures/**/*.ftml` source, its hash and origin, and one execution class:

- `saved-page-batch` and `page-preview-isolated` are context-free PagePreview cases that the FTML comparator can run.
- `wikijump-runtime` cases need site, page, actor, permission, query, import, file, or browser state and remain owned by Wikijump or another caller.
- `not-applicable` cases remain accounted for but are outside both PagePreview classes.

The dated `tests/fixtures/wikidot-parity/references-YYYYMMDD-NN.jsonl` files preserve the raw Wikidot responses and capture provenance. `tests/fixtures/wikidot-parity/bindings.json` binds each preview-compatible case to its reference hashes and comparator result. These files replace the old manual fixture table as the parity authority.

The current stable inventory has 177 cases: 149 PagePreview-compatible cases, 27 caller-runtime cases, and 1 not-applicable case. The runtime and not-applicable rows are accounted for with explicit reasons and are not sent to the anonymous PagePreview capture lane.

Each mismatch has one disposition: `intentional-security-divergence` means a frozen output difference that FTML deliberately preserves to enforce a security property; `caller-runtime` means a frozen output difference whose missing inputs belong to the caller runtime rather than FTML; `comparison-normalization` means a frozen raw-DOM difference with no demonstrated functional behavior difference; and `unresolved` remains allowed only for an active functional investigation. A classified mismatch remains visible as `status: "mismatch"` and does not become a match.

Ordinary FTML tests are offline. They validate inventory completeness, reference provenance and hashes, binding completeness, stable FTML output hashes, and local snapshot admission:

```sh
cargo test --test integration parity_index
```

Exact local FTML output hashing is skipped only for the bibliography, `embedvideo`, and `gallery` cases because production rendering generates random IDs. Their frozen Wikidot hashes, source hashes, binding status, and functional DOM and text checks remain enforced. The bibliography comparator canonicalizes only its generated suffix; the two caller-runtime mismatches remain mismatches and receive no snapshot.

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
  --cases-output "$PARITY_TMP/cases.jsonl" \
  --pages-output "$PARITY_TMP/pages.jsonl" \
  --slug-prefix ftml-parity
diff -u tests/fixtures/wikidot-parity/cases.jsonl "$PARITY_TMP/cases.jsonl"
```

Review every inventory and classification change. If it is intentional, update `tests/fixtures/wikidot-parity/cases.jsonl` from the generated file. `pages.jsonl` is only a required planning output from the adjacent generator; it is not FTML evidence.

### 2. Capture raw Wikidot references

Split the preview-compatible inventory into fresh JSONL batches of at most 16 cases and verify the current 149-row selection:

```sh
mkdir "$PARITY_TMP/batches"
jq -c 'select(.execution_class == "saved-page-batch" or .execution_class == "page-preview-isolated")' \
  tests/fixtures/wikidot-parity/cases.jsonl | \
  split -l 16 -d -a 2 --additional-suffix=.jsonl - "$PARITY_TMP/batches/cases-"
test "$(wc -l "$PARITY_TMP"/batches/cases-*.jsonl | tail -n 1 | awk '{print $1}')" -eq 149
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

### 6. Promote matched cases

For a new case, add `test/<group>/<case>/wikidot.html` only when its current binding has `status: "match"`. Create the empty file, run the existing FTML update path, and inspect every generated byte:

```sh
touch test/<group>/<case>/wikidot.html
FTML_UPDATE_TESTS=1 cargo test --lib test::ast::ast -- --exact --nocapture
git diff --no-index -- /dev/null test/<group>/<case>/wikidot.html || true
cargo test --test integration parity_index
```

Update mode deliberately fails after writing. It generates a derived local snapshot; by itself it is never evidence. Existing snapshots with recorded mismatch bindings remain visible historical regressions, but a new or changed snapshot must not be promoted from a mismatch.
