#!/usr/bin/env python3
"""Expand includes and render every selected page with the repo render harness.

For each data/pages/<slug>/:
  expanded.ftml        source after looping ftml::include (tools/include_expand)
  include_report.json  per-pass include targets, variables, found/missing
  ftml.html            harness HTML (Layout::Wikidot, WikitextMode::Page)
  ftml_result.json     harness result minus html (parse errors, engine info)

existing_pages for the harness is derived from the live HTML: an internal
link without class "newpage" names an existing page. Page existence is caller
runtime state, so this keeps it out of the diff.

usage: render.py --harness PATH --expander PATH
"""

import argparse
import html
import json
import os
import re
import subprocess
import sys
import urllib.parse

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
DATA = os.path.join(ROOT, "data")
SITE = "scp-jp"
SCHEMA = "wikijump_syntax_differential.syntax_case.v1"

A_TAG_RE = re.compile(r"<a\b([^>]*)>", re.IGNORECASE)
ATTR_RE = re.compile(r'([a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*=\s*"([^"]*)"')


def live_link_state(live_html):
    exists, missing = set(), set()
    for m in A_TAG_RE.finditer(live_html):
        attrs = dict(ATTR_RE.findall(m.group(1)))
        href = html.unescape(attrs.get("href", ""))
        for prefix in (f"http://{SITE}.wikidot.com", f"https://{SITE}.wikidot.com"):
            if href.startswith(prefix):
                href = href[len(prefix):]
        if not href.startswith("/") or href.startswith("//"):
            continue
        page = urllib.parse.unquote(href[1:]).split("#", 1)[0].split("/", 1)[0]
        if not page or page.startswith("system:") and "/" in href[1:]:
            continue
        if "newpage" in attrs.get("class", "").split():
            missing.add(page)
        else:
            exists.add(page)
    return sorted(exists), sorted(missing)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--harness", required=True)
    ap.add_argument("--expander", required=True)
    ap.add_argument("--passes", type=int, default=5)
    ap.add_argument("--only", nargs="*")
    args = ap.parse_args()

    pages = json.load(open(os.path.join(DATA, "pages.json"), encoding="utf-8"))["pages"]
    cases = []
    for row in pages:
        slug = row["slug"]
        if args.only and slug not in args.only:
            continue
        pdir = os.path.join(DATA, "pages", slug)
        source = open(os.path.join(pdir, "source.ftml"), encoding="utf-8").read()
        proc = subprocess.run(
            [args.expander, os.path.join(DATA, "includes"), SITE, str(args.passes)],
            input=source.encode("utf-8"),
            capture_output=True,
            check=True,
        )
        expanded = proc.stdout.decode("utf-8")
        report = json.loads(proc.stderr.decode("utf-8").strip().splitlines()[-1])
        with open(os.path.join(pdir, "expanded.ftml"), "w", encoding="utf-8", newline="") as fh:
            fh.write(expanded)
        live = open(os.path.join(pdir, "live.html"), encoding="utf-8").read()
        exists, missing = live_link_state(live)
        report["existing_pages_from_live"] = exists
        report["newpage_links_in_live"] = missing
        with open(os.path.join(pdir, "include_report.json"), "w", encoding="utf-8") as fh:
            json.dump(report, fh, ensure_ascii=False, indent=1)
            fh.write("\n")
        meta = json.load(open(os.path.join(pdir, "meta.json"), encoding="utf-8"))
        title = meta["wikirequest_info"].get("pageUnixName", slug)
        cases.append(
            {
                "schema": SCHEMA,
                "case_id": slug,
                "source": expanded,
                "title": title,
                "page_context": {"site": SITE, "page": slug},
                "layout": "wikidot",
                "existing_pages": exists,
            }
        )

    payload = "".join(json.dumps(c, ensure_ascii=False) + "\n" for c in cases)
    proc = subprocess.run([args.harness], input=payload.encode("utf-8"), capture_output=True, check=True)
    for line in proc.stdout.decode("utf-8").splitlines():
        result = json.loads(line)
        slug = result["case_id"]
        pdir = os.path.join(DATA, "pages", slug)
        with open(os.path.join(pdir, "ftml.html"), "w", encoding="utf-8", newline="") as fh:
            fh.write(result.get("html") or "")
        slim = {k: v for k, v in result.items() if k not in ("html", "text")}
        with open(os.path.join(pdir, "ftml_result.json"), "w", encoding="utf-8") as fh:
            json.dump(slim, fh, ensure_ascii=False, indent=1)
            fh.write("\n")
        print(f"{slug}: status={result['status']} parse_errors={len(result['parse_errors'])} html_bytes={len((result.get('html') or '').encode())}")


if __name__ == "__main__":
    sys.exit(main())
