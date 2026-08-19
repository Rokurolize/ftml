#!/usr/bin/env python3

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CASES = ROOT / "tests/fixtures/wikidot-parity/cases.jsonl"
BINDINGS = ROOT / "tests/fixtures/wikidot-parity/bindings.json"
SYNTAX_CASE_SCHEMA = "wikijump_syntax_differential.syntax_case.v1"
PREVIEW_CLASSES = {"saved-page-batch", "page-preview-isolated"}


def load_candidates(cases_path: Path = CASES, bindings_path: Path = BINDINGS) -> list[dict]:
    bindings = {
        row["case_id"]: row
        for row in json.loads(bindings_path.read_text(encoding="utf-8"))["bindings"]
    }
    candidates = []
    # JSONL is LF-delimited. str.splitlines() would also split valid U+2028 and
    # U+2029 characters embedded inside a JSON string and corrupt that record.
    for line in cases_path.read_text(encoding="utf-8").split("\n"):
        if not line.strip():
            continue
        case = json.loads(line)
        binding = bindings.get(case["case_id"])
        if (
            case["execution_class"] in PREVIEW_CLASSES
            and binding is not None
            and binding["status"] == "match"
        ):
            candidates.append(case)
    candidates.sort(key=lambda case: case["case_id"])
    return candidates


def select_rotation(candidates: list[dict], slot: int, count: int) -> list[dict]:
    if not candidates:
        raise ValueError("no exact-match preview candidates")
    if count <= 0:
        raise ValueError("count must be positive")
    count = min(count, len(candidates))
    start = (slot * count) % len(candidates)
    return [candidates[(start + offset) % len(candidates)] for offset in range(count)]


def syntax_case(case: dict) -> dict:
    return {
        "schema": SYNTAX_CASE_SCHEMA,
        "case_id": case["case_id"],
        "source": case["source"],
        "title": case["case_id"],
        "wikidot_observation_tier": "page-preview",
        "local_execution_tier": "ftml",
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Select a deterministic rotating sample of exact-match Wikidot parity cases."
    )
    parser.add_argument("--slot", type=int, required=True)
    parser.add_argument("--count", type=int, default=8)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    selected = select_rotation(load_candidates(), args.slot, args.count)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as output:
        for case in selected:
            output.write(json.dumps(syntax_case(case), ensure_ascii=False, separators=(",", ":")))
            output.write("\n")
    print(
        json.dumps(
            {
                "candidate_count": len(load_candidates()),
                "selected": len(selected),
                "slot": args.slot,
                "status": "pass",
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
