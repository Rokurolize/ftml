# FTML open-issue remediation plan — 2026-08-18

## Purpose

This plan covers every GitHub issue that was open in `Rokurolize/ftml` when the issue set was fetched with `gh` on 2026-08-18, including every comment present on each issue at that fetch. It is an execution plan, not a claim that any issue is already fixed.

The remediation order is deliberately dependency-driven. Safety-first repair precedes changes that widen accepted input. Lexical ownership repair precedes owner-specific patches that would otherwise duplicate whitespace, comment, control-byte, or delimiter behavior. Structural owner repair precedes removal of scan windows. Expression compatibility repair preserves typed values and diagnostics before its resource limits are relaxed. Caller-runtime disposition remains outside static FTML parsing.

## Skill decision

`ask-matt` routes a pile of incoming bugs and requests through `triage`, so `triage` is the primary skill for the present planning pass. The repository already has `docs/agents/issue-tracker.md`, so tracker setup is not needed. `semantic-generation` applies because this file is a remediation plan; its required referent artifact is `docs/development/referent-table-open-issue-remediation-plan.md` with the adjacent SHA-256 file. When implementation is authorized, use `implement` with TDD at the public seams below. The installed `code-review` skill requires parallel sub-agents, which are not available in this environment; its Standards and Spec axes will therefore be performed as two separate solo review passes rather than pretending the unavailable runner was used.

## Authoritative snapshot and checkout state

- Repository: `Rokurolize/ftml`.
- Open issues fetched through `gh`: 135.
- Open issues containing comments: 50.
- Comments read: 138.
- Current-open issues with a recorded reopen event: 15.
- Label state at the fetch: 27 `bug`, 3 `needs-triage`, 43 `ready-for-agent`, 62 unlabeled. Labels are therefore not reliable enough to drive implementation order by themselves.
- Source identity inspected while planning: commit `01ee219df84febb20cded87e7383a0ad0f50b6ad`, tree `63f937ee3fb29e9ed93c1afdabf07c44194b1df5`.
- Current checkout branch observed while planning: `coverage/wikidot-parity-evidence-20260817`.
- The checkout already contains substantial modified and untracked parity-discovery/evidence work under `scripts/` and `tests/fixtures/`. That state predates this plan and must be preserved. No reset, clean, mass checkout, or broad staging is allowed. The only intentional repository additions from this planning pass are this plan, its referent table, and the referent-table hash.

The issue set is mutable. Immediately before implementing an issue, run `gh issue view <number> --comments --json number,title,body,labels,comments,updatedAt` and merge any newer acceptance evidence into the active slice. Immediately before closing an issue, repeat that read so a late comment cannot be silently missed.

## Reopened issues: what changed after the earlier closure

The reopen events are material evidence that an earlier local approximation was too narrow. These cases must be tested against the newest witness, not merely against the behavior that justified the earlier closure.

