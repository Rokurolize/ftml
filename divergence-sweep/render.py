#!/usr/bin/env python3
"""Render every DATA_DIR/<slug>/source.ftml with the repo render harness (Layout::Wikidot)
and write ftml.html + ftml.parse_errors.json beside it.
Usage: render.py DATA_DIR HARNESS_BIN"""
import json, subprocess, sys
from pathlib import Path

data = Path(sys.argv[1]); bin_ = sys.argv[2]
slugs = sorted(p.name for p in data.iterdir() if (p / "source.ftml").exists())
lines = [json.dumps({"schema": "wikijump_syntax_differential.syntax_case.v1", "case_id": s,
                     "source": (data / s / "source.ftml").read_text(), "title": s, "layout": "wikidot",
                     "page_context": {"site": "scp-wiki", "page": s}}) for s in slugs]
res = subprocess.run([bin_], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True)
for row in map(json.loads, res.stdout.splitlines()):
    d = data / row["case_id"]
    assert row["status"] == "rendered", row
    (d / "ftml.html").write_text(row["html"])
    (d / "ftml.parse_errors.json").write_text(json.dumps(row["parse_errors"], indent=1))
    eng = row["engine"]
print(len(slugs), "rendered;", "engine", eng["version"], eng["git_commit"])
