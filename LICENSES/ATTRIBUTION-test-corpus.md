# Attribution: wiki content in the `test/` fixture corpus

The `test/` directory holds this project's parser fixture corpus. Most fixtures were written by project contributors for this repository and are covered by the project license (the GNU Affero General Public License, see `LICENSE.md` in the repository root).

Some fixtures are instead derived from pages on the SCP Foundation wiki (scp-wiki, https://scp-wiki.wikidot.com/) and the SCP-JP wiki (https://scp-jp.wikidot.com/), most visibly the `test/harvest-*` groups, which were harvested from real pages, and individual cases reduced from page sources elsewhere in the corpus. Content on those wikis is published by their contributor communities under the Creative Commons Attribution-ShareAlike 3.0 Unported license (CC BY-SA 3.0, https://creativecommons.org/licenses/by-sa/3.0/), and fixtures derived from wiki pages are licensed under CC BY-SA 3.0 rather than the project license. The full license text is in [`CC-BY-SA-3.0.txt`](CC-BY-SA-3.0.txt), alongside this file.

Fixtures derived from wiki pages were adapted for testing: page sources were cut down to the fragments that exercise the behaviour under test, and the expected outputs record how the live Wikidot renderer renders those fragments.

This notice lives outside `test/` because the fixture loader enumerates that directory and rejects any entry that is not a test case (see `src/test/ast/loader.rs`), so the corpus cannot carry the notice itself.

The fixtures `list/explicit-item-ends-at-barrier`, `table/quote-row-cell-blank-item-merge`, `footnote/explicit-list-ends-body-run` and `list/explicit-list-before-marker-list-no-break` are reduced from the same pool of live-measured Wikidot pages (records 6a23608dcca5, 9f5d4ae61d25, 94756254068f and 058cd6dbf85e): only structural marker syntax was kept, no page prose was carried into the fixtures; the expected outputs were captured via saved-page probes on the sandbox (probes muse-probe-f3rem-d-div-alone, muse-probe-f3rem-foot-list-boundary, muse-probe-f3rem-e2e-quote-marker-head).