| Issue | New evidence that controls the next fix |
| --- | --- |
| #293 | The uppercase-final-host-label heuristic is false: live accepts an image link target such as `https://example.COM/target`. Keep image-link safety work such as #605 owner-specific instead of inferring a host-casing ban. |
| #297 | A 35/36 malformed-region boundary exposes the global `ConditionalScanBudget` suppressing an independent later valid parser function. New literal-owner probes also show parser-function suppression ranges are wider than the real `code`/raw/HTML-code owners. |
| #304 | Case variation bypasses the lowercase-only compatibility helper and changes quote/container ownership. Fix the ownership rule, not a case-specific source rewrite. |
| #318 | Crossed bibliography rollback changes whether the first item commits and therefore changes downstream numbering and `bibcite` resolution. Mutable bibliography metadata must participate in commit/rollback. |
| #319 | Live code bodies delete NUL/VT/FF where the current code path substitutes or preserves them. Code byte normalization is now part of the issue, in addition to code-block ownership/recovery. |
| #331 | Live preserves a safe button style such as `width:100px` that the typed button style filter currently drops. The action descriptor and the style allowlist must remain separate concerns. |
| #339 | NBSP around a color field is authored content, not ignorable padding. Rust Unicode `trim()` is broader than the live field grammar. |
| #340 | Definition term/value edge NBSP is preserved by live; only the evidenced ASCII edge space is trim syntax. |
| #342 | Quoted/structural file path bytes and NBSP fields remain authored attachment data in live behavior. Preserve typed untrusted identity rather than normalizing into broader authority. |
| #343 | A leading NBSP prevents horizontal-rule ownership. This depends on the shared leading-non-ASCII-space repair in #547; trailing-space behavior remains owner-specific. |
| #351 | Invalid named-anchor recovery currently trims edge NBSP that live preserves in fallback text. |
| #353 | Inline math formula edge NBSP is significant; current Unicode-wide trimming changes formula bytes. |
| #476 | The repository's compatibility policy now intentionally reproduces the live named-anchor `href` normalization and documents the hazard. The new NBSP-padded witness still mismatches, so the compatibility exception must be fixed only at that explicit anchor-block seam. |
| #660 | It was closed as overlapping #639, then reopened when a duplicate-`tz` head showed a distinct live recovery: live ignores the malformed/duplicate timezone input while FTML applies the last value. Keep #639 and #660 as one implementation area with separate regressions. |
| #668 | Concurrent duplicate triage briefly created a #668/#670 duplicate cycle. #668 is now the canonical open owner for the shared `get_head_name_map_wikidot` / `get_block_name_internal` Unicode-`trim()` NBSP defect. |

## Execution invariants

1. **Independent oracle first.** A Wikidot parity change starts from the frozen raw reference and public `Layout::Wikidot` behavior. A derived `wikidot.html` snapshot is never evidence for itself.
2. **Red at the public seam.** For a functional mismatch, first make or identify a public parse/render regression that fails on the current implementation and is backed by the issue's frozen reference. Internal helper tests may accompany it but may not replace it.
3. **Safety before permissiveness.** Parser panics, active-syntax provenance leaks, unsafe resource admission, recursive rendering, and superlinear scans are repaired before any FTML-only length/depth/count cap is raised or removed.
4. **One root, many witnesses.** When several issues exercise one implementation defect, make one shared implementation change and keep discriminating owner-specific regressions. Do not duplicate source rewrites per fixture.
5. **Preserve rejected source.** A rejected owner returns the exact authored/residual bytes to downstream grammar wherever the live behavior does. Generated fallback text must not silently gain or lose syntax authority.
6. **Typed data is not authority.** Attachment paths, resource descriptors, user/page lookups, gallery selections, and embed providers remain typed requests until the caller authorizes or resolves them.
7. **No timing-only security proof.** Complexity fixes should be structurally linear or cached by construction and tested with deterministic scan/work invariants where practical. Wall-clock thresholds are only a secondary smoke check.
8. **No snapshot promotion on mismatch.** Follow `docs/ParityTests.md`: only promote a changed/new local snapshot after the frozen comparator reports `match`.

## Public seams to use for TDD

Approval of this plan defines the intended seams for the later TDD cycles.

- Primary compatibility seam: public `Layout::Wikidot` parse/render output for the frozen fixture source, compared against provenance-backed raw Wikidot JSONL.
- Parser safety seam: public parse/render of adversarial source, asserting no panic, bounded work, preserved output ownership, and mode gates.
- Serialized-AST safety seam: public renderer entry point for #609, because the exploit enters after parsing.
- Workflow seam: the actual CI gate script/workflow for #616, exercised with completed and in-progress check data.
- Caller-runtime seam: Wikijump or the appropriate caller integration for #506, #507, #550 and caller-dependent subcases; static FTML output is only the typed request contract.

## Package 1 — Safety-first repair

Canonical issues: **#589, #590, #591, #592, #593, #594, #595, #596, #597, #598, #599, #600, #601, #602, #603, #604, #605, #606, #608, #609, #610, #611, #612, #613, #614, #615, #616**.

### 1A. Remove crashes, side-effect leaks, and authority creation

