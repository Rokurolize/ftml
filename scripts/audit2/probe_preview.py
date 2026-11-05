#!/usr/bin/env python3
"""Run ad-hoc live PagePreview probes twice per shape and record evidence.

Input JSON: [{"name": "...", "source": "...", "expected": "..."|null}, ...]
Each shape is posted twice with independent anonymous token cookies; both raw
bodies are stored with SHA-256 so a rule can cite a stable capture.

Usage:
  probe_preview.py --list probes.json --out probes/
"""

import argparse
import hashlib
import json
import random
import string
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

import html5lib

SITE = "sandbox-for-codex"
MODULE = "edit/PagePreviewModule"
BLOCK = {
    "address", "blockquote", "body", "caption", "center", "col", "colgroup",
    "dd", "div", "dl", "dt", "fieldset", "figure", "footer", "form", "h1",
    "h2", "h3", "h4", "h5", "h6", "header", "hr", "iframe", "li", "ol", "p",
    "pre", "section", "table", "tbody", "td", "tfoot", "th", "thead", "tr",
    "ul", "script", "style", "html", "head",
}


def sha256(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def post(source):
    token = "".join(random.choice(string.ascii_lowercase + string.digits)
                    for _ in range(16))
    data = urllib.parse.urlencode({
        "moduleName": MODULE,
        "source": source,
        "mode": "page",
        "wikidot_token7": token,
    }).encode()
    url = f"https://{SITE}.wikidot.com/ajax-module-connector.php"
    last = None
    for attempt in range(5):
        request = urllib.request.Request(url, data=data, method="POST", headers={
            "Cookie": f"wikidot_token7={token}",
            "Content-Type": "application/x-www-form-urlencoded; charset=UTF-8",
            "User-Agent": "ftml-corpus-audit2 (anonymous PagePreview, read-only)",
            "X-Requested-With": "XMLHttpRequest",
        })
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                body = json.loads(response.read().decode("utf-8"))
            if body.get("status") != "ok":
                last = f"status {body.get('status')}"
                time.sleep(2 ** attempt * 2)
                continue
            return body["body"]
        except (urllib.error.URLError, TimeoutError, json.JSONDecodeError) as error:
            last = str(error)
            time.sleep(2 ** attempt * 2)
    raise RuntimeError(f"PagePreview failed: {last}")


def parse(html):
    return html5lib.parseFragment(html, container="div", treebuilder="etree",
                                  namespaceHTMLElements=False)


def render(html):
    """Readable normalized serialization."""
    out = []
    for token in tokens(parse(html), True):
        if token[0] == "text":
            out.append(token[1].replace("&", "&amp;").replace("<", "&lt;"))
        elif token[0] == "comment":
            out.append(f"<!--{token[1]}-->")
        elif token[0] == "open":
            attrs = "".join(f' {k}="{v}"' for k, v in token[2])
            out.append(f"<{token[1]}{attrs}>")
        elif token[1] not in {"area", "base", "br", "col", "embed", "hr",
                              "img", "input", "link", "meta", "source",
                              "track", "wbr"}:
            out.append(f"</{token[1]}>")
    return "".join(out)


def tokens(element, normalize):
    out = []

    def is_block(node):
        return isinstance(node.tag, str) and node.tag.lower() in BLOCK

    def emit_text(text, prev_block, next_block, container_block):
        if not text:
            return
        if normalize and not text.strip() and (prev_block or next_block
                                               or container_block):
            return
        out.append(("text", text))

    def walk(node, container_block):
        children = list(node)
        emit_text(node.text, container_block,
                  bool(children) and is_block(children[0]),
                  container_block and not children)
        for index, child in enumerate(children):
            if not isinstance(child.tag, str):
                out.append(("comment", child.text or ""))
            else:
                attrs = sorted((k, v) for k, v in child.attrib.items())
                out.append(("open", child.tag.lower(), tuple(attrs)))
                walk(child, is_block(child))
                out.append(("close", child.tag.lower()))
            following = children[index + 1] if index + 1 < len(children) else None
            emit_text(child.tail,
                      is_block(child) if isinstance(child.tag, str) else False,
                      following is not None and is_block(following),
                      container_block and following is None)

    walk(element, True)
    if normalize:
        if out and out[0][0] == "text":
            out[0] = ("text", out[0][1].lstrip())
            if not out[0][1]:
                out.pop(0)
        if out and out[-1][0] == "text":
            out[-1] = ("text", out[-1][1].rstrip())
            if not out[-1][1]:
                out.pop()
    return out


def visible_text(element):
    parts = []

    def walk(node):
        tag = node.tag.lower() if isinstance(node.tag, str) else None
        if tag in ("script", "style"):
            pass
        elif tag is not None:
            if tag in BLOCK or tag == "br":
                parts.append(" ")
            parts.append(node.text or "")
            for child in node:
                walk(child)
            if tag in BLOCK:
                parts.append(" ")
        if node is not element:
            parts.append(node.tail or "")

    walk(element)
    return " ".join("".join(parts).split())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--list", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--delay", type=float, default=0.8)
    args = ap.parse_args()
    probes = json.loads(Path(args.list).read_text(encoding="utf-8"))
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    summary = {}
    for probe in probes:
        name = probe["name"]
        source = probe["source"]
        record = {"name": name, "source": source,
                  "source_sha256": sha256(source), "site": SITE}
        html1 = post(source)
        time.sleep(args.delay)
        html2 = post(source)
        time.sleep(args.delay)
        record["raw_html"] = [html1, html2]
        record["raw_html_sha256"] = [sha256(html1), sha256(html2)]
        record["stable"] = html1 == html2
        record["captured_at"] = datetime.now(timezone.utc).isoformat()
        record["live_norm"] = render(html1)
        record["live_text"] = visible_text(parse(html1))
        if probe.get("expected") is not None:
            record["expected"] = probe["expected"]
            record["matches_expected"] = render(html1) == render(probe["expected"])
        (out / f"{name}.json").write_text(
            json.dumps(record, ensure_ascii=False, indent=1), encoding="utf-8")
        summary[name] = {
            "stable": record["stable"],
            "live_sha": record["raw_html_sha256"][0][:12],
            "matches_expected": record.get("matches_expected"),
            "live_norm": record["live_norm"],
        }
        print(f"{name:44} stable={record['stable']} "
              f"match={record.get('matches_expected')} {record['live_norm'][:160]}")
    (out / "summary.json").write_text(json.dumps(summary, indent=1))


if __name__ == "__main__":
    main()
