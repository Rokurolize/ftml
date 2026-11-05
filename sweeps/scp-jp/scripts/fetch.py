#!/usr/bin/env python3
"""Fetch candidate pages, wikitext, live HTML, and include targets from scp-jp.

Steps (each idempotent, cached under --cache):

  candidates   ListPages by rating for mainlist SCPs and tales -> data/candidates.json
  capture      For every candidate: page HTML (live render + page id) and
               ViewSource wikitext; formatting statistics -> data/candidates.json
  select       Pick heavily formatted pages -> data/pages.json
  materialize  Write data/pages/<slug>/{source.ftml,live.html,meta.json} and
               recursively prefetch [[include]] targets into data/includes/

Run with -I so nothing is imported from the data directories.
"""

import argparse
import json
import math
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import wd  # noqa: E402

SITE = "scp-jp"
ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
DATA = os.path.join(ROOT, "data")


def write_json(path, value):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(value, fh, ensure_ascii=False, indent=1, sort_keys=True)
        fh.write("\n")


def read_json(path):
    with open(path, encoding="utf-8") as fh:
        return json.load(fh)


def write_text(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="") as fh:
        fh.write(text)


# --------------------------------------------------------------------------
# candidates


def list_pages(tags, limit):
    body_fmt = "QQROWQQ%%fullname%%|%%rating%%|%%size%%|%%created_at%%|%%title%%QQENDQQ"
    payload, prov = wd.module(
        SITE,
        "list/ListPagesModule",
        category="*",
        tags=tags,
        order="rating desc",
        perPage=str(limit),
        limit=str(limit),
        separate="false",
        module_body=body_fmt,
    )
    rows = []
    for m in re.finditer(r"QQROWQQ(.*?)QQENDQQ", payload["body"], re.DOTALL):
        text = wd.html.unescape(re.sub(r"<[^>]+>", "", m.group(1)))
        fullname, rating, size, created, title = text.split("|", 4)
        rows.append(
            {
                "slug": fullname.strip(),
                "rating": int(rating),
                "listed_size": int(size),
                "created_at": created.strip(),
                "title": title.strip(),
                "list_tags": tags,
            }
        )
    return rows, prov


def cmd_candidates(args):
    scp, prov_scp = list_pages("+scp +jp", args.scp_limit)
    scp = [r for r in scp if re.fullmatch(r"scp-\d+-jp", r["slug"])]
    tales, prov_tale = list_pages("+tale +jp", args.tale_limit)
    for r in scp:
        r["kind"] = "scp"
    for r in tales:
        r["kind"] = "tale"
    write_json(
        os.path.join(DATA, "candidates.json"),
        {"site": SITE, "provenance": [prov_scp, prov_tale], "pages": scp + tales},
    )
    print(f"{len(scp)} mainlist scp, {len(tales)} tale candidates")


# --------------------------------------------------------------------------
# capture

CJK = r"[　-ヿ㐀-䶿一-鿿豈-﫿＀-￯]"
FEATURES = {
    "blocks": r"\[\[(?!\[)",
    "bold": r"\*\*",
    "italic": r"(?<!:)//",
    "underline": r"__",
    "strike": r"(?<![-<!])--(?![->])",
    "sup_sub": r"\^\^|,,",
    "mono": r"\{\{|\}\}",
    "raw": r"@@|@<",
    "color": r"##\w+\|",
    "span": r"\[\[span\b",
    "div": r"\[\[div\b",
    "size": r"\[\[size\b",
    "footnote": r"\[\[footnote\]\]",
    "collapsible": r"\[\[collapsible\b",
    "tabview": r"\[\[tab(view)?\b",
    "table": r"^\|\|",
    "quote": r"^>",
    "list": r"^\s*[*#] ",
    "heading": r"^\++ ",
    "hr": r"^-{4,}\s*$",
    "link3": r"\[\[\[",
    "html": r"\[\[html\b",
    "module": r"\[\[module\b",
    "include": r"\[\[include\b",
    "image": r"\[\[(f?[<>=]?image)\b",
    "ruby": r"\[\[(ruby|rb|rt|rubytext)\b|class=\"[^\"]*\bruby\b",
    "anchor": r"\[\[#|\[\[a(_| )|\[\[\[#|\[#\s|\[\[toc",
    "cjk_emphasis": CJK + r"(\*\*|//|__|--)|(\*\*|//|__|--)" + CJK,
    "fullwidth_space": "　",
    "cjk_dash_pair": CJK + r"--" + CJK,
    "ellipsis_dash": r"──|―|…",
    "iftags": r"\[\[iftags\b",
    "css": r"\[\[module css\]\]|\[\[module CSS\]\]",
}
FEATURE_RES = {k: re.compile(v, re.MULTILINE | re.IGNORECASE) for k, v in FEATURES.items()}
MARKUP_KEYS = [k for k in FEATURES if k not in ("fullwidth_space", "ellipsis_dash")]