- #590: replace the parser-wide residual gallery flag assumption with state whose lifetime is scoped to the parse owner; nested no-paragraph parsing must not leave a stale `MIXED_PARAGRAPH_OWNERSHIP` bit that can panic later.
- #597: make CSS close recognition use one shared closer predicate and return an ordinary malformed/recovery result for user input; remove the `expect` path that can be reached by a whitespace-spaced close.
- #601: speculative div recovery must not share mutable footnote/HTML-block side effects with the committed parser state. Prefer eliminating side-effecting speculation; otherwise snapshot and rollback all mutable state, not only the cursor.
- #602: only authored block-name provenance may create an EmbedVideo resource requirement. Runtime/generated text can remain text but cannot forge active block authority.
- #604: enforce `enable_page_syntax` before standalone-button action parsing so disabled page syntax cannot still create an action.
- #609: add render recursion/cycle protection for indexed footnotes in serialized AST input.
- #610: make `iftags` remaining-close caching exact for the branch decision; an early-stop cache may not undercount and expose hidden content.

Completion criterion: each exploit witness is red before its fix, then green through the public seam; no panic/active action/resource/recursive render remains for the reported input.

### 1B. Make adversarial scans linear before widening limits

- #589: stop rescanning the growing multiline standalone-button head with `is_set_tags_head`; classify incrementally or after one bounded head acquisition.
- #591: remove repeated suffix scans for spaced unmatched block closes.
- #592: replace repeated right-angle-quote run regex/count work with one linear pass.
- #593: stop cloning/re-running the raw grammar from every triple-link label probe; memoize or stream owner boundaries.
- #594: avoid repeated line-prefix `rfind` work for rejected/duplicate footnoteblocks.
- #595: stop malformed single-link math-owner probing at the physical line boundary that already invalidates the label.
- #596: replace per-opener full owner prescans in single-link recovery with cached/streamed owner facts.
- #598: make malformed `[[/math` typography-range collection a one-pass state machine.
- #599: reuse maintained line-start state instead of `source[..opener].rfind('\n')` for every line-owner probe.
- #600: maintain bracket-run length/parity while tokenizing rather than scanning backward for every bracket.
- #606: build an indexed/streaming comment representation once and reuse it for attribute seam checks; this becomes a prerequisite for the comment-elision package.
- #608: index or memoize unterminated Wikidot code-block candidate outcomes rather than rescanning the full suffix for each opener.

Completion criterion: each reported adversarial family has one regression whose work grows linearly or whose repeated suffix fact is demonstrably cached; the safety tests remain stable without relying on a tight elapsed-time threshold.

### 1C. Close security-imported correctness gaps

- #603 and #605 are resolved at owner-specific image URL/link boundaries: preserve live-compatible grammar classification while preventing unsafe active image sources and browser-authority backslash escapes.
- #611 marks note blocks non-paragraph-safe at the correct public element/container boundary.
- #612 fixes the document-start boundary for scored inline-div recovery without changing later-line behavior.
- #613 makes residual gallery close ownership CRLF-safe and AST-round-trip-safe.
- #614 aligns delayed bound suppression with synthesized `TypographyBoundary` nodes.
- #615 merges/normalizes protected literal/comment intervals before end-based lookup so overlapping ranges cannot expose tab replacement inside literals. This must land before the broader typography/literal work in Package 3.
- #616 makes the metadata PR gate select the appropriate completed check rather than treating its own in-progress run as the authoritative prior result.

Completion criterion: focused security regressions pass, then the existing security suite passes as a group before later packages are allowed to widen syntax inputs.

## Package 2 — Lexical ownership repair for comments and line starts

Canonical issues: **#544, #546, #547, #548, #549, #551, #552, #553, #554, #555, #556, #557, #558, #559, #561, #562**.

The shared implementation direction is to keep source spans/provenance and use the indexed comment facts from #606 rather than globally rewriting all source into one comment-free string.

