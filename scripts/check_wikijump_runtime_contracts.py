#!/usr/bin/env python3

import argparse
import json
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tests/fixtures/wikidot-parity/caller-runtime-contracts.json"


def source_path_for_test(wikijump_root: Path, test_id: str) -> Path:
    parts = test_id.split("::")
    if parts[:2] != ["services", "render"] or len(parts) < 5:
        raise ValueError(f"unsupported Wikijump render test path: {test_id}")
    module_parts = [part for part in parts[2:-1] if part != "tests"]
    base = wikijump_root / "deepwell/src/services/render"
    if module_parts == ["service"]:
        return base / "service/tests.rs"
    return base.joinpath(*module_parts).with_suffix(".rs")


def declares_test(path: Path, function_name: str) -> bool:
    if not path.is_file():
        return False
    source = path.read_text(encoding="utf-8")
    return re.search(
        rf"(?m)^\s*(?:async\s+)?fn\s+{re.escape(function_name)}\s*\(", source
    ) is not None


def check_contracts(wikijump_root: Path, manifest_path: Path = MANIFEST) -> list[str]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    errors = []
    for contract in manifest["contracts"]:
        test_id = contract["wikijump_test"]
        function_name = test_id.rsplit("::", 1)[1]
        try:
            path = source_path_for_test(wikijump_root, test_id)
        except ValueError as error:
            errors.append(str(error))
            continue
        if not declares_test(path, function_name):
            errors.append(
                f"{contract['id']}: {test_id} not found at {path.relative_to(wikijump_root)}"
            )
    return errors


def run_contract_tests(wikijump_root: Path, manifest_path: Path = MANIFEST) -> None:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    cargo_manifest = wikijump_root / "deepwell/Cargo.toml"
    for contract in manifest["contracts"]:
        subprocess.run(
            [
                "cargo",
                "test",
                "--manifest-path",
                str(cargo_manifest),
                contract["wikijump_test"],
                "--",
                "--exact",
                "--nocapture",
            ],
            cwd=wikijump_root,
            check=True,
        )


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Verify that every FTML caller-runtime contract still names a concrete Wikijump test."
    )
    parser.add_argument("--wikijump-root", required=True, type=Path)
    parser.add_argument(
        "--run-tests",
        action="store_true",
        help="Run every named Wikijump test after verifying that it exists.",
    )
    args = parser.parse_args()
    wikijump_root = args.wikijump_root.resolve()
    errors = check_contracts(wikijump_root)
    if errors:
        raise SystemExit("\n".join(errors))
    if args.run_tests:
        run_contract_tests(wikijump_root)
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    print(
        json.dumps(
            {
                "contracts": len(manifest["contracts"]),
                "caller_runtime_cases": sum(
                    len(contract["cases"]) for contract in manifest["contracts"]
                ),
                "status": "pass",
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
