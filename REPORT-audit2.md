# Corpus fixture provenance audit, pass 2 (full sweep)

Date: 2026-11-05. Branch: `fix/dsh-audit2-corpus-audit`, based on
`claude/audit-corpus-fixtures-9fa427` (`a68272762`). Evidence-only: no library
source and no fixture file is changed in this branch; fixtures are fixed by
separate lanes.

## STATUS

**Done**

- Audited **987 of 987** fixtures that carry a `wikidot.html`, across all 59
  families, against live Wikidot PagePreview with the pass-1 harness
  (`scripts/audit_corpus_live.py`). The pass-1 set (231 fixtures) was re-audited
  on the current branch; the remaining 756 are new.
- Classified every non-match (see taxonomy mapping below). All full evidence is
  committed under `audit/`: merged live captures, per-fixture verdicts,
  classification evidence, saved-page captures, whitespace adjudication, probes.
- Settled **135**
  preview-opaque shapes with saved-page captures on `sandbox-for-codex` using
  account A and tags `fruit, component`, including the pass-1 leftovers: the 7
  iftags fixtures (`color/iftags-seam-spec`, `list/iftags-body-escape*`,
  `span/iftags-seam*`) all match live exactly.
- Adjudicated all **53** whitespace-only divergences against live bytes:
  every one is normalization-only.
- Opened **B7–B24** for the newly found
  bug-encoded fixtures (continuing the B sequence; B1–B6 verdicts untouched).

**Result totals**

- audited: 987 fixtures with a `wikidot.html` (987/987 with `wikidot.html`; all 59 families).
- match: 810 (including every first-pass match re-checked)
- harmless-whitespace: 53
- unjudgeable (PagePreview cannot observe): 0
- bug-encoded: 51
- caller-runtime / boundary context: 73

Taxonomy mapping: `harmless-whitespace` = the pass-1 "stale but harmless"
class; `unjudgeable` = the pass-1 "context-only" class (PagePreview cannot
observe what the fixture runner renders); `bug-encoded` = element/attribute/
visible-text divergence. `caller-runtime` and `boundary` are the project's
frozen dispositions from `docs/ParityTests.md`; they are the "unjudgeable"
sub-cases where the missing input belongs to the caller/runtime rather than
PagePreview's gate evaluation.

## Coverage

Every fixture with a `wikidot.html` was posted to live PagePreview. Counts per
family below; `context` includes caller-runtime and boundary dispositions.

| family | fixtures | match | harmless-whitespace | context | unjudgeable | bug-encoded |
|---|---|---|---|---|---|---|
| `align` | 3 | 3 | 0 | 0 | 0 | 0 |
| `anchor` | 31 | 28 | 0 | 0 | 0 | 3 |
| `audio` | 3 | 3 | 0 | 0 | 0 | 0 |
| `bibliography` | 11 | 10 | 0 | 0 | 0 | 1 |
| `blockquote` | 27 | 26 | 0 | 0 | 0 | 1 |
| `bold` | 29 | 29 | 0 | 0 | 0 | 0 |
| `button` | 2 | 2 | 0 | 0 | 0 | 0 |
| `char` | 2 | 2 | 0 | 0 | 0 | 0 |
| `checkbox` | 2 | 2 | 0 | 0 | 0 | 0 |
| `code` | 19 | 13 | 4 | 0 | 0 | 2 |
| `collapsible` | 6 | 5 | 0 | 0 | 0 | 1 |
| `color` | 24 | 23 | 0 | 1 | 0 | 0 |
| `date` | 13 | 12 | 0 | 0 | 0 | 1 |
| `definition-list` | 12 | 12 | 0 | 0 | 0 | 0 |
| `diff` | 3 | 3 | 0 | 0 | 0 | 0 |
| `div` | 37 | 35 | 1 | 0 | 0 | 1 |
| `embed` | 1 | 1 | 0 | 0 | 0 | 0 |
| `embedvideo` | 1 | 0 | 0 | 1 | 0 | 0 |
| `file` | 3 | 0 | 0 | 3 | 0 | 0 |
| `footnote` | 54 | 43 | 4 | 1 | 0 | 6 |
| `gallery` | 21 | 0 | 0 | 21 | 0 | 0 |
| `heading` | 3 | 3 | 0 | 0 | 0 | 0 |
| `hidden` | 1 | 1 | 0 | 0 | 0 | 0 |
| `html` | 2 | 2 | 0 | 0 | 0 | 0 |
| `ifcategory` | 1 | 1 | 0 | 0 | 0 | 0 |
| `iframe` | 8 | 8 | 0 | 0 | 0 | 0 |
| `iftags` | 132 | 116 | 14 | 1 | 0 | 1 |
| `image` | 7 | 0 | 0 | 6 | 0 | 1 |
| `invisible` | 1 | 1 | 0 | 0 | 0 | 0 |
| `italics` | 11 | 11 | 0 | 0 | 0 | 0 |
| `later` | 1 | 1 | 0 | 0 | 0 | 0 |
| `line-breaks` | 3 | 3 | 0 | 0 | 0 | 0 |
| `lines` | 1 | 1 | 0 | 0 | 0 | 0 |
| `link` | 51 | 40 | 0 | 11 | 0 | 0 |
| `list` | 55 | 45 | 8 | 0 | 0 | 2 |
| `mark` | 1 | 1 | 0 | 0 | 0 | 0 |
| `math` | 9 | 7 | 0 | 0 | 0 | 2 |
| `media` | 2 | 2 | 0 | 0 | 0 | 0 |
| `misc` | 42 | 38 | 2 | 1 | 0 | 1 |
| `module` | 2 | 2 | 0 | 0 | 0 | 0 |
| `monospace` | 14 | 14 | 0 | 0 | 0 | 0 |
| `note` | 4 | 4 | 0 | 0 | 0 | 0 |
| `paragraph` | 21 | 21 | 0 | 0 | 0 | 0 |
| `radio` | 3 | 3 | 0 | 0 | 0 | 0 |
| `raw` | 16 | 16 | 0 | 0 | 0 | 0 |
| `ruby` | 3 | 3 | 0 | 0 | 0 | 0 |
| `size` | 13 | 13 | 0 | 0 | 0 | 0 |
| `span` | 54 | 52 | 1 | 0 | 0 | 1 |
| `strikethrough` | 69 | 67 | 1 | 1 | 0 | 0 |
| `subscript` | 7 | 7 | 0 | 0 | 0 | 0 |
| `superscript` | 7 | 7 | 0 | 0 | 0 | 0 |
| `table` | 50 | 30 | 18 | 0 | 0 | 2 |
| `tabview` | 57 | 13 | 0 | 19 | 0 | 25 |
| `target` | 4 | 4 | 0 | 0 | 0 | 0 |
| `toc` | 9 | 9 | 0 | 0 | 0 | 0 |
| `underline` | 5 | 5 | 0 | 0 | 0 | 0 |
| `underscore` | 3 | 3 | 0 | 0 | 0 | 0 |
| `user` | 8 | 1 | 0 | 7 | 0 | 0 |
| `video` | 3 | 3 | 0 | 0 | 0 | 0 |
| **total** | **987** | **810** | **53** | **73** | **0** | **51** |

## Method

- PagePreview harness (pass 1): `POST edit/PagePreviewModule` anonymously,
  twice-cached by source hash, html5lib DOM + visible-text comparison, generated
  ids masked, formatting whitespace at block edges dropped.
- Saved-page harness (new): `scripts/audit2/saved_probe.py` creates/updates
  run-owned `muse-probe-audit2-N` pages with the fixture source and the corpus
  runner's tags (`fruit, component`), fetches each saved page twice anonymously,
  extracts `#page-content` inner HTML, and compares it with the fixture using the
  same comparator. This is required wherever anonymous PagePreview cannot render
  what the runner renders: `[[iftags]]` gates (preview evaluates every gate
  false) and `[[html]]` blocks (preview keeps them literal).
- Ad-hoc probes (new): `scripts/audit2/probe_preview.py`, two independent
  anonymous requests per shape, raw bodies and SHA-256 recorded.
- Classifier (new): `scripts/audit2/classify_audit.py`; structure (element +
  attribute token sequence) and collapsed visible text are compared separately,
  caller-runtime output is relaxed (newpage class, file-host URLs, Wikidot
  asset/init scripts, generated ids, html-block iframe URL), and a saved-page
  capture is used in preference to PagePreview when both exist.
- Frozen parity evidence was adjudicated before live probing: `bindings.json`
  and `tests/fixtures/wikidot-parity/references-*.jsonl` were consulted for each
  suspect fixture (notably `test--anchor--xss`, which has a frozen match).

## Preview-opaque shapes settled by saved pages

