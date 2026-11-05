#!/usr/bin/env python3
"""Walk candidates.json (seeded shuffle) and fetch until N include-free articles are kept.
Articles whose source has [[include ...]] are recorded in skipped.json and their bulky files
removed: the render harness has no Includer, so they cannot be compared meaningfully.
Usage: select_and_fetch.py DATA_DIR N"""
import json, re, shutil, sys, time
from pathlib import Path
import fetch

HERE = Path(__file__).resolve().parent
out = Path(sys.argv[1]); n = int(sys.argv[2])
cands = json.loads((HERE / "candidates.json").read_text())
kept, skipped = [], {}
sk_path = HERE / "skipped.json"
if sk_path.exists():
    skipped = json.loads(sk_path.read_text())
for slug in cands:
    if len(kept) >= n:
        break
    if slug in skipped:
        continue
    r = fetch.fetch(slug, out)
    src = out / slug / "source.ftml"
    if r not in ("ok", "cached") or not src.exists():
        skipped[slug] = r
        shutil.rmtree(out / slug, ignore_errors=True)
    elif re.search(r"\[\[\s*include\b", src.read_text(), re.I):
        skipped[slug] = "has-include"
        shutil.rmtree(out / slug, ignore_errors=True)
    else:
        kept.append(slug)
    print(slug, skipped.get(slug, "KEPT"), len(kept), flush=True)
    sk_path.write_text(json.dumps(skipped, indent=1))
    (HERE / "selected.json").write_text(json.dumps(kept))
    time.sleep(1)
