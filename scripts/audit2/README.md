# audit2: corpus fixture provenance audit (pass 2)

These are the harnesses used for the full corpus audit recorded in
`REPORT-audit2.md` and `audit/`. They are external to the offline test suite;
their caches and captures are audit evidence, not frozen reference batches.

All scripts take the audit record directory from `FTML_AUDIT_RECORD` (default
`<repo>/.audit`) and write repo-relative artifacts.

| script | purpose |
|---|---|
| `probe_preview.py` | Post ad-hoc shapes to `edit/PagePreviewModule` twice, record both raw bodies and hashes. |
| `saved_probe.py` | Create/update run-owned `muse-probe-audit2-N` pages on `sandbox-for-codex`, capture `#page-content` twice, compare with the fixture. Needs the sandbox account store and `wikidot.py`. |
| `classify_audit.py` | Merge the verdict JSONL files, compare structure and visible text, relax caller-runtime output, prefer saved-page evidence, and emit `classified.json` plus the non-match evidence bundle. |
| `whitespace_audit.py` | Adjudicate whitespace-only mismatches against live bytes and emit `whitespace-analysis.json`. |
| `tabview_classify.py` | Minimal-difference resolution of the tabview structural divergences (`tabview-class.json`). |
| `package_evidence.py` | Copy the working evidence into the committed `audit/` bundle. |
| `build_report.py` | Render `REPORT-audit2.md` from the classified evidence. |

The primary PagePreview harness is `scripts/audit_corpus_live.py` (pass 1,
unchanged on this branch). Reproduce with a record directory holding
`verdicts-*.jsonl`, `preview-cache/`, `saved-pages/`, `probes/`, and
`b_items.json`; see the "Reproduction" section of `REPORT-audit2.md`.