def formatting_stats(source):
    counts = {k: len(r.findall(source)) for k, r in FEATURE_RES.items()}
    markup = sum(counts[k] for k in MARKUP_KEYS)
    chars = max(len(source), 1)
    return {
        "counts": counts,
        "markup_tokens": markup,
        "density_per_kchar": round(markup * 1000 / chars, 2),
    }


def cache_path(cache, slug):
    return os.path.join(cache, "pages", f"{slug.replace(':', '__')}.json")


def capture_page(cache, slug, refresh=False):
    path = cache_path(cache, slug)
    if os.path.exists(path) and not refresh:
        return read_json(path)
    live = wd.page_html(SITE, slug)
    record = {"slug": slug, "live": live}
    if live["page_id"] is not None:
        source, prov = wd.view_source(SITE, live["page_id"])
        record["source"] = source
        record["source_provenance"] = prov
    write_json(path, record)
    return record


def cmd_capture(args):
    cand_path = os.path.join(DATA, "candidates.json")
    cands = read_json(cand_path)
    for i, row in enumerate(cands["pages"]):
        rec = capture_page(args.cache, row["slug"])
        src = rec.get("source")
        if src is None:
            row["capture_error"] = f"HTTP {rec['live']['status']}"
            continue
        row["page_id"] = rec["live"]["page_id"]
        row["decoded_size"] = len(src)
        row["size_matches"] = len(src) == row["listed_size"]
        row["stats"] = formatting_stats(src)
        print(f"[{i + 1}/{len(cands['pages'])}] {row['slug']} size_ok={row['size_matches']} density={row['stats']['density_per_kchar']}")
    write_json(cand_path, cands)


# --------------------------------------------------------------------------
# select


def cmd_select(args):
    cands = read_json(os.path.join(DATA, "candidates.json"))["pages"]
    ok = [r for r in cands if r.get("stats") and r.get("size_matches")]

    def score(r):
        s = r["stats"]
        c = s["counts"]
        jp = 3 * min(c["ruby"], 10) + 2 * min(c["anchor"], 10) + min(c["cjk_emphasis"], 40) / 4
        return s["density_per_kchar"] * math.log10(max(r["rating"], 10)) + jp

    chosen = []
    for kind, n in (("scp", args.scp), ("tale", args.tale)):
        pool = sorted((r for r in ok if r["kind"] == kind), key=score, reverse=True)
        chosen += pool[:n]
    # Always keep any page that uses ruby or explicit anchors, the JP focus areas.
    have = {r["slug"] for r in chosen}
    for r in sorted(ok, key=score, reverse=True):
        c = r["stats"]["counts"]
        if r["slug"] not in have and (c["ruby"] or c["anchor"] >= 3) and len(chosen) < args.scp + args.tale + args.extra:
            chosen.append(r)
            have.add(r["slug"])
    for r in chosen:
        r["selection_score"] = round(score(r), 2)
    write_json(os.path.join(DATA, "pages.json"), {"site": SITE, "pages": chosen})
    for r in chosen:
        c = r["stats"]["counts"]
        print(f"{r['kind']:4} {r['slug']:45} rating={r['rating']:5} density={r['stats']['density_per_kchar']:6} ruby={c['ruby']} anchor={c['anchor']} cjk_em={c['cjk_emphasis']}")


