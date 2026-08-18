#!/usr/bin/env python3

import argparse
import json
from pathlib import Path


MINIMUM_BRANCH_PERCENT = {
    "src/parsing/consume.rs": 80.0,
    "src/parsing/rule/impls/block/parser.rs": 68.0,
    "src/parsing/rule/impls/url.rs": 78.0,
    "src/preproc/compatibility.rs": 70.0,
}


def branch_percentages(report: dict) -> dict[str, float]:
    percentages: dict[str, float] = {}
    for data in report.get("data", []):
        for file in data.get("files", []):
            filename = file["filename"].replace("\\", "/")
            for suffix in MINIMUM_BRANCH_PERCENT:
                if filename.endswith(suffix):
                    percentages[suffix] = file["summary"]["branches"]["percent"]
    return percentages


def check(report: dict) -> list[str]:
    percentages = branch_percentages(report)
    errors = []
    for path, minimum in MINIMUM_BRANCH_PERCENT.items():
        actual = percentages.get(path)
        if actual is None:
            errors.append(f"{path}: missing from coverage report")
        elif actual < minimum:
            errors.append(f"{path}: branch coverage {actual:.2f}% < {minimum:.2f}%")
    return errors


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Check branch-coverage floors for high-risk FTML compatibility paths."
    )
    parser.add_argument("report", type=Path)
    args = parser.parse_args()
    report = json.loads(args.report.read_text(encoding="utf-8"))
    errors = check(report)
    if errors:
        raise SystemExit("\n".join(errors))
    print(
        json.dumps(
            {"status": "pass", "branches": branch_percentages(report)},
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
