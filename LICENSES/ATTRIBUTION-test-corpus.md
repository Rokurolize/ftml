# Attribution: wiki content in the `test/` fixture corpus

The `test/` directory holds this project's parser fixture corpus. Most fixtures were written by project contributors for this repository and are covered by the project license (the GNU Affero General Public License, see `LICENSE.md` in the repository root).

Some fixtures are instead derived from pages on the SCP Foundation wiki (scp-wiki, https://scp-wiki.wikidot.com/) and the SCP-JP wiki (https://scp-jp.wikidot.com/), most visibly the `test/harvest-*` groups, which were harvested from real pages, and individual cases reduced from page sources elsewhere in the corpus. Content on those wikis is published by their contributor communities under the Creative Commons Attribution-ShareAlike 3.0 Unported license (CC BY-SA 3.0, https://creativecommons.org/licenses/by-sa/3.0/), and fixtures derived from wiki pages are licensed under CC BY-SA 3.0 rather than the project license. The full license text is in [`CC-BY-SA-3.0.txt`](CC-BY-SA-3.0.txt), alongside this file.

Fixtures derived from wiki pages were adapted for testing: page sources were cut down to the fragments that exercise the behaviour under test, and the expected outputs record how the live Wikidot renderer renders those fragments.

This notice lives outside `test/` because the fixture loader enumerates that directory and rejects any entry that is not a test case (see `src/test/ast/loader.rs`), so the corpus cannot carry the notice itself.
