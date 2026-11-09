# Attribution: wiki content in `verification/`

The verification harnesses in this directory (scripts and Rust sources) are project code, covered by the project license (the GNU Affero General Public License, see `LICENSE.md` in the repository root).

The evidence files kept alongside them (for example `verification/seam-typography/evidence/*.json`) are different in kind: they record wikitext inputs together with the rendered output measured on live Wikidot, captured from pages on the SCP Foundation wiki (scp-wiki, https://scp-wiki.wikidot.com/), the SCP-JP wiki (https://scp-jp.wikidot.com/), and project sandbox wikis hosted on Wikidot. Where a recorded input reproduces material from a wiki page, that material belongs to the wiki's contributor community and is licensed under the Creative Commons Attribution-ShareAlike 3.0 Unported license (CC BY-SA 3.0, https://creativecommons.org/licenses/by-sa/3.0/), not under the project license. The full license text is in [`LICENSES/CC-BY-SA-3.0.txt`](../LICENSES/CC-BY-SA-3.0.txt).

The inputs were assembled as measurement probes and the outputs were recorded as rendered by live Wikidot at the capture dates named in each file; both are kept as evidence for the verification harnesses.
