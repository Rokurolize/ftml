#!/usr/bin/env python3
"""fetch N articles -> expand includes (ftml::include via expander, fetching missing components)
-> write expanded.ftml. Usage: pipeline.py DATA_DIR COMP_DIR N"""
import json, subprocess, sys, time
from pathlib import Path
import fetch

HERE = Path(__file__).resolve().parent
data, comp, n = Path(sys.argv[1]), Path(sys.argv[2]), int(sys.argv[3])
comp.mkdir(parents=True, exist_ok=True)
EXP = HERE.parent / "target/release/sweep-expander"
cands = json.loads((HERE / "candidates.json").read_text())
slugs, bad = [], {}
for s in cands:
    if len(slugs) >= n: break
    r = fetch.fetch(s, data)
    if (data / s / "source.ftml").exists(): slugs.append(s)
    else: bad[s] = r
    print(s, r, len(slugs), flush=True); time.sleep(1)
(HERE / "fetch-failures.json").write_text(json.dumps(bad, indent=1))

def comps():
    out = {}
    for p in comp.glob("*/source.ftml"):
        out[p.parent.name.lower()] = p.read_text()
    return out

def run():
    pages = comps(); pj = comp / "_pages.json"; pj.write_text(json.dumps(pages))
    inp = "\n".join(json.dumps({"case_id": s, "source": (data/s/"source.ftml").read_text()}) for s in slugs)
    r = subprocess.run([str(EXP), str(pj)], input=inp, capture_output=True, text=True, check=True)
    return [json.loads(l) for l in r.stdout.splitlines()]

for rnd in range(6):
    rows = run()
    miss = sorted({m for r in rows for m in r["missing"] if not m.startswith(":")})
    todo = [m for m in miss if not (comp / m / "source.ftml").exists() and m not in json.loads((HERE/"missing-components.json").read_text() if (HERE/"missing-components.json").exists() else "{}")]
    print("round", rnd, "missing", len(miss), "to fetch", len(todo), flush=True)
    if not todo: break
    fails = json.loads((HERE/"missing-components.json").read_text()) if (HERE/"missing-components.json").exists() else {}
    for m in todo:
        r = fetch.fetch(m, comp); print(" comp", m, r, flush=True)
        if r not in ("ok", "cached"): fails[m] = r
        (HERE/"missing-components.json").write_text(json.dumps(fails, indent=1)); time.sleep(1)
for r in rows:
    (data / r["case_id"] / "expanded.ftml").write_text(r["expanded"])
json.dump({r["case_id"]: r["missing"] for r in rows}, open(HERE/"unresolved-includes.json","w"), indent=1)
print("done", len(rows))