- #555 is the broad delimiter witness: valid comments can be transparent while forming multi-character syntax, but raw/literal ownership remains a barrier. Implement the shared behavior once, then prove the individual delimiters.
- #544 applies comment removal inside attribute-name/value collection without quadratic rescans.
- #548/#549/#551/#552/#554/#556 cover block closer, single-link scheme, autolink scheme, image scheme, triple-link scheme, and email-token joining. These must use the correct owner collector, not a universal pre-join pass that would create syntax in literal owners.
- #553/#558 cover comment deletion across separator and line-continuation boundaries; preserve physical-line semantics where live does.
- #557/#561 are the counterweight: a leading or in-head comment does not universally grant start-of-line/code ownership. Physical-line ownership must be decided before a comment-created candidate is allowed to claim a line.
- #546 keeps content separators exact: unpadded four-or-more `=` works; leading/trailing ASCII SP/TAB remains authored text.
- #547 removes the global non-ASCII-leading-space rewrite. NBSP/U+2007 must remain a structural barrier for the evidenced heading/list/HR/definition/quote/table owners.
- #559 keeps the single-link adjacent-tab split semantics after comment elision.
- #562 removes comments inside already-owned positional date/iframe values without letting comments manufacture the owner name.

Completion criterion: all 16 issue witnesses pass through public `Layout::Wikidot`; the comment-heavy adversarial test from #606 remains linear; raw/code/literal controls prove the fix did not turn global comment stripping into new syntax authority.

## Package 3 — Whitespace, C0 controls, literal typography, and owner-local normalization

Canonical issues: **#339, #340, #342, #343, #351, #353, #476, #581, #582, #618, #619, #620, #625, #626, #627, #629, #630, #631, #632, #638, #646, #655, #668, #677, #690, #691**.

Use narrow helpers for evidenced byte classes instead of Rust Unicode-wide `trim()` or a global normalization pass. The likely reusable primitives are ASCII-edge trim, discarded-C0 deletion for a selected semantic field, literal/protected range lookup, and exact owner-name/closer extraction.

### 3A. Literal/protected regions and typography

- Build on the merged protected intervals from #615.
- #581/#582 protect generic/legacy attribute values plus link labels/targets from prose typography while preserving the live prose and monospace behavior.
- #618 applies the same literal-region exclusion to number-space, unit-space, double-quote, and single-quote typography inside `[[code]]` and `@@...@@` bodies.
- #619 gives raw bodies the live four-column tab expansion while preserving one-space prose/monospace and four-column code behavior.
- #620 fixes only the terminal authored U+FFFD marker class; keep mid-line drop and own-line marker behavior intact.

### 3B. C0 deletion at semantic owner boundaries

- #625 deletes discarded C0 bytes inside active raw bodies without changing delimiters or other raw bytes.
- #626 deletes C0 controls before autolink, single-link, and triple-link target recognition while preserving URL safety.
- #627 deletes C0 bytes in inline math source; block math controls remain unchanged.
- #629 deletes C0 bytes from explicit single/triple link labels without changing target normalization.
- #630 normalizes image `link=` C0 bytes before target classification.
- #631 normalizes positional iframe/image URL/source C0 bytes and preserves its image-source leading-NBSP witness; the iframe NBSP owner remains #668.
- #632 deletes C0 controls from standalone-button `text=` without altering `set-tags` alteration parsing or JavaScript escaping.

### 3C. Exact whitespace/entity owner grammar

- #339, #340, #342, #351, #353, #646, #655 replace Unicode-wide trim behavior with the live owner-specific ASCII grammar for color, definition lists, file fields, anchor recovery labels, inline math, link labels, and size arguments.
- #343 consumes the shared leading-NBSP barrier from #547 and retains horizontal-rule-specific trailing grammar.
- #476 applies the documented named-anchor compatibility normalization at that explicit owner only; it is not a generic permission to weaken URL safety.
- #638 requires a complete column-zero clear-float physical line; an indented marker or marker followed by authored text stays source.
- #668 removes `get_block_name_internal`'s Unicode-wide post-collection trim for the affected date/iframe positional head, preserving ASCII controls.
- #677 rejects whitespace-padded body closers, including ordinary ASCII trailing space and NBSP, while preserving exact case-insensitive closers where live supports them.
- #690 fixes entity-decoding order only in the evidenced block-head fields. Do not pre-decode globally, because entity decoding can manufacture URL schemes or syntax.
- #691 implements the measured owner-specific leading-space matrix before action/resource authority is created. Gallery/image/math/bibliography/date/eref/footnote/iframe/size/toc/file/button reject the measured leading-space form; collapsible remains a positive control.

