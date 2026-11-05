#!/usr/bin/env python3
"""Render wikitext (argv[1] file or stdin) with the harness, Wikidot layout; print html."""
import json, subprocess, sys
src = open(sys.argv[1]).read() if len(sys.argv) > 1 else sys.stdin.read()
BIN = "/home/user/ftml/target/release/examples/render_html_jsonl"
row = {"schema": "wikijump_syntax_differential.syntax_case.v1", "case_id": "r", "source": src, "title": "r", "layout": "wikidot"}
r = json.loads(subprocess.run([BIN], input=json.dumps(row) + "\n", capture_output=True, text=True).stdout)
print(r["html"])
