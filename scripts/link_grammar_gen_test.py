#!/usr/bin/env python3
"""Tests for scripts/link_grammar_gen.py (Track T5)."""

import json
import unittest
from pathlib import Path

from link_grammar_gen import adjudicate_row, generate_cases


class LinkGrammarGenTest(unittest.TestCase):
    def test_findings_854_to_862_covered(self) -> None:
        cases = generate_cases()
        self.assertGreaterEqual(len(cases), 1500)
        seen_findings = set()
        for c in cases:
            seen_findings.update(c["findings"])
        for expected in ("854", "855", "856", "857", "858", "859", "862"):
            self.assertIn(expected, seen_findings, f"Missing finding {expected}")

    def test_adjudicate_row_classifications(self) -> None:
        eq = adjudicate_row(
            {"save_inadmissible": False, "source": "http://a", "family": "scheme_url"},
            "<p><a href=\"http://a\">http://a</a></p>",
            "<p><a href=\"http://a\">http://a</a></p>",
        )
        self.assertEqual(eq[0], "pinned_equal")

        inad = adjudicate_row(
            {"save_inadmissible": True, "source": "http://\x00b", "family": "scheme_url"},
            "<p>a</p>",
            "<p>b</p>",
        )
        self.assertEqual(inad[0], "out_of_scope_save_inadmissible")

        ssh_bug = adjudicate_row(
            {"save_inadmissible": False, "source": "ssh://[", "family": "scheme_url"},
            "<p>ssh://[</p>",
            '<p><a href="ssh://[">ssh://[</a></p>',
        )
        self.assertEqual(ssh_bug[0], "ftml_bug_candidate")

    def test_corpus_jsonl_exists_and_all_rows_adjudicated(self) -> None:
        corpus = Path(__file__).resolve().parent.parent / "verification/link-grammar/corpus.jsonl"
        if not corpus.exists():
            self.skipTest("corpus.jsonl not yet generated")
        rows = [json.loads(line) for line in corpus.read_text(encoding="utf-8").split("\n") if line.strip()]
        self.assertEqual(len(rows), len(generate_cases()))
        for r in rows:
            self.assertIn("adjudication", r)
            self.assertTrue(r["adjudication"])


if __name__ == "__main__":
    unittest.main()
