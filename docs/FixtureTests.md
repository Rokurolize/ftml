[<< Return to the README](../README.md)

## Fixture-Driven Syntax Tests

Fixture-driven syntax tests track FTML syntax gaps discovered from real Wikijump or Wikidot fixture pages. They exist to keep parser and syntax-representation work tied to concrete pages the user cares about, not to expand syntax support speculatively.

### Scope Boundary

FTML is responsible for parsing and syntax representation. Runtime execution belongs to Wikijump unless a specific FTML API already owns the behavior.

For delayed runtime constructs such as ListPages, FTML should preserve a structured delayed node and its body template. Do not move ListPages execution into FTML without an explicit architecture decision.

### Naming

Use `tests/<normalized_article_slug>_wikidot_syntax.rs` for article-driven integration tests. For hyphenated slugs, drop the hyphen in the Rust filename when the remaining name is readable, for example `scp-8980` becomes `tests/scp8980_wikidot_syntax.rs`. Use underscores where they improve readability, for example `tests/scp_anthology_2024_wikidot_syntax.rs`.

### Linkage

Every fixture-driven syntax regression should include a nearby comment that links the originating fixture issue or source. Use this form:

```rust
/// Fixture: Rokurolize/wikijump#17.
#[test]
fn scp8980_listpages_shape_is_preserved_as_delayed_node() {
    // focused parser or renderer assertion
}
```

If a fixture page does not currently expose an FTML syntax gap, list it in the index below but do not add an empty placeholder test.

### Fixture Drivers

| Driver | FTML test file | Current tracked FTML syntax gaps | Originating Wikijump issue(s) |
|---|---|---:|---|
| `scp-9506` | `tests/scp9506_wikidot_syntax.rs` | 2 | Rokurolize/wikijump#52, Rokurolize/wikijump#59, Rokurolize/wikijump#60 |
| `scp-8980` | `tests/scp8980_wikidot_syntax.rs` | 1 | Rokurolize/wikijump#7, Rokurolize/wikijump#17, Rokurolize/wikijump#18, Rokurolize/wikijump#19, Rokurolize/wikijump#20 |
| `theme:yossistyle` | `tests/yossistyle_wikidot_syntax.rs` | 1 | [Canonical Wikidot page](https://scp-wiki.wikidot.com/theme:yossistyle), read-only GET verified 2026-07-13 |
| `scp-3352` | Not added until a concrete FTML syntax gap is identified | 0 | Rokurolize/wikijump#4, Rokurolize/wikijump#15 |
| `scp-anthology-2024` | Not added until a concrete FTML syntax gap is identified | 0 | Rokurolize/wikijump#8, Rokurolize/wikijump#23 |

When adding a new fixture driver, add the Wikijump issue link here first, then add only the FTML tests required by proven parser or syntax-representation gaps.