135 preview-opaque shapes were re-captured from saved pages (tags `fruit, component`):

- 9 × bug-encoded
- 10 × context-boundary (Wikijump gallery marker)
- 2 × context-boundary (caller user resolution)
- 2 × context-caller-runtime (asset-script+tabview-init-script)
- 1 × context-caller-runtime (caller-iframe)
- 3 × context-caller-runtime (fileurl)
- 2 × context-caller-runtime (newpage)
- 14 × harmless-whitespace (normalization-only)
- 92 × match-exact

Every capture was fetched twice; six first attempts hit a transient Wikidot 503 and were re-captured until both fetches returned the page content, so every committed capture is a pair of successful fetches (gallery captures are stable after masking the generated `gallery-box-<id>`).

| class | fixtures |
|---|---|
| bug-encoded | `date/subname-iftags-seam-whitespace`, `footnote/block-iftags-seam-name`, `iftags/list-item-inside-span-stays-nested`, `image/subname-iftags-seam-whitespace`, `math/eref-head-comment-seam`, `math/eref-head-segment-seam`, `table/iftags-seam-fused-close`, `tabview/iftags-seam`, `tabview/iftags-seam-abutting` |
| context-boundary (Wikijump gallery marker) | `gallery/entry-segment-in-url`, `gallery/iftags-seam-end-name`, `gallery/iftags-seam-fused-open`, `gallery/iftags-seam-fused-open-entries`, `gallery/iftags-seam-name`, `gallery/iftags-seam-name-args`, `gallery/iftags-seam-name-current-page-files`, `gallery/segment-after-arguments`, `gallery/segment-before-arguments`, `gallery/segment-inside-argument-value` |
| context-boundary (caller user resolution) | `user/iftags-seam`, `user/iftags-seam-mid-name` |
| context-caller-runtime (asset-script+tabview-init-script) | `tabview/iftags-seam-fused-close`, `tabview/iftags-seam-name` |
| context-caller-runtime (caller-iframe) | `iftags/comment-close-false` |
| context-caller-runtime (fileurl) | `file/iftags-seam`, `image/iftags-seam-name`, `image/subname-iftags-seam` |
| context-caller-runtime (newpage) | `link/iftags-seam`, `link/iftags-seam-trailing` |
| harmless-whitespace (normalization-only) | `iftags/body-div-literal`, `iftags/closer-midline`, `iftags/kept-gate-list-close-inside-unclosed-item`, `iftags/kept-spaced-head-doc-start`, `iftags/list-item-after-closer-joins`, `iftags/list-item-after-paragraph-break`, `iftags/list-item-joins-from-second-item`, `iftags/list-item-line-at-body-start`, `iftags/list-items-close-definition-list`, `iftags/list-items-join`, `iftags/list-items-join-ordered`, `iftags/list-items-split-on-marker-type`, `iftags/list-parabreak-starts-new-list`, `iftags/list-plain-line-before-starts-new-list` |
| match-exact | `anchor/iftags-seam-trailing`, `blockquote/iftags-kept-blank-line-new-run-shallower`, `blockquote/iftags-kept-deeper-row-first-segment-wraps`, `blockquote/iftags-kept-depth-two-sibling-quote`, `blockquote/iftags-kept-empty-first-segment-no-blockquote`, `blockquote/iftags-kept-empty-row-splits-paragraph`, `blockquote/iftags-kept-escaped-quote-with-inner-break`, `blockquote/iftags-kept-escapes-after-break`, `blockquote/iftags-kept-escapes-unquoted-continuation`, `blockquote/iftags-kept-quoted-segment-becomes-sibling`, `blockquote/iftags-kept-rows-form-blockquote`, `blockquote/iftags-kept-shallower-row-joins-run`, `blockquote/iftags-seam-line-start`, `bold/split-close-literal`, `bold/split-close-segment-body`, `bold/split-opener-segment-body`, `button/head-segment-seam`, `color/iftags-seam-spec`, `definition-list/iftags-body-escape`, `footnote/iftags-seam-name-kept-body`, `footnote/iftags-seam-name-whitespace-body`, `iftags/body-heading-crossed-closer`, `iftags/body-heading-then-text`, `iftags/body-paragraph-break-around`, `iftags/body-paragraph-break-in-span`, `iftags/body-paragraph-breaks`, `iftags/comment-close-true`, `iftags/empty-midline`, `iftags/fused-close`, `iftags/kept-bold-close-inside`, `iftags/kept-content-space-merge`, `iftags/kept-empty-first-segment-mixed`, `iftags/kept-empty-indented-closer`, `iftags/kept-gate-bibliography-close-inside`, `iftags/kept-gate-center-close-inside`, `iftags/kept-gate-collapsible-close-inside`, `iftags/kept-gate-div-close-inside`, `iftags/kept-gate-footnote-close-inside`, `iftags/kept-gate-list-item-close-inside`, `iftags/kept-gate-math-body-splice`, `iftags/kept-gate-math-close-inside`, `iftags/kept-gate-math-nested-crossed`, `iftags/kept-gate-math-nested-discarded-literal`, `iftags/kept-gate-math-nested-inner-literal`, `iftags/kept-glued-body-space`, `iftags/kept-heading-segment-wraps-text`, `iftags/kept-hr-segment-wraps-both-sides`, `iftags/kept-indented-closer`, `iftags/kept-indented-closer-blank-after`, `iftags/kept-indented-closer-doc-end`, `iftags/kept-indented-closer-glued-after`, `iftags/kept-indented-closer-space-after`, `iftags/kept-indented-closer-tab`, `iftags/kept-indented-opener`, `iftags/kept-list-segment-wraps-trailing-text`, `iftags/kept-mixed-segment-wraps-text`, `iftags/kept-mixed-segment-wraps-trailing-text`, `iftags/kept-para-tail-indented-closer`, `iftags/kept-spaced-head`, `iftags/kept-spaced-head-blank-before`, `iftags/kept-spaced-head-multiline`, `iftags/kept-unclosed-child-code`, `iftags/kept-unsafe-first-segment-then-text`, `iftags/lines-between-text`, `iftags/lines-in-span`, `iftags/list-plain-line-after-escapes`, `iftags/midline-open`, `iftags/nested-inner-literal`, `iftags/nested-literal`, `iftags/nested-literal-break`, `iftags/nested-unbalanced`, `iftags/spec-comment-close-in-comment-keep`, `iftags/spec-comment-straddle-keep`, `iftags/spec-fused-close-keep`, `iftags/spec-segment-inner-gate-keep`, `iftags/spec-straddle-bracket-keep`, `iftags/trailing-blank`, `iftags/two-gates-lines`, `iftags/whitespace-only-body`, `iftags/whitespace-only-body-run`, `iftags/whole-lines`, `italics/split-close-segment`, `list/iftags-body-escape`, `list/iftags-body-escape-empty-item`, `list/iftags-body-escape-ends-run`, `list/iftags-body-escape-single-line`, `math/iftags-seam-name`, `size/head-segment-seam`, `span/iftags-seam`, `span/iftags-seam-attribute-name`, `table/simple-split-close-iftags-excess`, `toc/iftags` |

## New bug-encoded fixtures

B1–B6 (pass 1) remain assigned to the fixture-fix lane and are not re-judged
here; their fixture still shows the same mismatch on this branch
(`code/css-pseudo-class-split`, `code/css-url-token-intact`, `list/comment-line-start`, `list/fused-close`, `misc/bibliography-duplicate-definition`, `span/fused-opener-scored-map-residual`). The items below are new
findings for a follow-up fix lane; B1–B6 verdicts are untouched.

### B7. Anchor dangerous-scheme href regression (`anchor/xss`, `anchor/href-slug`)

| fixture | live PagePreview sha256 |
|---|---|
| `anchor/xss` | `7c32bdb13bb55d45` |
| `anchor/href-slug` | `dca28623c9efb305` |

**Live rule.** Wikidot anchor blocks slugify the raw href instead of rejecting the scheme: `[[a href="javascript:alert(1)"]]` -> `javascript:alert-1`, `data:text/html;base64,AAAA` -> `data:text-html-base64-aaaa`, `HTTP://EXAMPLE.COM/A` -> `http:example-com-a`, `Some Page Name` -> `some-page-name`.

**FTML / fixture.** wikidot.html expects href="#invalid-url" for javascript:/data: anchor targets (regression introduced by commit 7227cb1cd, 2026-11-04).

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `anchor-js-control` | `8930def47a0ea28b` | `8930def47a0ea28b` | identical |
| `anchor-data-url` | `fa21bada6561cadd` | `fa21bada6561cadd` | identical |
| `anchor-http-upper` | `279657617f49e2a3` | `279657617f49e2a3` | identical |
| `anchor-spaces` | `85c3d5eb4534a994` | `85c3d5eb4534a994` | identical |
| `anchor-href-slug-fixture` | `dca28623c9efb305` | `dca28623c9efb305` | identical |

