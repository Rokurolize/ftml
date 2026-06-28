# Contributing

FTML work should be categorized before implementation and review. The two categories that are easiest to confuse are coverage work and Wikidot parity work.

## Coverage Work

Coverage work improves test breadth, public API exercise, or line and branch coverage without claiming that FTML now matches a concrete Wikidot behavior.

Coverage-only PRs should stay small, independently mergeable, and explicit about their scope. If a coverage refactor changes parser or renderer behavior, the PR must include a fixture regression check or a clear non-impact rationale.

## Wikidot Parity Work

Parity work changes FTML so parsing or rendering matches concrete Wikidot behavior. A parity claim needs fixture evidence: a source page, a specific behavior, and usually a `find test -name "wikidot.html"`-discovered fixture or a focused `tests/*_wikidot_syntax.rs` regression.

Do not bundle unrelated parity fixes into coverage attribution PRs. Do not add speculative syntax support just because a syntax exists somewhere; prioritize behavior observed in fixture pages and linked issues.

## Finding Parity Fixtures

Tree-test cases with `wikidot.html` are Wikidot layout parity assertions:

```sh
find test -name "wikidot.html" -exec dirname {} \;
```

Fixture-driven integration tests live under `tests/` and use names such as `tests/scp9506_wikidot_syntax.rs`. These tests should link back to the Wikijump issue or fixture source that proved the syntax gap.
