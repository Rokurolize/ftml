# Wikidot Compatibility Hazards

FTML's `Layout::Wikidot` is a compatibility implementation. Deterministic, FTML-owned live Wikidot behavior is the authority even when reproducing it is less defensive than a modern implementation would normally choose. Security hardening belongs in a separate caller or deployment boundary unless the hardening is itself part of the observed Wikidot contract.

This is the same constraint that applies to an emulator: replacing an old machine's observable behavior with a safer or more modern behavior is still an emulation bug. A concerning Wikidot behavior should therefore be documented here and covered by provenance-backed live evidence, not silently changed while claiming parity.

## Observed hazards reproduced for parity

- Wikidot anchor blocks preserve dangerous-looking `href` schemes after legacy normalization. For example, `[[a href="javascript:alert(1)"]]` renders an `href` of `javascript:alert-1`. Frozen evidence: `test--anchor--xss` in `references-20260815-01.jsonl`.
- Wikidot new-tab links emit `target="_blank"` without adding `rel="noopener noreferrer"`. Frozen evidence: `test--link--single` in `references-20260815-04.jsonl` and the starred-link references in later parity batches.
- A valid comment can join a CSS value into `javascript:` and Wikidot preserves that style value. Frozen evidence: `comment-style-javascript-seam` and `comment-style-unicode-javascript-seam` in the 2026-08-16 parity references.
- Wikidot accepts a backslash in a `set-tags` alteration and places the authored backslash directly in the generated JavaScript handler string. Frozen evidence: `wikidot-button-set-tags-backslash` in the button parity references.
- Wikidot user karma images use an `http://www.wikidot.com/userkarma.php` URL. `Layout::Wikidot` preserves that transport choice rather than silently upgrading it to HTTPS.

These observations are compatibility requirements for `Layout::Wikidot`. They are not recommendations for new application code. Callers that need stronger isolation should enforce it outside the Wikidot compatibility renderer so that the compatibility surface remains testable against the live oracle.