Completion criterion: all 26 canonical witnesses pass, plus the shared controls named in their latest comments. The implementation contains no new Unicode-wide trim used as a Wikidot grammar oracle and no global entity/control normalization that can create active syntax.

## Package 4 — Structural owner repair and transactional recovery

Canonical issues: **#304, #318, #319, #567, #574, #575, #673, #674, #675, #676, #685**.

- #304 fixes blockquote-to-unquoted/container ownership at the real line/owner seam; case changes must not determine whether a compatibility rewrite happens.
- #318 makes bibliography item numbering/metadata transactional with the owner commit. A crossed candidate that live commits must increment; a rolled-back candidate must not leak metadata.
- #319 gives code blocks one owner-aware byte normalization path and preserves body/nesting/crossed-closer rollback. Reuse the linear unterminated-code work from #608 rather than adding a new scan.
- #567 changes invalid same-line block rollback so the rejected outer marker remains authored source while independently valid nested inline syntax is still eligible downstream.
- #574 preserves adjacent triple-link closing runs and named-anchor fallback ownership without swallowing or fabricating residual brackets.
- #575 uses the actual standalone physical-line close predicate for crossed code/math blocks; an inline/crossed close stays body text until a valid later close.
- #673 replaces whole-line raw masking in crossed bold/size compatibility with byte-range-local literal masking.
- #674 fixes root collapsible inline quoted-close case handling, trailing horizontal whitespace, and the following paragraph boundary in one helper rather than three source-specific rewrites.
- #675/#676 make the two measured crossed-recovery helpers case-insensitive where the underlying live owner is case-insensitive while retaining the independent cap work for later.
- #685 makes the shared block-head collector commit the first valid `]]` prefix and return every extra bracket as residual source across the evidenced owner matrix. The user-owner row stays caller-runtime for user existence, but the generic parser fix is proven by non-user owners.

Completion criterion: public render regressions show exact owner graph and residual source for every canonical issue; speculative paths do not leak mutable state after #601; no new whole-line literal mask or page-wide reparse is introduced.

## Package 5 — Parser-function source pipeline and recovery provenance

Canonical issues: **#297, #545, #560, #564, #565**.

This package fixes parser-function recognition/recovery before changing expression semantics.

- #297: replace the global conditional scan cliff with suffix facts or a work model in which malformed early candidates cannot starve an independent later valid candidate. Tighten literal suppression to the real live owners rather than malformed `[[ code]]`, malformed raw, or unrelated HTML-code lookalikes.
- #545: comment joining inside a parser-function-looking name must produce the live downstream recovery, not retroactively evaluate a joined function name.
- #560: preserve the distinction between a comment-elided parser-function lookalike and a function that actually owned its source.
- #564: unsupported/rejected parser functions must retain authored bracket provenance so ordinary single-link grammar can recover `#name` targets.
- #565: recognized `#if`/`#ifexpr` first-closer recovery must return authored residual brackets/source with the correct downstream link ownership rather than a semantically identical generated string with different authority.

Completion criterion: malformed-candidate count no longer suppresses later independent valid functions; public recovery DOM matches for all five issues; literal regions remain owner-local; scan work remains bounded without a small global candidate cliff.

## Package 6 — Expression compatibility repair

Canonical issues: **#570, #573, #640, #641, #642, #643, #666, #680, #681, #688, #692**.

The current evaluator collapses values to `f64` too early. Implement the smallest typed value model needed by the evidence—number, boolean, and null—plus structured error information. Do not build a general expression language beyond the open witnesses.