**Notes.** Frozen parity reference test--anchor--xss (references-20260815-01.jsonl, source_sha256 5fc2cb1a..., raw_html_sha256 7c32bdb1...) matches the fresh capture and its binding says status=match with ftml_html_sha256 f252ad81...; the current fixture hash is 2aae557c..., so the fixture drifted after the frozen capture. docs/WikidotCompatibilityHazards.md still declares this hazard a parity requirement. Disposition decision required: either restore parity or record an explicit documented `security-boundary` disposition (today the hardening has neither a hazards-doc update nor a binding disposition).

### B8. Bracket-anchor reference text skips typography (`anchor/name-with-dots`)

| fixture | live PagePreview sha256 |
|---|---|
| `anchor/name-with-dots` | `7dedc203a4f99c95` |

**Live rule.** Literal bracket-anchor reference text still goes through typography: `[#aa...bb ref]` -> `[#aa…bb ref]` (U+2026). The `[[# aa...bb]]` anchor name attribute keeps the three dots.

**FTML / fixture.** `[#aa...bb ref]` stays literal `[#aa...bb ref]`; the anchor name attribute also stays `aa...bb`.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `anchor2-ref-dots` | `4f161d8e864af3f0` | `4f161d8e864af3f0` | identical |
| `anchor2-ref-dots-only` | `a2ec5b2cd1f8cef1` | `a2ec5b2cd1f8cef1` | identical |
| `anchor2-anchor-dots` | `227bc2cfd196b05c` | `227bc2cfd196b05c` | identical |
| `anchor2-link-dots` | `3833d48843570892` | `3833d48843570892` | identical |

**Notes.** Probes show the conversion in any bracket-anchor reference text and in ordinary prose, but not in the anchor name attribute value.

### B9. Unclosed `[[bibliography]]` opener swallows its line break (`bibliography/unclosed-chain-literal`)

| fixture | live PagePreview sha256 |
|---|---|
| `bibliography/unclosed-chain-literal` | `55128b03754dbbfe` |

**Live rule.** A literalized `[[bibliography]]` opener keeps the newline before it: `unterminated-0<br>[[bibliography]]<br>unterminated-1<br>...`; a run of unclosed openers renders as one paragraph with `<br>` between every source line.

**FTML / fixture.** `unterminated-0[[bibliography]]<br>` - the failed opener is literal but the preceding line break is gone.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `bib-chain-fixture` | `55128b03754dbbfe` | `55128b03754dbbfe` | identical |
| `bib-one-unclosed` | `fd7c529fdb219182` | `fd7c529fdb219182` | identical |
| `bib-two-unclosed` | `1034431b32b42ee2` | `1034431b32b42ee2` | identical |
| `bib-unclosed-then-text` | `619567005929bac0` | `619567005929bac0` | identical |
| `bib-unclosed-blank` | `4875f595f1ab8e11` | `4875f595f1ab8e11` | identical |
| `bib2-unclosed-then-bq` | `523c868513b0aae6` | `523c868513b0aae6` | identical |

**Notes.** The same shape with a following blockquote/list/heading keeps the openers literal at the start of the paragraph; only the inter-line break before each unclosed opener differs.

### B10. Paragraph wrapper for a line-break-only blockquote body (`blockquote/indented-underscore-break`)

| fixture | live PagePreview sha256 |
|---|---|
| `blockquote/indented-underscore-break` | `54e18f12b46cbcbb` |

**Live rule.** `>  _` (quote marker, two spaces, lone underscore at end of line) renders `<blockquote><br>\n<br></blockquote>` with no `<p>` wrapper.

**FTML / fixture.** `<blockquote><p><br>\n<br>\n</p></blockquote>` - a `<p>` wrapper around the line breaks.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `bq-underscore-fixture` | `54e18f12b46cbcbb` | `54e18f12b46cbcbb` | identical |
| `bq-underscore-one-space` | `5e9bcaa0ee7f955f` | `5e9bcaa0ee7f955f` | identical |
| `bq-underscore-two-space-nonl` | `54e18f12b46cbcbb` | `54e18f12b46cbcbb` | identical |
| `bq-underscore-two-space-nl` | `54e18f12b46cbcbb` | `54e18f12b46cbcbb` | identical |
| `bq-underscore-then-text` | `21dfc6a81c5d6b25` | `21dfc6a81c5d6b25` | identical |
| `bq-underscore-two-rows` | `3365aeddd3ac1fcc` | `3365aeddd3ac1fcc` | identical |
| `bq2-underscore-two` | `54e18f12b46cbcbb` | `54e18f12b46cbcbb` | identical |
| `bq2-underscore-three` | `54e18f12b46cbcbb` | `54e18f12b46cbcbb` | identical |
| `bq2-underscore-two-space-trail` | `5e9bcaa0ee7f955f` | `5e9bcaa0ee7f955f` | identical |
| `bq2-double-underscore` | `44f7c26c00c7255e` | `44f7c26c00c7255e` | identical |
| `bq2-underscore-a` | `737d523e4d023cbd` | `737d523e4d023cbd` | identical |
| `bq2-underscore-then-blank` | `8cb5f7933b5a79be` | `8cb5f7933b5a79be` | identical |

**Notes.** Probes isolate the trigger: `> _`, `>  __`, `>  ___`, `>  _a`, `>  _x_`, `>  _#` and `>  _ ` all render `<blockquote><p>…</p></blockquote>` in live; only the lone `_` as the final character after two or more spaces drops the paragraph wrapper. The same body shape preceded by text (`a\n>  _\nb`) still drops the wrapper.

### B11. Trailing `<br>` in a footnote footer whose body starts with a literalized block (`footnote/*-first-line-literal`)

| fixture | live PagePreview sha256 |
|---|---|
| `footnote/blockquote-first-line-literal` | `9a6944d2f15bc0c6` |
| `footnote/definition-list-first-line-literal` | `ed1ae45bf4b5ba43` |
| `footnote/heading-first-line-literal` | `b4b0bd3a763d060d` |
| `footnote/table-first-line-literal` | `cf2f362ba30c647d` |
| `footnote/tabview-first-line-literal` | `424d9e876b1da80d` |

**Live rule.** The footnote footer ends immediately after the body text: `…&gt; quoted</div>`; no trailing `<br>` for any first-line-literal body shape (quote, heading, table, definition list, tabview source lines).

**FTML / fixture.** The footnote footer ends with `…<br>\n</div>`.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `fn-bq-fixture` | `9a6944d2f15bc0c6` | `9a6944d2f15bc0c6` | identical |
| `fn-bq-no-trailing-nl` | `9a6944d2f15bc0c6` | `9a6944d2f15bc0c6` | identical |
| `fn-bq-blank-end` | `9a6944d2f15bc0c6` | `9a6944d2f15bc0c6` | identical |
| `fn-bq-inline` | `9a6944d2f15bc0c6` | `9a6944d2f15bc0c6` | identical |
| `fn-head-fixture` | `b4b0bd3a763d060d` | `b4b0bd3a763d060d` | identical |
| `fn-table-fixture` | `cf2f362ba30c647d` | `cf2f362ba30c647d` | identical |
| `fn-deflist-fixture` | `ed1ae45bf4b5ba43` | `ed1ae45bf4b5ba43` | identical |
| `fn-tabview-first` | `424d9e876b1da80d` | `424d9e876b1da80d` | identical |
| `fn-plain` | `a5efee6818db26cf` | `a5efee6818db26cf` | identical |
| `fn-list` | `265528424a5a9186` | `265528424a5a9186` | identical |
| `fn-bq-then-text` | `b46af12bc8d0e760` | `b46af12bc8d0e760` | identical |
| `fn2-underline-first` | `4f9705b28ce76f09` | `4f9705b28ce76f09` | identical |

**Notes.** Controls `fn-plain`, `fn-empty`, `fn-list`, `fn-bq-then-text` match; only bodies whose first non-blank line is a literalized block marker (blockquote, heading, table row, definition list, tabview opener) gain the extra break in FTML.

### B12. Fused-closer residual ends the inline flow (div/collapsible/table closers)

| fixture | evidence | live sha256 |
|---|---|---|
| `div/fused-close` | PagePreview | `1324aaa78ee1da4f` |
| `collapsible/fused-close` | PagePreview | `261d2634f07a5f67` |
| `table/fused-close` | PagePreview | `c90f7cff4c08c0d7` |
| `table/iftags-seam-fused-close` | saved page `muse-probe-audit2-127` | `600c2ff16e4477ee` |

