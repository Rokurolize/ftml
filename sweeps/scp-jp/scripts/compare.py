#!/usr/bin/env python3
"""Normalize live and ftml HTML, mask per-render values, and diff per page.

Writes data/pages/<slug>/{live.norm.txt,ftml.norm.txt,diff.txt} and
data/diff_summary.json. Needs html5lib (use a venv; run with -I from a
different cwd than the data).

Normalization: parse as a body fragment, serialize one node per line with
sorted attributes. Whitespace runs collapse to one space (not inside pre or
textarea); whitespace-only text is dropped. U+00A0 and U+3000 are rendered
as visible markers. Masks cover ids and timestamps that differ every render.
"""

import difflib
import html
import json
import os
import re
import sys

import html5lib

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
DATA = os.path.join(ROOT, "data")

MASKS = [
    (re.compile(r"wj-id-[A-Za-z0-9]{16}"), "wj-id-<ID>"),
    (re.compile(r"(wiki-tabview|wj-button|wj-embed-video|wj-gallery|wj-social)-[0-9a-f]{32}"), r"\1-<ID>"),
    (re.compile(r"tabview-[0-9a-f]{32}"), "tabview-<ID>"),
    (re.compile(r"\bprw\d+\b"), "prw<N>"),
    (re.compile(r"timestamp=\d+"), "timestamp=<TS>"),
    (re.compile(r"(local--html/[^/\"]+/)[0-9a-f]+(-\d+)?"), r"\1<HASH>"),
    (re.compile(r"bibcite-\d+-\d+a?"), "bibcite-<ID>"),
    (re.compile(r"\b\d{10}\b"), "<TS10>"),
]
VOID = {"br", "hr", "img", "input", "meta", "link"}
PRE = {"pre", "textarea"}


def mask(s):
    for rx, rep in MASKS:
        s = rx.sub(rep, s)
    return s


def vis(s):
    return s.replace(" ", "<NBSP>").replace("　", "<IDSP>")


def walk(node, out, pre=False):
    tag = node.tag if isinstance(node.tag, str) else None
    if tag is None:
        return
    name = tag.split("}")[-1]
    if name in ("DOCUMENT_FRAGMENT",):
        pass
    attrs = " ".join(f'{k}="{mask(v)}"' for k, v in sorted(node.attrib.items()))
    out.append(f"<{name}{(' ' + attrs) if attrs else ''}>")
    inpre = pre or name in PRE
    def text(t):
        if not t:
            return
        t = vis(mask(t))
        if not inpre:
            t = re.sub(r"[ \t\r\n]+", " ", t)
            if not t.strip():
                return
            t = t.strip() if False else t
        out.append("T:" + json.dumps(t, ensure_ascii=False))
    text(node.text)
    for ch in node:
        walk(ch, out, inpre)
        text(ch.tail)
    if name not in VOID:
        out.append(f"</{name}>")


def normalize(fragment):
    doc = html5lib.parse(f"<div id=root>{fragment}</div>", namespaceHTMLElements=False, treebuilder="etree")
    root = [e for e in doc.iter() if e.get("id") == "root"][0]
    out = []
    for ch in root:
        walk(ch, out)
    # Text node placement differs benignly by whitespace; trim edges.
    lines = []
    for l in out:
        if l.startswith("T:"):
            s = json.loads(l[2:]).strip()
            if not s:
                continue
            lines.append("T:" + json.dumps(s, ensure_ascii=False))
        else:
            lines.append(l)
    return lines


def strip_live_chrome(live):
    """Drop caller-side wrappers: rate widget/credit module is theme CSS-owned HTML
    that comes from includes of other pages and is rendered by ftml too, so keep it."""
    return live


def main():
    pages = json.load(open(os.path.join(DATA, "pages.json"), encoding="utf-8"))["pages"]
    summary = {}
    for row in pages:
        slug = row["slug"]
        pdir = os.path.join(DATA, "pages", slug)
        if not os.path.exists(os.path.join(pdir, "ftml.html")):
            continue
        live = normalize(open(os.path.join(pdir, "live.html"), encoding="utf-8").read())
        ftml = normalize(open(os.path.join(pdir, "ftml.html"), encoding="utf-8").read())
        for name, lines in (("live", live), ("ftml", ftml)):
            with open(os.path.join(pdir, f"{name}.norm.txt"), "w", encoding="utf-8") as fh:
                fh.write("\n".join(lines) + "\n")
        diff = list(difflib.unified_diff(live, ftml, "live", "ftml", n=3, lineterm=""))
        with open(os.path.join(pdir, "diff.txt"), "w", encoding="utf-8") as fh:
            fh.write("\n".join(diff) + ("\n" if diff else ""))
        hunks = sum(1 for d in diff if d.startswith("@@"))
        summary[slug] = {"live_nodes": len(live), "ftml_nodes": len(ftml), "hunks": hunks, "identical": not diff}
        print(f"{slug}: hunks={hunks}")
    with open(os.path.join(DATA, "diff_summary.json"), "w", encoding="utf-8") as fh:
        json.dump(summary, fh, indent=1, sort_keys=True)
        fh.write("\n")


if __name__ == "__main__":
    sys.exit(main())
