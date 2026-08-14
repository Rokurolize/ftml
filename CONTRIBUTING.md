# Contributing

FTML work should be categorized before implementation and review. The two categories that are easiest to confuse are coverage work and Wikidot parity work.

## Coverage Work

Coverage work improves test breadth, public API exercise, or line and branch coverage without claiming that FTML now matches a concrete Wikidot behavior.

Coverage-only PRs should stay small, independently mergeable, and explicit about their scope. If a coverage refactor changes parser or renderer behavior, the PR must include a fixture regression check or a clear non-impact rationale.

## Wikidot Parity Work

Parity work changes FTML so parsing or rendering matches concrete Wikidot behavior. A parity claim needs raw, provenance-backed Wikidot reference JSONL as its independent oracle. A generated `wikidot.html` file is only a derived local `Layout::Wikidot` regression snapshot; it must never be cited as evidence for itself.

Do not bundle unrelated parity fixes into coverage attribution PRs. Do not add speculative syntax support just because a syntax exists somewhere; prioritize behavior observed in fixture pages and linked issues.

The checked-in case inventory and bindings are the machine authority for the stable corpus. Keep caller-owned runtime cases in that inventory, but verify behavior that needs site, page, actor, permission, query, import, file, or browser state in Wikijump or another caller.

## Parity Workflow

Follow the regenerate, capture, compare, bind, report, and promotion workflow in [docs/ParityTests.md](docs/ParityTests.md). Capture and comparison use the adjacent Wikijump verification toolkit and are never part of ordinary FTML tests. Ordinary tests are offline.

Only promote a `wikidot.html` snapshot when its binding status is `match`. `FTML_UPDATE_TESTS=1` may generate the derived snapshot, but update mode alone is not evidence and deliberately cannot pass CI.

## Finding Local Parity Snapshots

Tree-test cases with `wikidot.html` are derived Wikidot-layout regression snapshots:

```sh
find test -name "wikidot.html" -exec dirname {} \;
```

Fixture-driven integration tests live under `tests/` and use names such as `tests/scp9506_wikidot_syntax.rs`. These tests should retain their raw evidence provenance or link to the concrete source that proved the syntax gap.