**Live rule.** After a fused closer the residual `]` and the following non-blank line stay in one inline flow separated by `<br>`: `</div>\n]<br>\nafter` (table: `<p>]<br>\nafter</p>`); only a blank line starts a new paragraph.

**FTML / fixture.** `</div>]<p>after</p>` (table: `</table>]<p>after</p>`) - the residual `]` is bare and the next line starts a paragraph.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `div-fused-fixture` | `1324aaa78ee1da4f` | `1324aaa78ee1da4f` | identical |
| `div-fused-eof` | `167290d74d671cf0` | `167290d74d671cf0` | identical |
| `div-fused-blank` | `ef21d5eec27c90d1` | `ef21d5eec27c90d1` | identical |
| `div-fused-plain` | `b34dc9c687acfbc1` | `b34dc9c687acfbc1` | identical |
| `div-fused-x` | `3d60318c2fc98dec` | `3d60318c2fc98dec` | identical |
| `div-fused-two-lines` | `00b1c41ed07e21fc` | `00b1c41ed07e21fc` | identical |
| `coll-fused-fixture` | `261d2634f07a5f67` | `261d2634f07a5f67` | identical |
| `div3-two-residual` | `ede1abeef942ad14` | `ede1abeef942ad14` | identical |
| `div3-same-line-residual` | `fbb8b72592c0d32d` | `fbb8b72592c0d32d` | identical |
| `div3-after-two-lines` | `cb79c477c2cf03ac` | `cb79c477c2cf03ac` | identical |
| `table3-fused-blank` | `134515eb2534a514` | `134515eb2534a514` | identical |
| `coll3-fused-blank` | `48fd3ff0090f3899` | `48fd3ff0090f3899` | identical |

**Notes.** Probes fix the live rule: residual on the following line joins the next line with `<br>`; two residuals (`]]`) join as `]]<br>`; a residual on the same line stays same-line (`] after`); a blank line after the residual restores the paragraph split. `[[div]]…[[/div]]` followed by text (no residual) already matches FTML (`div/empty-attributed-stays`). The gate-seam variant `table/iftags-seam-fused-close` (saved page `muse-probe-audit2-127`, content sha256 600c2ff16e4477ee) shows the same class with no following line: live emits `<table>…</table><p>]</p>`, FTML a bare `]`.

### B13. `[[eref]]` after a literal inline math falls through to link parsing (`math/eref-head-comment-seam`, `math/eref-head-segment-seam`)

| fixture | evidence | live sha256 |
|---|---|---|
| `math/eref-head-comment-seam` | saved page `muse-probe-audit2-123` | `6f1c6c19f3fb91a2` |
| `math/eref-head-segment-seam` | saved page `muse-probe-audit2-124` | `6f1c6c19f3fb91a2` |

**Live rule.** `[[math eq1]]x^2[[/math]]` stays literal (inline math form) and the following `[[eref eq1]]` still renders an eref anchor: `<a class="eref" href="javascript:;" onclick="WIKIDOT.page.utils.scrollToReference('equation-')"></a>` (empty label when the equation is unknown). A standalone `[[eref eq1]]` renders the same anchor.

**FTML / fixture.** Renders `[[math eq1]]x^2[<a href="/math]]">[[eref eq1</a>]` - the `[[eref …]]` block is not recognized and the surrounding brackets/`[[/math]]` become a link.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `math-eref-fixture` | `c30a904dfc199450` | `c30a904dfc199450` | identical |
| `math-eref-plain` | `c30a904dfc199450` | `c30a904dfc199450` | identical |
| `math2-eref-alone` | `1815a806727dbdd0` | `1815a806727dbdd0` | identical |
| `math2-math-then-eref-block` | `f046422b4a12d3fd` | `f046422b4a12d3fd` | identical |
| `math-simple` | `6e5707d1e78a305b` | `6e5707d1e78a305b` | identical |
| `math-label` | `1958ce45430f27f0` | `1958ce45430f27f0` | identical |

**Notes.** Both fixtures reduce to the same head `[[eref eq1]]` after comment/gate removal; the same shape with a real rendered equation head matches live, so only the literal-inline-math context is affected.

### B14. Tabview panel visibility: non-selected panels lack `style="display:none"`

| fixture | evidence | live sha256 |
|---|---|---|
| `tabview/blockquote` | PagePreview | `a31102bd9a9880a2` |
| `tabview/blockquote-nested` | PagePreview | `c45e2c6316941f65` |
| `tabview/title-footnote-numbering` | PagePreview | `d69d449047ce5190` |
| `tabview/title-typography-raw` | PagePreview | `df78044f4fae4dab` |
| `tabview/iftags-seam-abutting` | saved page `muse-probe-audit2-130` | `7ba3ada35bcb3a62` |

**Live rule.** Every tab panel after the first is emitted as `<div id="wiki-tab-…" style="display:none">`; the first panel has no style.

**FTML / fixture.** All `wiki-tab` content divs are emitted without a style attribute, so non-selected tab bodies are visible without JavaScript.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `tab2-plain-two-tabs` | `076713048e712467` | `4fdafd37eecb2d25` | canonically identical |
| `tab2-blockquote` | `a019a811f7665bb9` | `9d9fe123872de3a5` | canonically identical |
| `tab2-title-typography-raw` | `a1ba36f3b7b88c57` | `f820df9112ef013e` | canonically identical |

**Notes.** Fixtures with a single tab are unaffected. Live also emits a per-page `<script src=…tabview-min.js>` asset include and an inline YUI `new YAHOO.widget.TabView(...)` init script plus per-request `wiki-tabview-<hex>` ids; both are caller/browser-runtime and are relaxed for the audit (they are not counted as mismatches).

### B15. Trailing `<br>` in tab bodies of blockquote-hosted tabviews

| fixture | live PagePreview sha256 |
|---|---|
| `tabview/blockquote` | `a31102bd9a9880a2` |
| `tabview/blockquote-nested` | `c45e2c6316941f65` |
| `tabview/blockquote-nested-tab-literal` | `300a6dc816bd5827` |
| `tabview/blockquote-sameline-closes` | `651bb7c0b448b9ec` |

**Live rule.** Quoted tab bodies end `Body one</p>` - no trailing `<br>`.

**FTML / fixture.** Tab body paragraphs end `Body one<br>\n</p>`.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `tab2-blockquote` | `a019a811f7665bb9` | `9d9fe123872de3a5` | canonically identical |
| `tab2-blockquote-eject` | `e496b5c3401fb5f4` | `471832cd2a386275` | canonically identical |
| `tab2-blockquote-body-shallower-line` | `f70f745ead369761` | `ac54c1f805872e71` | canonically identical |

**Notes.** Two additional tabview cases (`tabview/blockquote` and `tabview/blockquote-nested`) also carry B14.

### B16. Empty `<em>` wrapper in tabview nav titles

| fixture | live PagePreview sha256 |
|---|---|
| `tabview/title-head-only-unit` | `ff412c661daeba4f` |
| `tabview/title-inline-unit` | `719f7d87346a42af` |
| `tabview/title-footnote-numbering` | `d69d449047ce5190` |
| `tabview/title-footnote-unit` | `1e95c54d2cbb3fcd` |

**Live rule.** Live emits `<a href="javascript:;"></a>` with no `<em>` element when the computed title is empty (title built only from a caller-owned placeholder such as `[[user]]` or from an elided `[[footnote]]`).

**FTML / fixture.** Emits `<a href="javascript:;"><em></em></a>` when the computed tab title is empty.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `tab2-empty-title` | `85c2e6610468ab3d` | `bca2a28fd6a7900b` | canonically identical |
| `tab2-title-footnote` | `d9a89403629afbe2` | `1b6b2790159eef19` | canonically identical |
| `tab2-title-inline-unit` | `164c5e4950e03a6a` | `e18a4c7edaf4f513` | canonically identical |

**Notes.** Caller-owned title content (user lookup) is separately caller-runtime; the structural `<em>` wrapper is FTML-owned.

### B17. Tabview row ejection leaves an unmatched empty trailing blockquote in live

| fixture | live PagePreview sha256 |
|---|---|
| `tabview/blockquote-eject` | `340277dfbea8a526` |
| `tabview/blockquote-eject-blank-separated` | `64d7bb56bb647177` |
| `tabview/blockquote-eject-depth2` | `47c9408db69f691c` |
| `tabview/blockquote-eject-lesser-row` | `5da2a9e3a5f69d2b` |
| `tabview/blockquote-eject-multiple` | `e75eb220b07048c7` |
| `tabview/blockquote-eject-row-between` | `35b56738c25d234e` |
| `tabview/blockquote-eject-sameline-closes` | `0a86298509978cc5` |
| `tabview/blockquote-eject-shallower-closes` | `c4d3b11bf60a6638` |
| `tabview/blockquote-body-shallower-line` | `2c98d0ed2c1f3926` |

