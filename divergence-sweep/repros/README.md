Minimized repros (run with `python3 repro.py FILE`, or `sweep-expander repros/pagesN.json` for include cases).
- F1: `printf 'a [Note: x] b' | python3 repro.py`  -> `<p>a x b</p>` (live keeps `[Note: …]` literally)
- F2: `[[include component:t\n|u=http://x.net/d\n]]` with component:t = `[{$u} label]` (repros/pages.json) expands to `[http://x.net/d\n label]`: value keeps its trailing newline.
- F2b: `printf '[[div class="a\n\nb"]]\nz\n[[/div]]' | python3 repro.py` -> `class="a b"`.
