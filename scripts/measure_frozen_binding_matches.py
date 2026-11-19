#!/usr/bin/env python3
"""Measure the current renderer against the frozen Wikidot binding oracle."""

import argparse
import importlib.util
import json
import subprocess
import sys
from pathlib import Path


def load_parity(root):
    sys.path.insert(0, str(root))
    sys.path.insert(0, str(root / "scripts"))
    spec = importlib.util.spec_from_file_location(
        "wikidot_parity", root / "scripts/wikidot_parity.py"
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def select_references(root, parity, bindings):
    cases = parity.load_cases(root)
    rows = []
    for path in sorted((root / "tests/fixtures/wikidot-parity").glob("references-*.jsonl")):
        for line in path.read_text(encoding="utf-8").split("\n"):
            if line.strip():
                rows.append(json.loads(line))
    selected = {}
    for row in rows:
        case_id = row.get("syntax_case", {}).get("case_id")
        if case_id not in bindings or case_id in selected:
            continue
        binding = bindings[case_id]
        if (
            row.get("syntax_case", {}).get("source") == cases[case_id]["source"]
            and row.get("source_sha256") == binding["source_sha256"]
            and row.get("raw_html_sha256") == binding["wikidot_html_sha256"]
        ):
            selected[case_id] = row
    missing = sorted(set(bindings) - set(selected))
    if missing:
        raise SystemExit(f"no frozen reference matches binding hash for {missing[:5]}")
    for row in selected.values():
        provenance = row.setdefault("provenance", {})
        provenance.setdefault("wikidot_py_version", "4.4.1")
        provenance.setdefault("wikidot_py_commit", "0" * 40)
        provenance.setdefault("requirements_sha256", "0" * 64)
    return selected


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--renderer", type=Path, required=True)
    parser.add_argument("--runner", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    parity = load_parity(args.root)
    binding_file = args.root / "tests/fixtures/wikidot-parity/bindings.json"
    bindings = {
        row["case_id"]: row
        for row in json.loads(binding_file.read_text(encoding="utf-8"))["bindings"]
    }
    selected = select_references(args.root, parity, bindings)
    references_path = args.output.with_suffix(".references.jsonl")
    references_path.write_text(
        "".join(json.dumps(row, ensure_ascii=False) + "\n" for row in selected.values()),
        encoding="utf-8",
    )
    verdict_path = args.output.with_suffix(".verdict.json")
    verdict_path.unlink(missing_ok=True)
    result = subprocess.run(
        [
            "node",
            str(args.runner),
            "--references",
            str(references_path),
            "--renderer",
            str(args.renderer),
            "--timeout-ms",
            "120000",
            "--output",
            str(verdict_path),
        ],
        capture_output=True,
        text=True,
    )
    if result.returncode not in (0, 1) or not verdict_path.exists():
        raise SystemExit(f"differential runner failed: {result.stderr[-2000:]}")
    comparisons = json.loads(verdict_path.read_text(encoding="utf-8"))["comparisons"]
    counts = {name: 0 for name in ("match", "mismatch", "runner-error", "not-applicable")}
    changed = []
    for comparison in comparisons:
        case_id = comparison["case_id"]
        counts[comparison["status"]] += 1
        if comparison["status"] != bindings[case_id]["status"]:
            changed.append((case_id, bindings[case_id]["status"], comparison["status"]))
    report = {"counts": counts, "changed_vs_bindings": changed}
    args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"total": len(comparisons), **counts, "changed": len(changed)}))


if __name__ == "__main__":
    main()
