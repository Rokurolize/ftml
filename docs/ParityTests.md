[<< Return to the README](../README.md)

## Wikidot Parity Test Reference

This document lists tree-test cases with `wikidot.html`. These files are the current canonical parity assertions for Wikidot layout rendering.

Update this list whenever a new `wikidot.html` fixture is added or removed. Keep descriptions short and tied to behavior visible in the fixture.

| Category | Test case | Parity assertion |
|---|---|---|
| anchor | `test/anchor/basic` | Anchor syntax renders the Wikidot-compatible anchor target shape. |
| audio | `test/audio/basic` | Audio blocks render expected Wikidot audio markup. |
| collapsible | `test/collapsible/wikidot-parity` | Collapsible blocks preserve folded state, labels, and button placement. |
| date | `test/date/agohover` | Date rendering preserves Wikidot ago-hover behavior. |
| date | `test/date/hover` | Date rendering preserves Wikidot hover text behavior. |
| date | `test/date/matrix` | Date formatting matrix matches Wikidot layout expectations. |
| date | `test/date/timezone` | Date rendering handles timezone inputs in Wikidot layout. |
| definition-list | `test/definition-list/basic` | Definition list syntax emits Wikidot-compatible definition markup. |
| div | `test/div/wikidot-nested-list-boundary` | Div blocks preserve nested explicit-list boundaries in Wikidot layout. |
| file | `test/file/wikidot-attachment` | Evidenced current-page file syntax emits a direct Wikidot attachment anchor. |
| image | `test/image/basic` | Image syntax, alignment, local-file paths, attributes, and linked images render in Wikidot layout. |
| link | `test/link/canonical-inline` | Inline link forms preserve their distinct label parsing behavior. |
| link | `test/link/single` | Single-bracket links render expected Wikidot href and label output. |
| link | `test/link/triple` | Triple-bracket links render expected Wikidot page and URL output. |
| list | `test/list/native-skipped-depth-empty-parent` | Native lists distinguish styled synthetic skipped-depth items from authored empty parents. |
| list | `test/list/native-wikidot-structure` | Native list nesting, mixed markers, run boundaries, and literal orphan rows match Wikidot. |
| misc | `test/misc/clear-float` | Clear-float syntax renders the Wikidot clear element. |
| misc | `test/misc/dash-edges` | Dash conversion preserves Wikidot behavior at ambiguous dash boundaries. |
| misc | `test/misc/email` | Email syntax renders Wikidot-compatible mail links. |
| radio | `test/radio/basic` | Radio inputs render expected Wikidot form markup. |
| raw | `test/raw/basic` | Raw inline or block content preserves Wikidot layout output. |
| raw | `test/raw/block` | Raw block content preserves Wikidot layout output. |
| table | `test/table/advanced` | Advanced table syntax renders Wikidot-compatible table structure. |
| table | `test/table/simple` | Simple table syntax renders Wikidot-compatible table structure. |
| user | `test/user/basic` | User references render expected Wikidot user markup. |
| video | `test/video/basic` | Video blocks render expected Wikidot video markup. |

Run this command to verify the current parity fixture set:

```sh
find test -name "wikidot.html" -exec dirname {} \; | sort
```