# --------------------------------------------------------------------------
# materialize

INCLUDE_RE = re.compile(r"^(?:[ \t]*>)*\[\[\s*include\s+([^\s\]|]+)", re.IGNORECASE | re.MULTILINE)


def include_dir(site, page):
    return os.path.join(DATA, "includes", site, page.replace(":", "__"))


def fetch_include(cache, site, page, index):
    key = f"{site}:{page}"
    if key in index:
        return index[key]
    out = include_dir(site, page)
    if os.path.exists(os.path.join(out, "meta.json")):
        meta = read_json(os.path.join(out, "meta.json"))
    else:
        live = wd.page_html(site, page)
        meta = {"site": site, "page": page, "exists": live["page_id"] is not None, "live_provenance": live["provenance"]}
        if meta["exists"]:
            source, prov = wd.view_source(site, live["page_id"])
            meta["page_id"] = live["page_id"]
            meta["source_provenance"] = prov
            meta["source_sha256"] = wd.sha256(source)
            write_text(os.path.join(out, "source.ftml"), source)
        write_json(os.path.join(out, "meta.json"), meta)
    index[key] = meta
    if meta["exists"]:
        with open(os.path.join(out, "source.ftml"), encoding="utf-8") as fh:
            prefetch_includes(cache, fh.read(), site, index)
    return meta


def prefetch_includes(cache, source, site, index):
    for m in INCLUDE_RE.finditer(source):
        inc_site, inc_page = wd.split_page_ref(m.group(1), site)
        if inc_page:
            fetch_include(cache, inc_site, inc_page, index)


def cmd_materialize(args):
    pages = read_json(os.path.join(DATA, "pages.json"))["pages"]
    index = {}
    for row in pages:
        rec = capture_page(args.cache, row["slug"])
        out = os.path.join(DATA, "pages", row["slug"])
        write_text(os.path.join(out, "source.ftml"), rec["source"])
        write_text(os.path.join(out, "live.html"), rec["live"]["page_content"])
        meta = {
            "site": SITE,
            "slug": row["slug"],
            "title": row["title"],
            "kind": row["kind"],
            "rating": row["rating"],
            "page_id": rec["live"]["page_id"],
            "wikirequest_info": rec["live"]["info"],
            "live_provenance": rec["live"]["provenance"],
            "source_provenance": rec["source_provenance"],
            "source_sha256": wd.sha256(rec["source"]),
            "live_page_content_sha256": wd.sha256(rec["live"]["page_content"]),
            "listed_size": row["listed_size"],
            "stats": row["stats"],
        }
        write_json(os.path.join(out, "meta.json"), meta)
        prefetch_includes(args.cache, rec["source"], SITE, index)
        print(f"materialized {row['slug']} ({len(index)} include targets so far)")
    write_json(
        os.path.join(DATA, "includes", "index.json"),
        {k: {"exists": v["exists"], "page_id": v.get("page_id"), "source_sha256": v.get("source_sha256")} for k, v in sorted(index.items())},
    )


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--cache", default=os.environ.get("SWEEP_CACHE", os.path.join(ROOT, ".cache")))
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("candidates")
    c.add_argument("--scp-limit", type=int, default=150)
    c.add_argument("--tale-limit", type=int, default=80)
    sub.add_parser("capture")
    s = sub.add_parser("select")
    s.add_argument("--scp", type=int, default=20)
    s.add_argument("--tale", type=int, default=14)
    s.add_argument("--extra", type=int, default=6)
    sub.add_parser("materialize")
    args = ap.parse_args()
    {"candidates": cmd_candidates, "capture": cmd_capture, "select": cmd_select, "materialize": cmd_materialize}[args.cmd](args)


if __name__ == "__main__":
    main()
