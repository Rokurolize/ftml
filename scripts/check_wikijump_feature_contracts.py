#!/usr/bin/env python3

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTRACTS = ROOT / "tests/fixtures/wikidot-parity/wikijump-feature-contracts.json"
CASES = ROOT / "tests/fixtures/wikidot-parity/cases.jsonl"


def load_case_ids() -> set[str]:
    return {
        json.loads(line)["case_id"]
        for line in CASES.read_text(encoding="utf-8").split("\n")
        if line.strip()
    }


def check(wikijump_root: Path, contracts_path: Path = CONTRACTS) -> list[str]:
    ledger_path = wikijump_root / "docs/wikidot-specifications/implementation-ledger.json"
    if not ledger_path.is_file():
        return [f"missing Wikijump implementation ledger: {ledger_path}"]
    features = json.loads(ledger_path.read_text(encoding="utf-8"))["features"]
    cases = load_case_ids()
    contracts = json.loads(contracts_path.read_text(encoding="utf-8"))["contracts"]
    errors = []
    seen = set()
    for contract in contracts:
        cid = contract["id"]
        if cid in seen:
            errors.append(f"duplicate feature contract id: {cid}")
        seen.add(cid)
        feature = contract["wikijump_feature_id"]
        if feature not in features:
            errors.append(f"{cid}: Wikijump feature {feature} is absent from the ledger")
        for case_id in contract["cases"]:
            if case_id not in cases:
                errors.append(f"{cid}: FTML parity case does not exist: {case_id}")
    return errors


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Verify FTML parity property families against the adjacent Wikijump compatibility ledger."
    )
    parser.add_argument("--wikijump-root", required=True, type=Path)
    args = parser.parse_args()
    errors = check(args.wikijump_root.resolve())
    if errors:
        raise SystemExit("\n".join(errors))
    contracts = json.loads(CONTRACTS.read_text(encoding="utf-8"))["contracts"]
    print(json.dumps({"contracts": len(contracts), "status": "pass"}, sort_keys=True))


if __name__ == "__main__":
    main()
