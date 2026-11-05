#!/usr/bin/env python3
"""Normalize live.html and ftml.html, write diffs/<slug>.diff and hunks.json (with signatures).
Usage: diff_cluster.py DATA_DIR OUT_DIR"""
import difflib, json, re, sys
from pathlib import Path
from normalize import normalize

data, out = Path(sys.argv[1]), Path(sys.argv[2]); (out / "diffs").mkdir(parents=True, exist_ok=True)

def abst(tok):
    if tok.startswith('T"'): return "T"
    m = re.match(r"</?([a-z0-9]+)", tok)
    if not m: return tok
    attrs = re.findall(r' ([a-z-]+)=', tok)
    cls = re.search(r' class="([^"]*)"', tok)
    return f"{'/' if tok.startswith('</') else ''}{m.group(1)}[{','.join(attrs)}]" + (f".{cls.group(1)}" if cls else "")

hunks, summary = [], {}
for d in sorted(data.iterdir()):
    if not (d / "ftml.html").exists(): continue
    a = normalize((d / "live.html").read_text()); b = normalize((d / "ftml.html").read_text())
    (d / "live.norm").write_text("\n".join(a) + "\n"); (d / "ftml.norm").write_text("\n".join(b) + "\n")
    (out / "diffs" / f"{d.name}.diff").write_text("".join(difflib.unified_diff(
        [x + "\n" for x in a], [x + "\n" for x in b], "live", "ftml", n=2)))
    sm = difflib.SequenceMatcher(None, a, b, autojunk=False); n = 0
    for tag, i1, i2, j1, j2 in sm.get_opcodes():
        if tag == "equal": continue
        n += 1
        hunks.append({"slug": d.name, "op": tag, "live": a[i1:i2], "ftml": b[j1:j2],
                      "sig": [tag, [abst(x) for x in a[i1:i2][:4]], [abst(x) for x in b[j1:j2][:4]]]})
    summary[d.name] = {"live_tokens": len(a), "ftml_tokens": len(b), "hunks": n}
(out / "hunks.json").write_text(json.dumps(hunks, indent=1, ensure_ascii=False))
(out / "summary.json").write_text(json.dumps(summary, indent=1))
print(len(summary), "pages;", len(hunks), "hunks")