**Live rule.** A row that ejects out of the quoted tabview leaves an empty `<blockquote></blockquote>` (or a nested empty pair at depth 2) as the last child of the outer blockquote.

**FTML / fixture.** The quote nesting closes cleanly; no empty blockquote element remains.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `tab2-blockquote-eject` | `e496b5c3401fb5f4` | `471832cd2a386275` | canonically identical |
| `tab2-blockquote-body-shallower-line` | `f70f745ead369761` | `ac54c1f805872e71` | canonically identical |

**Notes.** Minimal-difference search: removing only empty `<blockquote>` elements from the live capture makes every listed fixture match; depth-2 needs iterative removal.

### B18. Text after a closing tabview starts a paragraph in FTML but continues inline in live (`tabview/blockquote-text-around`)

| fixture | live PagePreview sha256 |
|---|---|
| `tabview/blockquote-text-around` | `6a5c6b857dae3a06` |

**Live rule.** Inside the blockquote: `…</div><br>\noutro</blockquote>` - the line after the tabview continues the inline flow with a `<br>`, with no paragraph wrapper. At top level the same shape also gives `…</div><br>\noutro`; a blank line restores `<p>outro</p>`.

**FTML / fixture.** Inside the blockquote: `…</div><p>outro</p></blockquote>`.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `tab3-outro-top` | `67420e4effcc8e5f` | `2d4ff129f2e323d7` | canonically identical |
| `tab3-outro-top-blank` | `6c296dccf1045ccd` | `7c68a26e05e60932` | canonically identical |
| `tab3-outro-in-bq` | `eef3d8578a46969c` | `cc15290e91f4898d` | canonically identical |

**Notes.** FTML already models plain block+text continuation correctly for `[[div]]` (`div/empty-attributed-stays`); the paragraph start is tabview-specific.

### B19. Tabview failure/recovery shapes differ (`tab-stray-fails`, `title-footnoteblock-*`, `title-opener-literal`)

| fixture | live PagePreview sha256 |
|---|---|
| `tabview/tab-stray-fails` | `be6c746b650e02ed` |
| `tabview/title-footnoteblock-after` | `cec147dd6dacdc34` |
| `tabview/title-footnoteblock-alone` | `5d7cf32082eca354` |
| `tabview/title-opener-literal` | `e94496700ab2b9d9` |

**Live rule.** Live recovers a tabview from the later well-formed attempt in the same source, forms a navset whose title is the literal `[[tab ]]` text when the title contains `[[footnoteblock]]`, and keeps `[[/tab]]` residual text inside the panel body (`<p>]]<br>`).

**FTML / fixture.** FTML leaves the failed tabview source literal (`<p>[[tabview]]<br>…</p>`) or mangles the tab body opener (`]` before `<p>]</p>`).

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `tab2-tab-stray-fails` | `c4ebfd8d69fff94b` | `09b155717bb25323` | canonically identical |
| `tab2-title-footnoteblock-alone` | `5da320e83a2b88e6` | `826093c034e3c2e1` | canonically identical |
| `tab2-empty-title` | `85c2e6610468ab3d` | `bca2a28fd6a7900b` | canonically identical |

**Notes.** These are parse-recovery divergences rather than rendering differences; each fixture is pinned from FTML output.

### B20. `[[date]]` head with extra whitespace-separated tokens renders literally (`date/subname-iftags-seam-whitespace`)

| fixture | evidence | live sha256 |
|---|---|---|
| `date/subname-iftags-seam-whitespace` | saved page `muse-probe-audit2-25` | `041313bd96cfd4e2` |

**Live rule.** A `[[date]]` head with more than one whitespace-separated token still renders a date from the first token and ignores the rest: `[[date 123 45]]` and `[[date 123 45 67]]` both render `<span class="odate time_123">01 Jan 1970 00:02</span>`.

**FTML / fixture.** The block fails and the marker is emitted literally, with the typed space turned into `&nbsp;`: `<p>[[date 123\u00a045]]</p>`.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `date3-two-tokens` | `0a719f8a5825eeb2` | `0a719f8a5825eeb2` | identical |
| `date3-one` | `0a719f8a5825eeb2` | `0a719f8a5825eeb2` | identical |
| `date3-subname-space` | `0a719f8a5825eeb2` | `0a719f8a5825eeb2` | identical |

**Notes.** Found only after a saved-page capture: PagePreview evaluates the `[[iftags -zzz]]` seam false, so the preview-renderable source is the reduced `[[date 12]]` form. Saved page `muse-probe-audit2-25`, saved content sha256 041313bd96cfd4e2, two fetches stable.

### B21. `[[footnoteblock]]` assembled across an iftags seam right after a paragraph line stays literal (`footnote/block-iftags-seam-name`)

| fixture | evidence | live sha256 |
|---|---|---|
| `footnote/block-iftags-seam-name` | saved page `muse-probe-audit2-28` | `979daca6fffc2584` |

**Live rule.** Live recognizes the block assembled across the gate seam and consumes it: `<p>a<sup …>1</sup></p><div class="footnotes-footer">…` (a `[[footnoteblock]]` with no matching footnote renders nothing, as in `footnote/block-empty`).

**FTML / fixture.** Leaves the marker literal inside the paragraph: `<p>a<sup …>1</sup><br>\n[[footnoteblock]]</p><div class="footnotes-footer">…`.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `fnb3-alone` | `75a11da44c802486` | `75a11da44c802486` | identical |
| `fnb3-after-footnote` | `9903c0bad37c6072` | `9903c0bad37c6072` | identical |
| `fnb3-inline-name` | `75a11da44c802486` | `75a11da44c802486` | identical |

**Notes.** Found only after a saved-page capture (the `-zzz` gate is false in PagePreview). Saved page `muse-probe-audit2-28`, saved content sha256 979daca6fffc2584, two fetches stable. The seed shape is a block name split by a gate: `[[foot[[iftags -zzz]][[/iftags]]noteblock]]`.

### B22. List item opened inside a span becomes nested instead of a sibling (`iftags/list-item-inside-span-stays-nested`)

| fixture | evidence | live sha256 |
|---|---|---|
| `iftags/list-item-inside-span-stays-nested` | saved page `muse-probe-audit2-92` | `744b88cf6df3c4f7` |

**Live rule.** Both items are siblings: `<ul><li><span>A</span></li><li><span>B</span></li></ul>` — the span closes after `A` and the second list line is a sibling item.

**FTML / fixture.** Nests the second item under the first: `<ul><li><span>A</span><ul><li><span>B</span></li></ul></li></ul>`.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `list3-span-first` | `62da7ee06fc21ab3` | `62da7ee06fc21ab3` | identical |
| `list3-span-first-indent` | `6d3a74da7155ddf1` | `6d3a74da7155ddf1` | identical |
| `list3-span-plain` | `94db93e7a024b31c` | `94db93e7a024b31c` | identical |

**Notes.** Found only after a saved-page capture (the `+fruit` gate is true under the corpus tags but false in PagePreview). Saved page `muse-probe-audit2-92`, saved content sha256 744b88cf6df3c4f7, two fetches stable. The gate seam is what puts `A` and `B` in one span; the list nesting ownership is the FTML-owned part.

### B23. Quoted tab head attribute map is title text in FTML (`tabview/iftags-seam`)

| fixture | evidence | live sha256 |
|---|---|---|
| `tabview/iftags-seam` | saved page `muse-probe-audit2-129` | `f5e712d329a10dc9` |

**Live rule.** A quoted `key=value` tab head is parsed as an attribute map: `[[tab label="A"]]` renders title `untitled` (unknown key ignored) and `[[tab title="A"]]` renders title `A`; only the unquoted `[[tab label=A]]` form becomes literal title text `label=A`.

**FTML / fixture.** The whole head becomes the title: `<em>label="AB"</em>`.

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `tab5-label-attr` | `644ef5f6512cd9d6` | `57c4234b3bd096a2` | canonically identical |
| `tab5-title-attr` | `380f2b0de621a582` | `98716c58d2ee2510` | canonically identical |
| `tab5-label-plain` | `85851911de2a2993` | `81c0f41e0d2eb4d5` | canonically identical |

**Notes.** Found only after a saved-page capture (the `-zzz` gate seam is false in PagePreview). Saved page `muse-probe-audit2-131`, saved content sha256 f5e712d329a1 (see audit/classifications.json for the exact full hash), two fetches stable.

### B24. Whitespace in an image head: live splits filename from attribute tokens (`image/subname-iftags-seam-whitespace`)

| fixture | evidence | live sha256 |
|---|---|---|
| `image/subname-iftags-seam-whitespace` | saved page `muse-probe-audit2-119` | `a5d243b6ddd49b08` |

