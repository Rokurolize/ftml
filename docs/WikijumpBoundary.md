[<< Return to the README](../README.md)

## Wikijump Responsibility Boundary

The frozen responsibility boundary between FTML and its primary consumer Wikijump is specified in the Wikijump repository at `docs/ftml-boundary.md` (`https://github.com/Rokurolize/wikijump/blob/develop/docs/ftml-boundary.md`). That document is the contract of record; this page is a pointer plus the FTML-side obligations it implies.

### What FTML owns under that contract

- Tokenization, parsing, and AST representation for Wikidot/FTML syntax, including malformed-but-real Wikidot shapes backed by corpus evidence.
- Syntax-level HTML rendering for both `Layout::Wikijump` and `Layout::Wikidot`, including DOM shape for constructs such as tabview, footnotes, bibliography, collapsible, code, and math.
- Escaping and sanitization at the syntax render boundary.
- Wikidot comment semantics (`[!-- --]`), including the visibility of tokens inside comments during the include scan.
- Structured preserved/delayed representations for runtime constructs (includes, ListPages, CountPages, unknown modules, conditional blocks) rather than erasing or approximating them.
- Exact authored `[[embedvideo]]` ownership as an opaque `EmbedVideo` AST value. HTML output exposes an `EmbedVideoRequirement` containing the exact source, payload, and SHA-256 identity while rendering only a static marker.
- Exact authored `[[gallery]]` ownership as a typed `Gallery` AST value. HTML output exposes a `GalleryRequirement` with ordered head arguments, current-page or explicit-entry selection, exact source slices, and SHA-256 identity while rendering only a static marker.
- A parser performance envelope where dense real corpus pages parse within production budget or fail in a structured, caller-detectable way.

### What FTML must not absorb

Site-runtime semantics stay in Wikijump or the caller: page/site/user lookup, permissions, DB queries, include source fetching, ListPages/CountPages execution, Gallery current-page file selection, Gallery image URL materialization and viewer behavior, `[[embedvideo]]` provider recognition and iframe sanitization, module runtime data, browser capture, and fidelity validation. See `AGENTS.md` in this repository and the Wikijump contract document for the full split.

### Consumer coupling to be aware of

- Wikijump currently regex-rewrites some FTML output (`wj-code`, `wj-tabs`, `wj-footnote`, `wj-math` DOM shapes) into Wikidot DOM as a temporary measure. Those exact shapes are a temporary informal contract: changing them is allowed but is expected to surface in Wikijump's pin-bump canary, so record such changes in the PR body per `AGENTS.md`.
- Wikijump gates every FTML dependency bump on a marker-contract canary (golden-pair render comparison). Parser-visible behavior changes, especially around canonical Wikidot source markers, should call this out explicitly for the consumer.
- The first planned boundary migration into FTML is comment-aware include scanning; see the Wikijump contract document for the staged plan.

### Caller-owned module and include rendering (wikijump#2997, #3040, #3073, #2872)

These module and include behaviors need data FTML never receives, so they are not implemented here:

- **Failed-include notice** (wikijump#2997): FTML only calls `Includer::no_such_include` (`src/includes/mod.rs`) and splices the returned wikitext. The notice text, including any "edit component" link, is produced by the caller's `Includer` implementation. FTML's own implementations are `NullIncluder` (empty) and the test/debug includers. The frozen parity corpus holds no Wikidot failed-include markup to match.
- **`[[module Redirect]]` disabled-state copy** (wikijump#3040): there is no Redirect module rule. In `Layout::Wikidot` a bodiless `[[module Redirect]]` on its own physical line falls through to `wikidot_unknown_module_error` (`src/parsing/rule/impls/block/blocks/module/rule.rs`). Whether a redirect is disabled depends on the viewer's edit rights, which `PageInfo` does not carry.
- **`[[module Rate]]` "rating disabled"** (wikijump#3073): `Module::Rate` carries no state, and `PageInfo` has only `score`, with no rating-enabled flag. Output comes from `Handle::render_module` (`src/render/handle.rs`), which the caller supplies.
- **`[[module TagCloud]]` limit** (wikijump#2872): FTML has no TagCloud rule and no default limit of 20, and it never sees the site's tags. The limit and any truncation notice belong to the caller's implementation.
