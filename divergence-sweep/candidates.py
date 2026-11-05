#!/usr/bin/env python3
"""Build the candidate article list from the public scp-wiki series index pages,
minus every slug already mentioned anywhere in the ftml repository (fixtures, tests, docs)."""
import json, random, re, subprocess, sys, time, urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parent
UA = "ftml-divergence-sweep/1 (discovery; public pages only)"

def get(url):
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=60) as r:
        return r.read().decode("utf-8", "replace")

def repo_slugs():
    """Every scp-NNN[N][-suffix] token that appears anywhere in tracked files."""
    files = subprocess.check_output(["git", "-C", str(REPO), "ls-files", "-z"]).split(b"\0")
    seen = set()
    for f in files:
        p = REPO / f.decode()
        if not f or "divergence-sweep" in f.decode() or not p.is_file():
            continue
        try:
            t = p.read_text("utf-8", "ignore").lower()
        except Exception:
            continue
        for m in re.finditer(r"scp-0*(\d{1,4})(-[a-z0-9]+)?", t):
            seen.add(f"{int(m.group(1))}{m.group(2) or ''}")
    return seen

def main():
    used = repo_slugs()
    cands = []
    for idx in ["scp-series"] + [f"scp-series-{i}" for i in range(2, 8)]:
        html = get(f"https://scp-wiki.wikidot.com/{idx}")
        for m in re.finditer(r'href="/(scp-(\d{3,4}))"', html):
            n = int(m.group(2))
            if str(n) in used:
                continue
            cands.append(m.group(1))
        time.sleep(1)
    cands = sorted(set(cands))
    random.Random(20261105).shuffle(cands)
    (HERE / "excluded-repo-slugs.txt").write_text("\n".join(sorted(used, key=lambda s: (len(s), s))) + "\n")
    (HERE / "candidates.json").write_text(json.dumps(cands, indent=0) + "\n")
    print(len(used), "repo slugs excluded;", len(cands), "candidates")

main()