**Live rule.** The image filename is the first whitespace-delimited token; later tokens are attribute-like and, when any are present, live wraps the image in a link to the file and uses the resized `medium` URL: `[[image pictu re.png]]` -> `<a href="…/pictu"><img alt="pictu" class="image" src="…/resized-images/…/pictu/medium.jpg"></a>`. The same wrapper appears for `[[image a.png size="small"]]`; a head with no attributes (`[[image abc.png]]`) stays a bare `<img>`.

**FTML / fixture.** Keeps the whole space-containing value as the filename: `<img alt="pictu re.png" class="image" src="…/pictu re.png">` (no link wrapper).

**Live probes (PagePreview twice per shape).**

| probe | capture 1 sha256 | capture 2 sha256 | twice |
|---|---|---|---|
| `img5-space` | `e3c2260772ba78aa` | `e3c2260772ba78aa` | identical |
| `img5-space2` | `f768a52d342de2b3` | `f768a52d342de2b3` | identical |
| `img5-normal` | `8798ec8f6147459a` | `8798ec8f6147459a` | identical |
| `img5-space-size` | `e158bd3210c2e71b` | `e158bd3210c2e71b` | identical |

**Notes.** Found only after a saved-page capture (the `-zzz` gate seam is false in PagePreview). Saved page `muse-probe-audit2-119`, saved content sha256 a5d243b6ddd49b08, two fetches stable. The filename URL/host itself is caller-owned, but the token split and link wrapper are FTML-owned.


## Whitespace adjudication

All 53 cases are raw-DOM differences made only of whitespace text runs. For every case the element/attribute topology and the collapsed visible text are identical, the differing runs sit at a fragment/block/line edge, and no `pre`/`code`/`textarea` text differs. Position classes observed: line-start whitespace ×32, trailing block-edge whitespace ×25, line-edge (after br) ×11. Verdict: **normalization-only** (not visually observable under the HTML whitespace-collapsing rules). This is an audit verdict, not a comparison-normalization disposition: per `docs/ParityTests.md`, promoting a binding to `comparison-normalization` still needs a browser DOM review and an entry in `tests/fixtures/wikidot-parity/comparison-normalization-contracts.json`.

| fixture | evidence | live sha256 | whitespace position |
|---|---|---|---|
| `code/closer-first-suffixed-wins` | page-preview | `c7e0b88ae74f6f8c` | line-start whitespace |
| `code/closer-whitespace-then-text` | page-preview | `05965146d1f12656` | line-start whitespace |
| `code/nested-suffixed-outer-closer` | page-preview | `ed2f9bcdb13e820a` | line-start whitespace |
| `code/two-codes-suffixed-first-closer` | page-preview | `eb8813db84386fb2` | line-start whitespace, trailing block-edge whitespace |
| `div/list-item-end` | page-preview | `f28717f5bd791f1f` | line-start whitespace |
| `footnote/block-scope-wrapped` | page-preview | `8eb2131eeb3a8153` | line-edge (after br) |
| `footnote/code-in-footnote-suffixed-closer` | page-preview | `a3c6d5acc97fd0c1` | line-start whitespace, trailing block-edge whitespace |
| `footnote/heading-end` | page-preview | `f1eff40da459946d` | trailing block-edge whitespace |
| `footnote/list-item-end` | page-preview | `c96dc23a1b99e37a` | trailing block-edge whitespace |
| `iftags/body-div-literal` | saved-page | `cd611be213b4817a` | trailing block-edge whitespace |
| `iftags/closer-midline` | saved-page | `b2b8cda37c049e4d` | line-start whitespace |
| `iftags/kept-gate-list-close-inside-unclosed-item` | saved-page | `4928e8871bf033b6` | line-start whitespace |
| `iftags/kept-spaced-head-doc-start` | saved-page | `449ce08a84f92cc0` | line-start whitespace |
| `iftags/list-item-after-closer-joins` | saved-page | `855b4157640103d4` | trailing block-edge whitespace |
| `iftags/list-item-after-paragraph-break` | saved-page | `11a9555a4fa1e1c6` | trailing block-edge whitespace |
| `iftags/list-item-joins-from-second-item` | saved-page | `855b4157640103d4` | trailing block-edge whitespace |
| `iftags/list-item-line-at-body-start` | saved-page | `11a9555a4fa1e1c6` | trailing block-edge whitespace |
| `iftags/list-items-close-definition-list` | saved-page | `4de7e9a12c6683a5` | trailing block-edge whitespace |
| `iftags/list-items-join` | saved-page | `a0f65ebca56c1dd9` | trailing block-edge whitespace |
| `iftags/list-items-join-ordered` | saved-page | `730c55d0e078a633` | trailing block-edge whitespace |
| `iftags/list-items-split-on-marker-type` | saved-page | `a03d25960ad76b6e` | trailing block-edge whitespace |
| `iftags/list-parabreak-starts-new-list` | saved-page | `9f7d5e096c609f98` | trailing block-edge whitespace |
| `iftags/list-plain-line-before-starts-new-list` | saved-page | `290d1e2a66ce8db8` | trailing block-edge whitespace |
| `list/explicit-crossed-pre-trailing-dissolves` | page-preview | `483657d5e9e88477` | line-edge (after br) |
| `list/explicit-post-table-literal` | page-preview | `7ac859f0e3404d2a` | line-start whitespace |
| `list/explicit-pre-kept-before-prose` | page-preview | `65f708cada99a247` | line-start whitespace |
| `list/explicit-scope-break-inside-paragraph` | page-preview | `10b846067d257679` | line-edge (after br) |
| `list/explicit-scope-break-paragraph` | page-preview | `488b5e5b878344a0` | line-edge (after br) |
| `list/nested-multi-item-partial-close` | page-preview | `6c881923fdf2ed1b` | line-edge (after br), line-start whitespace |
| `list/nested-partial-close-in-item` | page-preview | `2433ad7e486774f7` | line-edge (after br), line-start whitespace |
| `list/zombie-pair-fired-scope-not-reopened` | page-preview | `27e52c3920572529` | line-edge (after br), line-start whitespace |
| `misc/bibliography-second-block` | page-preview | `a37f295156f765b0` | line-start whitespace |
| `misc/dash-line-fusion` | page-preview | `a71ee2e6d4adace4` | line-edge (after br), line-start whitespace |
| `span/basic` | page-preview | `b7190d37bc1cd575` | line-edge (after br) |
| `strikethrough/end-block-closes-scope` | page-preview | `384af40530ed6ba2` | line-start whitespace |
| `table/advcell-collapsible` | page-preview | `ed34ad98cd0634d4` | line-start whitespace |
| `table/eof-stray-root` | page-preview | `bf389fb03b58b670` | line-start whitespace |
| `table/paragraph-after-table-scope` | page-preview | `7812b0fbc6a7e96b` | line-edge (after br) |
| `table/simple-cell-carry-in` | page-preview | `933a7ef3a5360c5e` | line-edge (after br) |
| `table/simple-unterminated-row-advanced-body` | page-preview | `7c6c9392c98d0116` | line-start whitespace |
| `table/simple-unterminated-row-table-opener` | page-preview | `8b1f14f79b788289` | line-start whitespace |
| `table/t-c-before-closein` | page-preview | `95bc3aaabda8ab00` | line-start whitespace, trailing block-edge whitespace |
| `table/t-d-before-closein` | page-preview | `fb28a500734410ff` | line-start whitespace, trailing block-edge whitespace |
| `table/t-d-before-closein2` | page-preview | `0a35f014c88a586e` | line-start whitespace, trailing block-edge whitespace |
| `table/t-d-cell` | page-preview | `d065486e6abd17e4` | line-start whitespace |
| `table/t-d-in-c-in-cell` | page-preview | `41865af45bab908a` | line-start whitespace |
| `table/tf-block-after` | page-preview | `448ba70256a260a2` | line-start whitespace, trailing block-edge whitespace |
| `table/tf-coll-open-in-outer-cell` | page-preview | `8675413257e4f8c3` | line-start whitespace, trailing block-edge whitespace |
| `table/tf-first-in-cell` | page-preview | `276d0ebd153ef616` | trailing block-edge whitespace |
| `table/tf-inline-before-cell` | page-preview | `d49642de7088ac0d` | line-start whitespace, trailing block-edge whitespace |
| `table/tf-nested-table` | page-preview | `a5e92f08e137a071` | line-start whitespace, trailing block-edge whitespace |
| `table/tf-next-row` | page-preview | `6037dc7209e689ab` | line-start whitespace, trailing block-edge whitespace |
| `table/tf-second-close` | page-preview | `626b038c8e0094fb` | line-start whitespace, trailing block-edge whitespace |

