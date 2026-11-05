#!/usr/bin/env python3
"""Classify audit mismatches: whitespace-only, caller-runtime context, or bug.

Live evidence is PagePreview by default; when a saved-page capture exists for a
case (``saved-pages/saved-results.json``) the saved-page render is used instead,
because PagePreview cannot evaluate ``[[iftags]]`` gates and keeps ``[[html]]``
blocks literal.

Classes:
  match-exact                bytes equal after the audit comparator
  harmless-whitespace        same element/attribute structure + same visible text,
                             differences are formatting whitespace only (normalization)
  context-caller-runtime     equal after relaxing caller-owned output (newpage
                             class, file-host URLs, Wikidot asset/init scripts,
                             generated gallery/tabview ids)
  context-boundary           Wikijump runtime marker (gallery/embedvideo) or
                             caller-owned user lookup; documented in
                             docs/WikijumpBoundary.md and
                             tests/fixtures/wikidot-parity/caller-runtime-contracts.json
  unjudgeable                PagePreview cannot observe the fixture's true-gate
                             render and no saved-page capture exists
  bug-encoded                element/attribute/visible-text divergence
"""

import json
import re
import sys
from collections import defaultdict
from pathlib import Path

import os
REPO = Path(__file__).resolve().parents[2]
CFG = Path(os.environ.get("FTML_AUDIT_RECORD", REPO / ".audit"))
sys.path.insert(0, str(REPO / "scripts"))
import audit_corpus_live as audit  # noqa: E402

CACHE = CFG / "preview-cache"
SAVED = CFG / "saved-pages" / "saved-results.json"


def load_rows():
    rows = {}
    for path in sorted(CFG.glob("verdicts-*.jsonl")):
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.strip():
                row = json.loads(line)
                rows[row["case"]] = row
    return rows


def raw_for(row):
    path = CACHE / f"{row['source_sha256']}.json"
    return json.loads(path.read_text(encoding="utf-8")) if path.exists() else None


def relaxed_html(html, modes):
    if "newpage" in modes:
        html = re.sub(r'(<a\b[^>]*?)\s+class="newpage"', r"\1", html)
        html = re.sub(r'(<a\b[^>]*?)\s+class="newpage"', r"\1", html)
    if "fileurl" in modes:
        html = re.sub(
            r'https?://[^"\s<>]*(?:wdfiles|wjfiles)\.com/local--(?:files|resized-images)/[^"\s<>]*',
            "FILEURL", html)
    if "asset-script" in modes:
        html = re.sub(
            r'<script\b[^>]*src="https?://[^"]*cloudfront\.net/[^"]*"[^>]*>\s*</script>',
            "", html)
    if "tabview-init-script" in modes:
        html = re.sub(
            r'(?s)<script\b[^>]*>(?:(?!</script>).)*?YAHOO\.widget\.TabView.*?</script>',
            "", html)
    if "caller-iframe" in modes:
        def _iframe(match):
            tag = match.group(0)
            if "html-block-iframe" not in tag:
                return tag
            return re.sub(r'src="[^"]*"', 'src="#"', tag)
        html = re.sub(r'<iframe\b[^>]*>', _iframe, html)
    if "gallery-id" in modes:
        html = re.sub(r'gallery-box-\d+', 'gallery-box-#', html)
    if "tabview-id" in modes:
        html = re.sub(r'wiki-tabview-[0-9a-f]+', 'wiki-tabview-#', html)
    return html


def token_paths(fragment):
    out, paths = [], []

    def is_block(node):
        return isinstance(node.tag, str) and node.tag.lower() in audit.BLOCK

    def emit_text(text, prev_block, next_block, container_block, path):
        if not text:
            return
        if not text.strip() and (prev_block or next_block or container_block):
            return
        out.append(("text", text))
        paths.append(path)

    def walk(node, container_block, path):
        children = list(node)
        emit_text(node.text, container_block,
                  bool(children) and is_block(children[0]),
                  container_block and not children, path)
        for index, child in enumerate(children):
            if not isinstance(child.tag, str):
                out.append(("comment", child.text or ""))
                paths.append(path)
            else:
                attrs = sorted((k, audit.mask(v)) for k, v in child.attrib.items())
                tag = child.tag.lower()
                out.append(("open", tag, tuple(attrs)))
                paths.append(path)
                walk(child, is_block(child), path + (tag,))
                out.append(("close", tag))
                paths.append(path)
            following = children[index + 1] if index + 1 < len(children) else None
            emit_text(child.tail,
                      is_block(child) if isinstance(child.tag, str) else False,
                      following is not None and is_block(following),
                      container_block and following is None, path)

    walk(fragment, True, ())
    if out and out[0][0] == "text":
        stripped = out[0][1].lstrip()
        out[0] = ("text", stripped)
        if not stripped:
            out.pop(0)
            paths.pop(0)
    if out and out[-1][0] == "text":
        stripped = out[-1][1].rstrip()
        out[-1] = ("text", stripped)
        if not stripped:
            out.pop()
            paths.pop()
    return out, paths


def compare(html, expected):
    live = audit.parse(html)
    fixture = audit.parse(expected)
    structure = lambda frag: [t for t in audit.tokens(frag, False) if t[0] != "text"]
    live_struct = structure(live)
    fix_struct = structure(fixture)
    live_text = audit.visible_text(live)
    fix_text = audit.visible_text(fixture)
    exact, _ = token_paths(live)
    fix_exact, _ = token_paths(fixture)
    return {
        "exact": exact == fix_exact,
        "structure": live_struct == fix_struct,
        "text_equal": live_text == fix_text,
        "live_text": live_text,
        "fixture_text": fix_text,
        "live_struct": audit.serialize(live_struct),
        "fixture_struct": audit.serialize(fix_struct),
        "live_exact": audit.serialize(exact),
        "fixture_exact": audit.serialize(fix_exact),
        "live_norm": audit.serialize(audit.tokens(live, True)),
        "fixture_norm": audit.serialize(audit.tokens(fixture, True)),
    }


