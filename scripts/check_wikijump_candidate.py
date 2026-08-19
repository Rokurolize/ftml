#!/usr/bin/env python3

import argparse
import json
import re
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
CALLER_RUNTIME_MANIFEST = (
    ROOT / "tests/fixtures/wikidot-parity/caller-runtime-contracts.json"
)
FULL_PAGE_TEST = (
    "services::render::service::tests::"
    "scp9506_full_seed_survives_the_ftml_wikidot_pipeline"
)


def source_path_for_test(wikijump_root: Path, test_id: str) -> Path:
    parts = test_id.split("::")
    if parts[:2] != ["services", "render"] or len(parts) < 5:
        raise ValueError(f"unsupported Wikijump render test path: {test_id}")
    module_parts = [part for part in parts[2:-1] if part != "tests"]
    base = wikijump_root / "deepwell/src/services/render"
    if module_parts == ["service"]:
        return base / "service/tests.rs"
    return base.joinpath(*module_parts).with_suffix(".rs")


def verify_test_declarations(wikijump_root: Path, tests: list[str]) -> None:
    missing = []
    for test_id in tests:
        path = source_path_for_test(wikijump_root, test_id)
        function_name = test_id.rsplit("::", 1)[1]
        if not path.is_file() or re.search(
            rf"(?m)^\s*(?:async\s+)?fn\s+{re.escape(function_name)}\s*\(",
            path.read_text(encoding="utf-8"),
        ) is None:
            missing.append(f"{test_id} ({path.relative_to(wikijump_root)})")
    if missing:
        raise RuntimeError("missing Wikijump candidate tests:\n" + "\n".join(missing))


def candidate_test_ids(
    caller_runtime_manifest: Path = CALLER_RUNTIME_MANIFEST,
) -> list[str]:
    manifest = json.loads(caller_runtime_manifest.read_text(encoding="utf-8"))
    tests = [FULL_PAGE_TEST]
    tests.extend(contract["wikijump_test"] for contract in manifest["contracts"])
    return list(dict.fromkeys(tests))


def local_ftml_dependency_line(original: str, ftml_root: Path) -> str:
    match = re.fullmatch(r"(\s*ftml\s*=\s*\{)(.*)(\}\s*)", original)
    if match is None:
        raise ValueError(f"unsupported FTML dependency line: {original!r}")

    body = match.group(2)
    parts = [part.strip() for part in body.split(",") if part.strip()]
    kept = [
        part
        for part in parts
        if not re.match(r"^(?:git|rev|path)\s*=", part)
    ]
    path = str(ftml_root.resolve()).replace("\\", "\\\\").replace('"', '\\"')
    fields = [f'path = "{path}"', *kept]
    return f"{match.group(1)} {' , '.join(fields)} {match.group(3)}"


def patch_ftml_dependency(cargo_toml: Path, ftml_root: Path) -> str:
    source = cargo_toml.read_text(encoding="utf-8")
    lines = source.splitlines(keepends=True)
    hits = []
    for index, line in enumerate(lines):
        raw = line.rstrip("\r\n")
        if re.match(r"^\s*ftml\s*=", raw):
            hits.append(index)
    if len(hits) != 1:
        raise ValueError(
            f"expected exactly one FTML dependency in {cargo_toml}, found {len(hits)}"
        )
    index = hits[0]
    ending = "\r\n" if lines[index].endswith("\r\n") else "\n"
    lines[index] = local_ftml_dependency_line(lines[index].rstrip("\r\n"), ftml_root) + ending
    cargo_toml.write_text("".join(lines), encoding="utf-8")
    return source


def verify_candidate_metadata(wikijump_root: Path, ftml_root: Path) -> None:
    completed = subprocess.run(
        [
            "cargo",
            "metadata",
            "--manifest-path",
            str(wikijump_root / "deepwell/Cargo.toml"),
            "--format-version",
            "1",
        ],
        cwd=wikijump_root,
        check=True,
        text=True,
        capture_output=True,
    )
    metadata = json.loads(completed.stdout)
    candidates = [package for package in metadata["packages"] if package["name"] == "ftml"]
    if len(candidates) != 1:
        raise RuntimeError(f"expected one FTML package in cargo metadata, found {len(candidates)}")
    manifest = Path(candidates[0]["manifest_path"]).resolve()
    expected = (ftml_root / "Cargo.toml").resolve()
    if manifest != expected or candidates[0].get("source") is not None:
        raise RuntimeError(
            f"Wikijump did not resolve the candidate FTML path: {manifest} source={candidates[0].get('source')!r}"
        )


def run_candidate_tests(wikijump_root: Path, tests: list[str]) -> None:
    manifest = wikijump_root / "deepwell/Cargo.toml"
    for test_id in tests:
        subprocess.run(
            [
                "cargo",
                "test",
                "--manifest-path",
                str(manifest),
                "--lib",
                test_id,
                "--",
                "--exact",
                "--nocapture",
            ],
            cwd=wikijump_root,
            check=True,
        )


def main() -> None:
    parser = argparse.ArgumentParser(
        description=(
            "Temporarily bind a clean Wikijump checkout to the current FTML checkout "
            "and run downstream compatibility contracts."
        )
    )
    parser.add_argument("--wikijump-root", required=True, type=Path)
    parser.add_argument("--ftml-root", type=Path, default=ROOT)
    parser.add_argument("--run-tests", action="store_true")
    args = parser.parse_args()

    wikijump_root = args.wikijump_root.resolve()
    ftml_root = args.ftml_root.resolve()
    cargo_toml = wikijump_root / "deepwell/Cargo.toml"
    cargo_lock = wikijump_root / "deepwell/Cargo.lock"
    if not cargo_toml.is_file():
        raise SystemExit(f"Wikijump Deepwell manifest not found: {cargo_toml}")
    if not (ftml_root / "Cargo.toml").is_file():
        raise SystemExit(f"FTML manifest not found: {ftml_root / 'Cargo.toml'}")

    original_toml = cargo_toml.read_bytes()
    original_lock = cargo_lock.read_bytes() if cargo_lock.is_file() else None
    tests = candidate_test_ids()
    try:
        verify_test_declarations(wikijump_root, tests)
        patch_ftml_dependency(cargo_toml, ftml_root)
        verify_candidate_metadata(wikijump_root, ftml_root)
        if args.run_tests:
            run_candidate_tests(wikijump_root, tests)
    finally:
        cargo_toml.write_bytes(original_toml)
        if original_lock is None:
            cargo_lock.unlink(missing_ok=True)
        else:
            cargo_lock.write_bytes(original_lock)

    print(
        json.dumps(
            {
                "candidate_tests": len(tests),
                "full_page_test": FULL_PAGE_TEST,
                "status": "pass",
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