The 17 pass-1 whitespace cases were re-checked: 16 still reproduce as
whitespace-only (the 17th, `misc/quote-pair-close-in-owned-value`, is a
caller-owned file-host URL difference plus whitespace and is classified
caller-runtime). The remaining 37 cases are new: 18 `table`, 14 `iftags`, 4 `footnote`, 1 `div`
(14 of them found only on a saved page).

## Caller-runtime and boundary classes

These are not counted as bugs; they are the frozen dispositions named in
`tests/fixtures/wikidot-parity/caller-runtime-contracts.json` and
`docs/WikijumpBoundary.md`.

| caller-runtime / boundary class | fixtures |
|---|---|
| context-boundary (Wikijump embedvideo marker) | 1: `embedvideo/head-comment-seam` |
| context-boundary (Wikijump gallery marker) | 21: `gallery/comment-line-between-entries`, `gallery/comment-seam-end-name`, `gallery/comment-seam-name`, `gallery/entry-comment-in-url`, `gallery/entry-segment-in-url`, `gallery/fused-close-residual`, `gallery/fused-close-residual-text-after`, `gallery/fused-open-arguments-entries` (+13 more; full list in `audit/classifications.json`) |
| context-boundary (caller user resolution) | 7: `user/basic`, `user/casing`, `user/comment-name`, `user/iftags-seam`, `user/iftags-seam-mid-name`, `user/name-typography-raw`, `user/slug` |
| context-caller-runtime (asset-script+tabview-init-script) | 19: `footnote/tabview-after-first-line-forms`, `tabview/blockquote-eject-before-body`, `tabview/blockquote-eject-deeper-row`, `tabview/blockquote-eject-opener-row`, `tabview/blockquote-shallower-closes`, `tabview/iftags-seam-fused-close`, `tabview/iftags-seam-name`, `tabview/plain-nested-tab-literal` (+11 more; full list in `audit/classifications.json`) |
| context-caller-runtime (caller-iframe) | 1: `iftags/comment-close-false` |
| context-caller-runtime (fileurl) | 11: `file/iftags-seam`, `file/name-typography-raw`, `file/wikidot-attachment`, `image/basic`, `image/comment-filename`, `image/comment-filename-gap`, `image/iftags-seam-name`, `image/source-typography-raw` (+3 more; full list in `audit/classifications.json`) |
| context-caller-runtime (newpage) | 12: `color/whitespace-only-body-after-link`, `link/canonical-inline`, `link/comment-label`, `link/comment-target`, `link/iftags-seam`, `link/iftags-seam-trailing`, `link/label-quotes-raw`, `link/triple` (+4 more; full list in `audit/classifications.json`) |
| context-caller-runtime (newpage+fileurl+asset-script+tabview-init-script+gallery-id+tabview-id+caller-iframe) | 1: `tabview/tab-failed-div-inline-mangle` |

Additional PagePreview-specific observations:

- Anonymous PagePreview evaluates **every** `[[iftags]]` gate false (including
  `[[iftags -zzz]]`), so any fixture whose gate is true under the runner tags
  needs a saved page.
- PagePreview keeps `[[html]]` blocks literal; a saved page renders them as an
  iframe whose `src` is caller-supplied (`.../html/<hash>`), so the iframe URL
  is caller-runtime.
- PagePreview keeps inline `[[math]]` / `[[math label]]` text literal; the
  block form (`[[math]]` on its own lines) renders in both preview and saved
  pages.
- Tabview output includes a per-page asset `<script>` and an inline YUI init
  script plus per-request `wiki-tabview-<hex>` ids. None is FTML-owned; they are
  relaxed for the comparison, and the remaining tabview mismatches are the five
  structural classes above (B14–B19).
- PagePreview prepends root whitespace and serializes formatting newlines after
  block elements; this is the source of the whitespace-only class.

## Probe determinism

Every ad-hoc shape was posted twice with independent anonymous token cookies. Non-tabview shapes returned byte-identical bodies. Tabview bodies differ per request in two generated parts: the `wiki-tabview-<hex>` id (also embedded in the YUI init script) and the inline `new YAHOO.widget.TabView(...)` script; after masking those, both captures are identical for every tabview shape (verified in `audit/probes.json`). The corpus harness masks the same ids. Live also emits the per-page `<script src=.../tabview-min.js>` asset include, which FTML does not own.

## Remaining unjudgeable

None: every preview-opaque shape was settled by a saved-page capture.

## Committed evidence

- `audit/summary.json` - coverage and class totals per family.
- `audit/verdicts.jsonl` - one row per audited fixture (class, evidence, hashes).
- `audit/classifications.json` - per-non-match evidence, including live/expected
  structure and text and the saved-page reference where used.
- `audit/live-captures.jsonl.gz` - the raw PagePreview bodies keyed by source
  SHA-256 (985 unique sources; the 987 fixtures share two sources).
- `audit/saved-pages.json` - every `muse-probe-*` page, its tags, both saved
  content hashes, and the classification.
- `audit/whitespace.json` - the 53 whitespace adjudications with position class.
- `audit/b-items.json` - the new B-item definitions and their fixture lists.
- `audit/probes.json` - every ad-hoc probe with both raw bodies and hashes.
- `audit/tabview-class.json` - minimal-difference resolution per tabview fixture.
- `scripts/audit2/` - the saved-page harness, probe harness, classifier, and
  report/evidence builders.

## Reproduction

```sh
# Full PagePreview sweep (987 fixtures, ~30 min, anonymous read-only).
uv run --with html5lib python scripts/audit_corpus_live.py \
  --cache .audit-cache --output verdicts.jsonl

# Probe harness (needs a JSON list of {name, source}).
uv run --with html5lib python scripts/audit2/probe_preview.py \
  --list probes.json --out probes/

# Saved-page captures (sandbox account A; leaves muse-probe-* pages in place).
uv run --project /home/roku/src/Rokurolize/wikidot.py --with html5lib \
  python scripts/audit2/saved_probe.py --repo . --out saved-pages --cases-file cases.txt
```

## muse-probe-* pages left in place