- #570: preserve recognized parser-function ownership and return deterministic live-compatible syntax/runtime diagnostics instead of converting broad evaluator failures into literal fallback.
- #573: require at least two arguments for `min`/`max` and preserve valid 2+ behavior.
- #640: implement live chained-comparison evaluation/associativity rather than stopping after one comparison.
- #641: characterize and implement the live fractional/negative remainder coercion instead of Rust `f64 %`.
- #642: replace the FTML-only `f64::EPSILON` equality rule with the live comparison rule.
- #643: replace fixed 11-decimal formatting with the measured normal/scientific serialization, including the new `1/3` 12-digit witness.
- #666: preserve boolean type through constants, comparisons, and logical results so direct `#expr` output is `true`/`false`, while arithmetic such as `true+1` remains numeric.
- #680: add right-associative `^` exponentiation with the measured unary precedence and operation accounting.
- #681: make ordinary built-in constants/functions case-sensitive and route wrong-case names through #570 diagnostics.
- #688: remove single `=` from equality grammar; preserve `==` and emit the live syntax error for the rejected form.
- #692: model `null`, including its observed case behavior, empty direct serialization, and equality, without treating unknown identifiers as null.

Completion criterion: the expression evaluator has typed results and typed failures sufficient to make all 11 issues green; arithmetic controls remain numeric; invalid expressions remain owned and diagnostic where live owns them; dense power/parenthesis inputs remain bounded.

## Package 7 — Owner-specific grammar, rendering, and typed resource semantics

Canonical issues: **#293, #331, #566, #621, #633, #634, #635, #636, #639, #647, #660, #678, #682, #683, #684, #686, #687, #689, #693**.

### 7A. Links, images, iframe, and typed attachment sources

- #293 removes the false uppercase-final-label heuristic while retaining the real image-link target and safety rules, including #605's backslash hardening.
- #621 adjusts only the measured autolink continuation/termination rule around `>@` while retaining `>>`, bare `>`, and punctuation controls.
- #647/#686 replace generic `is_url` as the iframe grammar oracle with the exact measured scheme admission/case-folding matrix. Preserve authored scheme spelling where live does; do not globally lowercase or broaden all URL consumers.
- #682 makes `mailto:`, `data:`, and `dns:` image source tokens follow the live attachment/direct-source classification, with the measured direct-scheme controls retained.
- #693 accepts the measured quoted structural attachment tokens as exact untrusted typed data. Quotes, slashes, encoded traversal spellings, query/fragment bytes, and page/file-looking text cannot become caller file authority merely because the parser classified them.

The security/parity split is explicit: #682/#693 define static source classification; #603 controls whether an unsafe direct source may become an active rendered resource. #476's documented dangerous-anchor compatibility exception remains limited to that anchor owner and is not generalized to image resources.

### 7B. Block argument and owner identity semantics

- #331 preserves typed safe button actions while extending only the live-backed button style properties such as `width`; #604's page-syntax gate stays in front of action creation.
- #566 accepts/ignores nameless assignments on valid span/div heads while retaining named attributes and normal div line ownership.
- #633 makes date `hover` use the measured literal-`true` grammar instead of generic loose booleans.
- #634 decouples Wikidot extended-year rendering from the `time` crate's ±9999 representable range without weakening timestamp lexical validation.
- #635 implements the measured precedence for numeric-seconds vs HH/MM timezone forms.
- #639 makes the three measured unusable/conflicting colon-form timezone cases non-fatal to an otherwise valid date owner.
- #660 handles duplicate timezone arguments as invalid/ignored for date rather than generic last-wins, while duplicate `format`/`hover` controls remain unchanged.
- #636 gives size its exact unsigned decimal lexical grammar, rejecting signs, exponent, and non-finite Rust `f64` spellings while preserving `.5`/`1.5` and supported units.
- #678 adds Wikidot owner-specific attribute retention for div, blockquote, explicit list/table, anchor, image, and the existing span path instead of exposing every globally safe HTML attribute such as `hidden`.
- #683 rejects an explicit leading `+` in the date timestamp lexeme while preserving `-0`, leading zeros, and ordinary negatives.
- #684/#687 make only the measured `eref` and `date` owner names case-sensitive; iframe/image/size and other measured case-insensitive owners remain controls.
- #689 fixes context-free malformed `iftags` bare-sign recovery without attempting to evaluate caller-owned page tags.

