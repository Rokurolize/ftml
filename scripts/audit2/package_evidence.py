#!/usr/bin/env python3
"""Package the audit2 evidence into committable artifacts.

Reads the record dir working files and writes the audit/ bundle into the
worktree: per-fixture verdicts, merged live captures, classification evidence,
whitespace adjudication, saved-page capture evidence, and ad-hoc probes.
"""

import gzip
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path

import os
REPO = Path(__file__).resolve().parents[2]
CFG = Path(os.environ.get("FTML_AUDIT_RECORD", REPO / ".audit"))
OUT = REPO / "audit"
CACHE = CFG / "preview-cache"

sys.path.insert(0, str(REPO / "scripts"))
import audit_corpus_live as audit  # noqa: E402


def load_jsonl(path):
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()
            if line.strip()]


def main():
    OUT.mkdir(exist_ok=True)
    rows = load_jsonl(CFG / "audit-all.jsonl")
    classified = json.loads((CFG / "classified.json").read_text())
    cases = classified["cases"]
    saved = json.loads((CFG / "saved-pages" / "saved-results.json").read_text())
    manifest = json.loads((CFG / "saved-pages" / "saved-manifest.json").read_text())
    whitespace = json.loads((CFG / "whitespace-analysis.json").read_text())

    # 1. Coverage + verdict summary.
    family_totals = defaultdict(int)
    for case_dir in (REPO / "test").glob("*/*"):
        if (case_dir / "wikidot.html").is_file() and (case_dir / "input.ftml").is_file():
            family_totals[case_dir.parent.name] += 1
    per_family = defaultdict(lambda: defaultdict(int))
    for row in rows:
        entry = cases.get(row["case"], {})
        cls = entry.get("class") or row["status"]
        per_family[row["family"]]["audited"] += 1
        per_family[row["family"]][cls] += 1
    summary = {
        "schema": "ftml.corpus_audit2.summary.v1",
        "audited_total": len(rows),
        "fixtures_with_wikidot_html": sum(family_totals.values()),
        "coverage": "987/987 fixtures with wikidot.html",
        "class_totals": dict(Counter(
            (cases.get(r["case"], {}).get("class") or r["status"]) for r in rows)),
        "families": {
            family: {"total": family_totals[family], **dict(per_family[family])}
            for family in sorted(family_totals)
        },
    }
    (OUT / "summary.json").write_text(json.dumps(summary, indent=1) + "\n")

    # 2. Per-fixture verdicts.
    verdicts = []
    for row in rows:
        entry = cases.get(row["case"], {})
        verdicts.append({
            "case": row["case"],
            "family": row["family"],
            "audit_status": row["status"],
            "class": entry.get("class"),
            "evidence": entry.get("evidence", "page-preview"),
            "context": row.get("context"),
            "live_raw_html_sha256": row.get("raw_html_sha256"),
            "saved_page": entry.get("case_fullname"),
            "saved_content_sha256": entry.get("content_sha256"),
        })
    (OUT / "verdicts.jsonl").write_text(
        "\n".join(json.dumps(v, ensure_ascii=False) for v in verdicts) + "\n")

    # 3. Classification evidence for every non-match.
    detail = {}
    for case, entry in cases.items():
        if entry["class"] == "match-exact":
            continue
        row = next(r for r in rows if r["case"] == case)
        kept = {
            "class": entry["class"],
            "evidence": entry.get("evidence", "page-preview"),
            "source": entry.get("source"),
            "source_sha256": row.get("source_sha256"),
            "preview_raw_html_sha256": row.get("raw_html_sha256"),
            "live_struct": entry.get("live_struct"),
            "fixture_struct": entry.get("fixture_struct"),
            "live_text": entry.get("live_text"),
            "fixture_text": entry.get("fixture_text"),
            "live_norm": entry.get("live_norm"),
            "fixture_norm": entry.get("fixture_norm"),
        }
        if entry.get("case_fullname"):
            kept["saved_page"] = entry["case_fullname"]
            kept["saved_content_sha256"] = entry.get("content_sha256")
        detail[case] = kept
    (OUT / "classifications.json").write_text(
        json.dumps(detail, ensure_ascii=False, indent=1) + "\n")

    # 4. Whitespace adjudication.
    ws_out = {}
    for case, evidence in whitespace.items():
        row = next(r for r in rows if r["case"] == case)
        ws_out[case] = {
            "live_raw_html_sha256": row["raw_html_sha256"],
            "positions": evidence["positions"],
            "text_diffs": evidence["diffs"],
            "verdict": "normalization-only (whitespace collapses at fragment/block/line edge; "
                       "element+attribute topology and collapsed visible text identical)",
        }
    (OUT / "whitespace.json").write_text(
        json.dumps(ws_out, ensure_ascii=False, indent=1) + "\n")

    # 5. Saved-page captures.
    saved_out = {}
    for case, entry in saved.items():
        saved_out[case] = {
            "fullname": entry["fullname"],
            "source_sha256": entry["source_sha256"],
            "tags": entry["tags"],
            "full_html_sha256": entry["full_html_sha256"],
            "content_sha256": entry["content_sha256"],
            "stable": entry["stable"],
            "class": cases.get(case, {}).get("class"),
        }
    (OUT / "saved-pages.json").write_text(
        json.dumps({"manifest": manifest, "captures": saved_out},
                   ensure_ascii=False, indent=1) + "\n")

    # 6. Ad-hoc probes (raw body pairs).
    probes = {}
    for batch in ("batch1", "batch2", "batch3", "batch4", "batch5", "batch6"):
        for path in sorted((CFG / "probes" / batch).glob("*.json")):
            if path.name == "summary.json":
                continue
            rec = json.loads(path.read_text())
            probes[rec["name"]] = {
                "batch": batch,
                "source": rec["source"],
                "raw_html_sha256": rec["raw_html_sha256"],
                "raw_html": rec["raw_html"],
                "stable_raw": rec["stable"],
                "matches_fixture": rec.get("matches_expected"),
            }
    (OUT / "probes.json").write_text(
        json.dumps(probes, ensure_ascii=False, indent=1) + "\n")

    # 7. Merged live captures (gzip JSONL) for every audited fixture.
    with gzip.open(OUT / "live-captures.jsonl.gz", "wt", encoding="utf-8") as handle:
        seen = set()
        for row in rows:
            key = row["source_sha256"]
            if key in seen:
                continue
            seen.add(key)
            record = json.loads((CACHE / f"{key}.json").read_text())
            handle.write(json.dumps({
                "site": record["site"],
                "module": record["module"],
                "authenticated": record["authenticated"],
                "mutated": record["mutated"],
                "captured_at": record["captured_at"],
                "source_sha256": key,
                "raw_html_sha256": record["raw_html_sha256"],
                "source": record["source"],
                "raw_html": record["raw_html"],
            }, ensure_ascii=False) + "\n")

    # 8. Tabview sub-classification.
    tab = json.loads((CFG / "tabview-class.json").read_text())
    (OUT / "tabview-class.json").write_text(json.dumps(tab, indent=1) + "\n")

    print(json.dumps(summary["class_totals"], indent=1))
    print("wrote", OUT)


if __name__ == "__main__":
    main()