| page | fixture |
|---|---|
| `muse-probe-audit2-1` | `color/iftags-seam-spec` |
| `muse-probe-audit2-10` | `blockquote/iftags-kept-deeper-row-first-segment-wraps` |
| `muse-probe-audit2-100` | `iftags/list-plain-line-after-escapes` |
| `muse-probe-audit2-101` | `iftags/list-plain-line-before-starts-new-list` |
| `muse-probe-audit2-102` | `iftags/midline-open` |
| `muse-probe-audit2-103` | `iftags/nested-inner-literal` |
| `muse-probe-audit2-104` | `iftags/nested-literal` |
| `muse-probe-audit2-105` | `iftags/nested-literal-break` |
| `muse-probe-audit2-106` | `iftags/nested-unbalanced` |
| `muse-probe-audit2-107` | `iftags/spec-comment-close-in-comment-keep` |
| `muse-probe-audit2-108` | `iftags/spec-comment-straddle-keep` |
| `muse-probe-audit2-109` | `iftags/spec-fused-close-keep` |
| `muse-probe-audit2-11` | `blockquote/iftags-kept-depth-two-sibling-quote` |
| `muse-probe-audit2-110` | `iftags/spec-segment-inner-gate-keep` |
| `muse-probe-audit2-111` | `iftags/spec-straddle-bracket-keep` |
| `muse-probe-audit2-112` | `iftags/trailing-blank` |
| `muse-probe-audit2-113` | `iftags/two-gates-lines` |
| `muse-probe-audit2-114` | `iftags/whitespace-only-body` |
| `muse-probe-audit2-115` | `iftags/whitespace-only-body-run` |
| `muse-probe-audit2-116` | `iftags/whole-lines` |
| `muse-probe-audit2-117` | `image/iftags-seam-name` |
| `muse-probe-audit2-118` | `image/subname-iftags-seam` |
| `muse-probe-audit2-119` | `image/subname-iftags-seam-whitespace` |
| `muse-probe-audit2-12` | `blockquote/iftags-kept-empty-first-segment-no-blockquote` |
| `muse-probe-audit2-120` | `italics/split-close-segment` |
| `muse-probe-audit2-121` | `link/iftags-seam` |
| `muse-probe-audit2-122` | `link/iftags-seam-trailing` |
| `muse-probe-audit2-123` | `math/eref-head-comment-seam` |
| `muse-probe-audit2-124` | `math/eref-head-segment-seam` |
| `muse-probe-audit2-125` | `math/iftags-seam-name` |
| `muse-probe-audit2-126` | `size/head-segment-seam` |
| `muse-probe-audit2-127` | `table/iftags-seam-fused-close` |
| `muse-probe-audit2-128` | `table/simple-split-close-iftags-excess` |
| `muse-probe-audit2-129` | `tabview/iftags-seam` |
| `muse-probe-audit2-13` | `blockquote/iftags-kept-empty-row-splits-paragraph` |
| `muse-probe-audit2-130` | `tabview/iftags-seam-abutting` |
| `muse-probe-audit2-131` | `tabview/iftags-seam-fused-close` |
| `muse-probe-audit2-132` | `tabview/iftags-seam-name` |
| `muse-probe-audit2-133` | `toc/iftags` |
| `muse-probe-audit2-134` | `user/iftags-seam` |
| `muse-probe-audit2-135` | `user/iftags-seam-mid-name` |
| `muse-probe-audit2-14` | `blockquote/iftags-kept-escaped-quote-with-inner-break` |
| `muse-probe-audit2-15` | `blockquote/iftags-kept-escapes-after-break` |
| `muse-probe-audit2-16` | `blockquote/iftags-kept-escapes-unquoted-continuation` |
| `muse-probe-audit2-17` | `blockquote/iftags-kept-quoted-segment-becomes-sibling` |
| `muse-probe-audit2-18` | `blockquote/iftags-kept-rows-form-blockquote` |
| `muse-probe-audit2-19` | `blockquote/iftags-kept-shallower-row-joins-run` |
| `muse-probe-audit2-2` | `list/iftags-body-escape` |
| `muse-probe-audit2-20` | `blockquote/iftags-seam-line-start` |
| `muse-probe-audit2-21` | `bold/split-close-literal` |
| `muse-probe-audit2-22` | `bold/split-close-segment-body` |
| `muse-probe-audit2-23` | `bold/split-opener-segment-body` |
| `muse-probe-audit2-24` | `button/head-segment-seam` |
| `muse-probe-audit2-25` | `date/subname-iftags-seam-whitespace` |
| `muse-probe-audit2-26` | `definition-list/iftags-body-escape` |
| `muse-probe-audit2-27` | `file/iftags-seam` |
| `muse-probe-audit2-28` | `footnote/block-iftags-seam-name` |
| `muse-probe-audit2-29` | `footnote/iftags-seam-name-kept-body` |
| `muse-probe-audit2-3` | `list/iftags-body-escape-empty-item` |
| `muse-probe-audit2-30` | `footnote/iftags-seam-name-whitespace-body` |
| `muse-probe-audit2-31` | `gallery/entry-segment-in-url` |
| `muse-probe-audit2-32` | `gallery/iftags-seam-end-name` |
| `muse-probe-audit2-33` | `gallery/iftags-seam-fused-open` |
| `muse-probe-audit2-34` | `gallery/iftags-seam-fused-open-entries` |
| `muse-probe-audit2-35` | `gallery/iftags-seam-name` |
| `muse-probe-audit2-36` | `gallery/iftags-seam-name-args` |
| `muse-probe-audit2-37` | `gallery/iftags-seam-name-current-page-files` |
| `muse-probe-audit2-38` | `gallery/segment-after-arguments` |
| `muse-probe-audit2-39` | `gallery/segment-before-arguments` |
| `muse-probe-audit2-4` | `list/iftags-body-escape-ends-run` |
| `muse-probe-audit2-40` | `gallery/segment-inside-argument-value` |
| `muse-probe-audit2-41` | `iftags/body-div-literal` |
| `muse-probe-audit2-42` | `iftags/body-heading-crossed-closer` |
| `muse-probe-audit2-43` | `iftags/body-heading-then-text` |
| `muse-probe-audit2-44` | `iftags/body-paragraph-break-around` |
| `muse-probe-audit2-45` | `iftags/body-paragraph-break-in-span` |
| `muse-probe-audit2-46` | `iftags/body-paragraph-breaks` |
| `muse-probe-audit2-47` | `iftags/closer-midline` |
| `muse-probe-audit2-48` | `iftags/comment-close-false` |
| `muse-probe-audit2-49` | `iftags/comment-close-true` |
| `muse-probe-audit2-5` | `list/iftags-body-escape-single-line` |
| `muse-probe-audit2-50` | `iftags/empty-midline` |
| `muse-probe-audit2-51` | `iftags/fused-close` |
| `muse-probe-audit2-52` | `iftags/kept-bold-close-inside` |
| `muse-probe-audit2-53` | `iftags/kept-content-space-merge` |
| `muse-probe-audit2-54` | `iftags/kept-empty-first-segment-mixed` |
| `muse-probe-audit2-55` | `iftags/kept-empty-indented-closer` |
| `muse-probe-audit2-56` | `iftags/kept-gate-bibliography-close-inside` |
| `muse-probe-audit2-57` | `iftags/kept-gate-center-close-inside` |
| `muse-probe-audit2-58` | `iftags/kept-gate-collapsible-close-inside` |
| `muse-probe-audit2-59` | `iftags/kept-gate-div-close-inside` |
| `muse-probe-audit2-6` | `span/iftags-seam` |
| `muse-probe-audit2-60` | `iftags/kept-gate-footnote-close-inside` |
| `muse-probe-audit2-61` | `iftags/kept-gate-list-close-inside-unclosed-item` |
| `muse-probe-audit2-62` | `iftags/kept-gate-list-item-close-inside` |
| `muse-probe-audit2-63` | `iftags/kept-gate-math-body-splice` |
| `muse-probe-audit2-64` | `iftags/kept-gate-math-close-inside` |
| `muse-probe-audit2-65` | `iftags/kept-gate-math-nested-crossed` |
| `muse-probe-audit2-66` | `iftags/kept-gate-math-nested-discarded-literal` |
| `muse-probe-audit2-67` | `iftags/kept-gate-math-nested-inner-literal` |
| `muse-probe-audit2-68` | `iftags/kept-glued-body-space` |
| `muse-probe-audit2-69` | `iftags/kept-heading-segment-wraps-text` |
| `muse-probe-audit2-7` | `span/iftags-seam-attribute-name` |
| `muse-probe-audit2-70` | `iftags/kept-hr-segment-wraps-both-sides` |
| `muse-probe-audit2-71` | `iftags/kept-indented-closer` |
| `muse-probe-audit2-72` | `iftags/kept-indented-closer-blank-after` |
| `muse-probe-audit2-73` | `iftags/kept-indented-closer-doc-end` |
| `muse-probe-audit2-74` | `iftags/kept-indented-closer-glued-after` |
| `muse-probe-audit2-75` | `iftags/kept-indented-closer-space-after` |
| `muse-probe-audit2-76` | `iftags/kept-indented-closer-tab` |
| `muse-probe-audit2-77` | `iftags/kept-indented-opener` |
| `muse-probe-audit2-78` | `iftags/kept-list-segment-wraps-trailing-text` |
| `muse-probe-audit2-79` | `iftags/kept-mixed-segment-wraps-text` |
| `muse-probe-audit2-8` | `anchor/iftags-seam-trailing` |
| `muse-probe-audit2-80` | `iftags/kept-mixed-segment-wraps-trailing-text` |
| `muse-probe-audit2-81` | `iftags/kept-para-tail-indented-closer` |
| `muse-probe-audit2-82` | `iftags/kept-spaced-head` |
| `muse-probe-audit2-83` | `iftags/kept-spaced-head-blank-before` |
| `muse-probe-audit2-84` | `iftags/kept-spaced-head-doc-start` |
| `muse-probe-audit2-85` | `iftags/kept-spaced-head-multiline` |
| `muse-probe-audit2-86` | `iftags/kept-unclosed-child-code` |
| `muse-probe-audit2-87` | `iftags/kept-unsafe-first-segment-then-text` |
| `muse-probe-audit2-88` | `iftags/lines-between-text` |
| `muse-probe-audit2-89` | `iftags/lines-in-span` |
| `muse-probe-audit2-9` | `blockquote/iftags-kept-blank-line-new-run-shallower` |
| `muse-probe-audit2-90` | `iftags/list-item-after-closer-joins` |
| `muse-probe-audit2-91` | `iftags/list-item-after-paragraph-break` |
| `muse-probe-audit2-92` | `iftags/list-item-inside-span-stays-nested` |
| `muse-probe-audit2-93` | `iftags/list-item-joins-from-second-item` |
| `muse-probe-audit2-94` | `iftags/list-item-line-at-body-start` |
| `muse-probe-audit2-95` | `iftags/list-items-close-definition-list` |
| `muse-probe-audit2-96` | `iftags/list-items-join` |
| `muse-probe-audit2-97` | `iftags/list-items-join-ordered` |
| `muse-probe-audit2-98` | `iftags/list-items-split-on-marker-type` |
| `muse-probe-audit2-99` | `iftags/list-parabreak-starts-new-list` |