Completion criterion: all 19 canonical issues pass their frozen public witnesses and stated controls; generic URL/boolean/attribute/number parsers are no longer used as accidental Wikidot grammar oracles where the evidence shows owner-specific behavior.

## Package 8 — Syntax-budget repair after the parser is linear

Canonical issues: **#569, #571, #576, #577, #578, #579, #580, #583, #584, #585, #586, #587, #588, #607, #622, #623, #624**.

These issues are intentionally last among FTML-owned parser changes. Removing a visible cap before Packages 1, 4, and 5 eliminate superlinear scans would convert a compatibility bug into a denial-of-service regression.

- #569/#571: replace the 256-byte expression and 32-parenthesis syntax cliffs with bounded evaluator work/stack handling that accepts the frozen large valid expressions.
- #576/#607: remove 128/512-line div/center crossed-repair windows by using one forward owner scan or cached line-owner facts.
- #577: remove the 32-line center/collapsible prelude cliff using the same linear/cached ownership approach.
- #624: remove the separate 512-line collapsible/div crossed-repair window after #676 fixes its name predicate.
- #578/#588: remove the 64-byte scheme and 8 KiB single-link target compatibility cliffs while retaining safe label-only treatment for unknown schemes and the security rules for active hrefs.
- #579/#580: remove the 31-level native blockquote and 100-active-inline-scope syntax boundaries while retaining an internal stack/work guard that is not observable at the frozen boundary.
- #583: remove the 20-level list syntax cliff while preserving list ownership and bounded stack growth.
- #584: remove the 64 KiB button-head public cliff only after #589 makes head scanning linear.
- #585/#586/#587: remove parser-function nesting, unsupported-payload, and document-candidate cliffs only after Package 5 removes global starvation and repeated scans. Use explicit evaluator/scan work accounting rather than silently dropping the N+1 candidate.
- #622/#623: remove the 512-byte and 64-directive date-format public cliffs while keeping whatever validation is actually evidenced by live behavior.

Completion criterion: every N/N+1 frozen boundary pair behaves like live; adversarial cases remain bounded; no replacement constant is chosen merely because it is larger. If a safety guard must remain below a live-observed bound, document it as an intentional safety divergence rather than calling the issue fixed.

## Package 9 — Caller-runtime disposition

Canonical issues: **#506, #507, #550**.

These are real open compatibility observations, but they cannot honestly be fixed by inventing static FTML DOM because the missing inputs belong to callers.

- #506: EmbedVideo provider no-match behavior requires the caller/provider runtime. Keep FTML's typed request, verify the no-match error in the Wikijump integration, and close only when the caller contract is implemented and tested.
- #507: gallery empty-selection behavior needs current-page/file-service state. Keep typed gallery selection in FTML and verify the error/empty behavior in the caller.
- #550: `newpage` class depends on whether the target page exists. FTML should preserve target/label grammar; page existence and final class belong to the caller.

The mixed user witness noted by #685 follows the same rule for user existence: use non-user rows to prove the generic residual-bracket parser fix, then verify user lookup behavior at the caller seam.

Completion criterion: each issue has a passing caller integration test or an explicit cross-repository blocker with the exact required runtime input. Do not close one of these by snapshotting fabricated standalone data.

## Cross-package dependencies that must not be flattened

