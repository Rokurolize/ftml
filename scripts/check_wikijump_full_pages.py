#!/usr/bin/env python3

import argparse
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "tests/fixtures/wikidot-parity/downstream-pages.json"
INPUT_SCHEMA = "wikijump_syntax_differential.syntax_case.v1"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_cases(wikijump_root: Path, manifest_path: Path = MANIFEST) -> list[dict]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    cases = []
    for page in manifest["pages"]:
        path = wikijump_root / page["path"]
        if not path.is_file():
            raise ValueError(f"missing downstream page: {path}")
        actual = sha256(path)
        if actual != page["sha256"]:
            raise ValueError(
                f"downstream source drift for {page['id']}: expected {page['sha256']}, got {actual}"
            )
        source = path.read_text(encoding="utf-8")
        for layout in ("wikidot", "wikijump"):
            cases.append(
                {
                    "schema": INPUT_SCHEMA,
                    "case_id": f"downstream--{page['id']}--{layout}",
                    "source": source,
                    "title": page["title"],
                    "page_context": {"site": page["site"], "page": page["page"]},
                    "layout": layout,
                }
            )
    return cases


def run_renderer(renderer: Path, cases: list[dict], timeout: int = 120) -> list[dict]:
    payload = "".join(json.dumps(case, ensure_ascii=False) + "\n" for case in cases)
    completed = subprocess.run(
        [str(renderer)],
        input=payload,
        text=True,
        capture_output=True,
        check=False,
        timeout=timeout,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"renderer failed with exit {completed.returncode}: {completed.stderr[-4000:]}"
        )
    rows = [json.loads(line) for line in completed.stdout.splitlines() if line.strip()]
    if len(rows) != len(cases):
        raise RuntimeError(f"renderer returned {len(rows)} rows for {len(cases)} cases")
    errors = [row for row in rows if row.get("status") != "rendered"]
    if errors:
        raise RuntimeError(f"downstream render failures: {errors!r}")
    return rows


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Run pinned Wikijump full pages through the FTML public pipeline in both layouts."
    )
    parser.add_argument("--wikijump-root", required=True, type=Path)
    parser.add_argument("--renderer", type=Path)
    parser.add_argument("--build-renderer", action="store_true")
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()

    wikijump_root = args.wikijump_root.resolve()
    renderer = args.renderer
    if args.build_renderer:
        subprocess.run(
            ["cargo", "build", "--example", "render_html_jsonl"], cwd=ROOT, check=True
        )
        renderer = ROOT / "target/debug/examples/render_html_jsonl"
    if renderer is None:
        renderer = ROOT / "target/debug/examples/render_html_jsonl"
    renderer = renderer.resolve()
    if not renderer.is_file():
        raise SystemExit(f"renderer not found: {renderer}; pass --build-renderer")

    try:
        cases = load_cases(wikijump_root)
        rows = run_renderer(renderer, cases, args.timeout)
    except (ValueError, RuntimeError, subprocess.TimeoutExpired) as error:
        raise SystemExit(str(error)) from error

    print(
        json.dumps(
            {
                "pages": len(cases) // 2,
                "layout_runs": len(rows),
                "parse_errors": sum(len(row.get("parse_errors", [])) for row in rows),
                "status": "pass",
            },
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
