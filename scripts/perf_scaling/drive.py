"""Scaling driver: runs the repo render harness (examples/corpus_profile.rs)
on each adversarial family at doubling sizes and reports the growth exponent.

usage: python3 drive.py BIN OUT.json [family-substring ...]
env: LAYOUT (wikidot), START (256), MAXN (65536), BUDGET_MS (1500), TIMEOUT (30)
"""
import json, math, os, subprocess, sys, tempfile, time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from families import FAMILIES

BIN = sys.argv[1]
OUT = sys.argv[2]
FILTERS = sys.argv[3:]
LAYOUT = os.environ.get("LAYOUT", "wikidot")
START = int(os.environ.get("START", "256"))
MAXN = int(os.environ.get("MAXN", "65536"))
BUDGET_MS = float(os.environ.get("BUDGET_MS", "1500"))
TIMEOUT = float(os.environ.get("TIMEOUT", "30"))
REPEAT = int(os.environ.get("REPEAT", "1"))
TMP = tempfile.mkdtemp(dir=os.environ.get("TMPDIR"))


def run(text):
    page = {"page_id": 1, "site": "fuzz", "slug": "fuzz", "wikitext": text}
    inp = os.path.join(TMP, "in.jsonl")
    out = os.path.join(TMP, "out.tsv")
    with open(inp, "w") as f:
        f.write(json.dumps(page) + "\n")
    env = dict(os.environ, CORPUS_LAYOUT=LAYOUT, CORPUS_STAGE_COLUMNS="1")
    best = None
    for _ in range(REPEAT):
        try:
            with open(inp) as stdin:
                subprocess.run([BIN, out], stdin=stdin, env=env, timeout=TIMEOUT,
                               check=True, stderr=subprocess.DEVNULL)
        except subprocess.TimeoutExpired:
            return None
        except subprocess.CalledProcessError as e:
            return {"error": e.returncode}
        cols = open(out).read().split("\t")
        row = {"digest": cols[1], "us": int(cols[2]),
               "stages": [int(c) for c in cols[3:7]]}
        if best is None or row["us"] < best["us"]:
            best = row
    return best


def slope(a, b):
    (n1, t1), (n2, t2) = a, b
    return math.log(t2 / t1) / math.log(n2 / n1)


results = {}
if os.path.exists(OUT):
    results = json.load(open(OUT))

SHARD = os.environ.get("SHARD")
for index, (name, gen) in enumerate(FAMILIES.items()):
    if SHARD:
        i, k = map(int, SHARD.split("/"))
        if index % k != i:
            continue
    if FILTERS and not any(f in name for f in FILTERS):
        continue
    pts = []
    n = START
    status = "ok"
    while n <= MAXN:
        text = gen(n)
        r = run(text)
        if r is None:
            pts.append({"n": n, "bytes": len(text.encode()), "us": None})
            status = "timeout"
            break
        if "error" in r:
            pts.append({"n": n, "bytes": len(text.encode()), "error": r["error"]})
            status = "crash"
            break
        pts.append({"n": n, "bytes": len(text.encode()), **r})
        if r["us"] / 1000 > BUDGET_MS:
            break
        n *= 2
    timed = [(p["bytes"], max(p["us"], 1)) for p in pts if p.get("us")]
    big = [p for p in timed if p[1] > 3000]
    k = slope(big[-2], big[-1]) if len(big) >= 2 else (
        slope(timed[-2], timed[-1]) if len(timed) >= 2 else float("nan"))
    results[name] = {"status": status, "exponent": k, "points": pts}
    last = pts[-1]
    stage = last.get("stages")
    print(f"{name:34s} k={k:5.2f} {status:7s} n={last['n']:6d} "
          f"bytes={last['bytes']:8d} ms={(last.get('us') or 0)/1000:9.1f} "
          f"stages={stage}", flush=True)
    json.dump(results, open(OUT, "w"), indent=1)
