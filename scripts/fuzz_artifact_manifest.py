#!/usr/bin/env python3

import argparse
import hashlib
import json
from pathlib import Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def artifact_rows(artifact_dir: Path) -> list[dict]:
    rows = []
    if not artifact_dir.is_dir():
        return rows
    for path in sorted(path for path in artifact_dir.iterdir() if path.is_file()):
        data = path.read_bytes()
        try:
            data.decode("utf-8")
            utf8 = True
        except UnicodeDecodeError:
            utf8 = False
        rows.append(
            {
                "name": path.name,
                "sha256": sha256(path),
                "size": len(data),
                "valid_utf8": utf8,
                "reproduce": (
                    "cargo +nightly fuzz run --fuzz-dir fuzz public_pipeline -- "
                    f"-runs=1 -timeout=10 {artifact_dir.as_posix()}/{path.name}"
                ),
            }
        )
    return rows


def manifest(artifact_dir: Path, commit: str) -> dict:
    return {
        "schema": "ftml.fuzz_artifact_manifest.v1",
        "ftml_commit": commit,
        "target": "public_pipeline",
        "artifacts": artifact_rows(artifact_dir),
    }


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Describe public_pipeline fuzz artifacts with stable identities and reproduction commands."
    )
    parser.add_argument("--artifact-dir", required=True, type=Path)
    parser.add_argument("--commit", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    value = manifest(args.artifact_dir, args.commit)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"artifacts": len(value["artifacts"]), "status": "pass"}, sort_keys=True))


if __name__ == "__main__":
    main()