- #606 before #544-#562: comment elision must become indexed/streaming before comment transparency is broadened.
- #615 before #581/#582/#618/#619: protected intervals must be correct before more typography/whitespace passes rely on them.
- #601 before broad structural speculation changes: parser rollback must include mutable side effects.
- #547 before #343 and related leading-NBSP owner cases: remove the shared source rewrite once rather than adding HR/list/heading exceptions.
- #603/#605 and #293/#682/#693: image syntax classification, link-target normalization, rendered-resource safety, and caller attachment authorization are separate layers.
- #297/#545/#560/#564/#565 before #570-#573/#640-#643/#666/#680-#681/#688/#692: source ownership/recovery must be stable before evaluator semantics are judged.
- #589/#591-#600/#606/#608 before Package 8: visible hard caps cannot be safely widened while scans are superlinear.
- #639 and reopened #660 share the date timezone head but require separate acceptance rows: unusable colon forms versus malformed/duplicate timezone disposition.
- #668 is the canonical live issue after the #668/#670 duplicate cycle; do not reopen or bind work to closed #670.

## Per-package verification loop

For each package, run vertical slices rather than writing all tests and then all code:

1. Re-read the issue and latest comments with `gh`.
2. Identify the existing frozen reference and fixture; do not recapture when the source hash already has authoritative evidence.
3. Add or enable one failing public seam regression for one independent root branch.
4. Implement only enough to make that witness and its stated control pass.
5. Run the focused Rust integration/unit test file(s) for the touched owner.
6. Run the relevant parity ratchet, including `cargo test --test integration parity_index` where the stable inventory/binding surface changed.
7. Repeat for the next independent branch in the package.
8. Once the package is green, run `cargo test --test security_regressions` if any parser/preprocessor safety surface changed, then the broader offline Rust test suite before committing product changes.
9. For parity changes, run the regenerate/compare/bind/report flow from `docs/ParityTests.md` only for current-source references. Promote a local snapshot only after `status: "match"`.
10. Review the diff in two solo passes: Standards against `CONTRIBUTING.md` and nearby repository rules, then Spec against the exact issue bodies/comments in the package. Fix findings before closure.
11. Re-read every issue immediately before closing. Close only issues whose full newest acceptance criteria are satisfied; a shared code change may close several issues, but each gets its own evidence-backed closure statement.

## Commit and dirty-checkout discipline for the later execution

The current checkout is already dirty with parity discovery work. Implementation must use path-specific inspection and staging. Never use `git add -A`, `git reset --hard`, `git clean`, or checkout-wide restoration. Before each remediation commit, inspect `git diff -- <owned paths>` and stage only the files intentionally changed for that package. If pre-existing discovery changes touch a file that must also change for a fix, preserve them and review the combined hunk explicitly; do not silently overwrite or include them as though this plan created them.

Prefer one commit per coherent root fix or tightly coupled package slice, with issue numbers in the commit message. Do not create a single 135-issue mega-commit: the reopened issues show why reviewable root-scoped history matters.

## Coverage proof for this plan

The canonical issue lists in Packages 1 through 9 were checked against the `gh issue list --state open --limit 1000` snapshot. Their union contains **135 distinct issue numbers**, exactly matching the **135 open issues**, with **no missing and no duplicate canonical assignment**.

Canonical assignment counts: Package 1 = 27, Package 2 = 16, Package 3 = 26, Package 4 = 11, Package 5 = 5, Package 6 = 11, Package 7 = 19, Package 8 = 17, Package 9 = 3.

## Definition of completion

The campaign is complete only when all of the following are true at the then-current GitHub state:

- Every issue still open from this 2026-08-18 snapshot has either a verified FTML fix or, for genuinely caller-owned behavior, a verified caller integration resolution/blocker that the issue itself accepts.
- Every FTML-owned parity fix is green against its frozen independent Wikidot evidence at the public `Layout::Wikidot` seam.
- Every security finding is covered by a regression that demonstrates the reported panic, authority, recursion, state-leak, or complexity failure is gone.
- FTML-only observable caps in Package 8 are removed or explicitly retained as documented safety divergences; merely moving the constant upward does not count as a fix.
- Reopened issues pass the newest evidence that caused reopening, not only their earlier controls.
- Stable bindings/reporting are internally consistent, and no mismatching source has been promoted as a passing snapshot.
- The final product diff passes the repository's offline tests and the two-pass Standards/Spec review, and unrelated pre-existing checkout work remains intact.
