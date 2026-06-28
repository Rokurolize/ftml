## Classification

- [ ] Coverage improvement
- [ ] Wikidot parity fix
- [ ] Parser/rendering behavior change without a parity claim
- [ ] Documentation or workflow only

Coverage-only PRs should stay small and independently mergeable. Do not claim that a coverage PR fixes Wikidot fixture parity unless it includes direct fixture-driven syntax or rendering evidence.

## Parity Evidence

Required when "Wikidot parity fix" is selected:

- Fixture issue or PR:
- Fixture page(s):
- Fixture file(s), such as a `find test -name "wikidot.html"` result or `tests/*_wikidot_syntax.rs`:
- Concrete expected behavior:
- Concrete previous or observed failure:

Parity claims must link to real fixture evidence. A broad assertion that behavior is "more like Wikidot" is not enough.

## Parser Behavior Impact

- [ ] No parser changes
- [ ] Parser/rendering changes with fixture regression verified
- [ ] Parser/rendering changes with explicit non-impact rationale

If a coverage refactor changes parser or renderer behavior, include either a fixture regression check or an explicit non-impact note.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo test --all-features --lib`
- [ ] `cargo test --all-features --tests`
- [ ] `RUSTFLAGS="-A unused -D warnings" cargo clippy --tests --no-deps`
- [ ] Other:
