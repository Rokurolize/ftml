#!/usr/bin/env python3

import argparse
import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CASES = ROOT / "tests/fixtures/wikidot-parity/cases.jsonl"


def write_seed(output: Path, source: str) -> bool:
    encoded = source.encode("utf-8")
    digest = hashlib.sha256(encoded).hexdigest()
    target = output / digest
    if target.exists():
        return False
    target.write_bytes(encoded)
    return True


def load_stable_cases(output: Path) -> int:
    added = 0
    # JSON strings may contain authored U+2028/U+2029. Python's splitlines()
    # treats those code points as record separators even though JSONL does not.
    for line in CASES.read_text(encoding="utf-8").split("\n"):
        if not line.strip():
            continue
        case = json.loads(line)
        added += write_seed(output, case["source"])
    return added


def load_wikijump_seed_pages(output: Path, wikijump_root: Path) -> int:
    seeder = wikijump_root / "deepwell/seeder"
    if not seeder.is_dir():
        raise SystemExit(f"Wikijump seeder directory not found: {seeder}")

    added = 0
    for path in sorted(seeder.glob("*.ftml")):
        added += write_seed(output, path.read_text(encoding="utf-8"))
    return added


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Build a generated libFuzzer seed corpus from stable FTML parity cases."
    )
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument(
        "--wikijump-root",
        type=Path,
        help="Optionally add every full .ftml page/component from Wikijump deepwell/seeder.",
    )
    args = parser.parse_args()

    args.output.mkdir(parents=True, exist_ok=True)
    stable_added = load_stable_cases(args.output)
    wikijump_added = 0
    if args.wikijump_root is not None:
        wikijump_added = load_wikijump_seed_pages(
            args.output, args.wikijump_root.resolve()
        )

    print(
        json.dumps(
            {
                "stable_cases_added": stable_added,
                "wikijump_seeds_added": wikijump_added,
                "unique_seed_files": len(list(args.output.iterdir())),
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