def pre_whitespace_diff(html, expected):
    live = audit.parse(html)
    fixture = audit.parse(expected)
    live_tokens, live_paths = token_paths(live)
    fix_tokens, fix_paths = token_paths(fixture)
    for index in range(max(len(live_tokens), len(fix_tokens))):
        left = live_tokens[index] if index < len(live_tokens) else None
        right = fix_tokens[index] if index < len(fix_tokens) else None
        if left == right:
            continue
        path = live_paths[index] if index < len(live_paths) else ()
        if "pre" in path or "textarea" in path:
            return True
    return False


RELAX_SETS = [
    ("newpage",),
    ("fileurl",),
    ("asset-script",),
    ("asset-script", "tabview-init-script"),
    ("caller-iframe",),
    ("newpage", "fileurl", "asset-script", "tabview-init-script",
     "gallery-id", "tabview-id", "caller-iframe"),
]

BOUNDARY_MARKERS = {
    "<div class=\"wj-gallery\"": "Wikijump gallery marker",
    "wj-gallery-": "Wikijump gallery marker",
    "wj-embed-video": "Wikijump embedvideo marker",
}


def judge(html, expected, source, family=None, source_text=""):
    base = compare(html, expected)
    result = {"class": None, **base}
    if base["exact"]:
        result["class"] = "match-exact"
        return result
    if base["structure"] and base["text_equal"]:
        if pre_whitespace_diff(html, expected):
            result["class"] = "bug-encoded (visible whitespace inside pre)"
        else:
            result["class"] = "harmless-whitespace (normalization-only)"
        return result
    for modes in RELAX_SETS:
        relaxed = compare(relaxed_html(html, modes), relaxed_html(expected, modes))
        if relaxed["structure"] and relaxed["text_equal"]:
            result["class"] = f"context-caller-runtime ({'+'.join(modes)})"
            result["relaxations"] = list(modes)
            result.update({f"relaxed_{k}": v for k, v in relaxed.items()
                           if k in ("live_struct", "fixture_struct")})
            return result
    for marker, label in BOUNDARY_MARKERS.items():
        if marker in expected:
            result["class"] = f"context-boundary ({label})"
            return result
    if family == "user":
        result["class"] = "context-boundary (caller user resolution)"
        return result
    result["class"] = "bug-encoded"
    return result


def main():
    rows = load_rows()
    ordered = [rows[k] for k in sorted(rows)]
    saved = json.loads(SAVED.read_text()) if SAVED.exists() else {}
    counts = defaultdict(lambda: defaultdict(int))
    for row in ordered:
        counts[row["family"]][row["status"]] += 1
    summary = {"total": len(ordered), "families": {}}
    for family in sorted(counts):
        summary["families"][family] = dict(counts[family])
    (CFG / "audit-all.jsonl").write_text(
        "\n".join(json.dumps(r, ensure_ascii=False) for r in ordered) + "\n",
        encoding="utf-8")

    results = {}
    for row in ordered:
        if row["status"] == "match":
            continue
        expected = (REPO / "test" / row["case"] / "wikidot.html").read_text(
            encoding="utf-8")
        record = raw_for(row)
        entry = {
            "case": row["case"],
            "family": row["family"],
            "audit_status": row["status"],
            "audit_context": row["context"],
            "preview_raw_html_sha256": row["raw_html_sha256"],
            "expected": expected,
        }
        saved_entry = saved.get(row["case"])
        if saved_entry and saved_entry.get("content_file"):
            content = Path(saved_entry["content_file"]).read_text(encoding="utf-8")
            entry["evidence"] = "saved-page"
            entry["case_fullname"] = saved_entry.get("fullname")
            entry["content_sha256"] = saved_entry["content_sha256"]
            entry["content_stable"] = saved_entry.get("stable")
            entry["source"] = saved_entry["source"]
            entry.update(judge(content, expected, "saved", row["family"], entry["source"]))
        elif row["status"] == "context" and row["context"] == "iftags-true-gate":
            entry["evidence"] = "page-preview"
            entry["source"] = record["source"] if record else None
            entry["class"] = "unjudgeable (iftags true gate, preview all-false)"
            comparison = compare(record["raw_html"], expected) if record else {}
            entry.update(comparison)
        else:
            if record is None:
                continue
            entry["evidence"] = "page-preview"
            entry["source"] = record["source"]
            entry["live_raw_html"] = record["raw_html"]
            entry.update(judge(record["raw_html"], expected, "preview", row["family"], record["source"]))
        results[row["case"]] = entry

    classes = defaultdict(int)
    by_family = defaultdict(lambda: defaultdict(int))
    for entry in results.values():
        classes[entry["class"]] += 1
        by_family[entry["family"]][entry["class"]] += 1
    payload = {"summary": summary, "classes": dict(classes),
               "by_family": {k: dict(v) for k, v in by_family.items()},
               "cases": results}
    (CFG / "classified.json").write_text(
        json.dumps(payload, ensure_ascii=False, indent=1))
    print(json.dumps({"total": len(ordered), "classes": dict(classes)}, indent=1))


if __name__ == "__main__":
    main()
