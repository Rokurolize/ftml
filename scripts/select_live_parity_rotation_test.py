import json
import tempfile
import unittest
from pathlib import Path

from scripts.select_live_parity_rotation import load_candidates, select_rotation, syntax_case


class LiveParityRotationTest(unittest.TestCase):
    def test_load_candidates_uses_only_exact_preview_matches(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cases = root / "cases.jsonl"
            bindings = root / "bindings.json"
            rows = [
                {"case_id":"b","source":"B","execution_class":"saved-page-batch"},
                {"case_id":"a","source":"A","execution_class":"page-preview-isolated"},
                {"case_id":"c","source":"C","execution_class":"wikijump-runtime"},
                {"case_id":"d","source":"D","execution_class":"saved-page-batch"},
            ]
            cases.write_text("\n".join(json.dumps(row) for row in rows) + "\n", encoding="utf-8")
            bindings.write_text(
                json.dumps({"bindings":[
                    {"case_id":"a","status":"match"},
                    {"case_id":"b","status":"match"},
                    {"case_id":"d","status":"mismatch"},
                ]}),
                encoding="utf-8",
            )
            self.assertEqual(
                [row["case_id"] for row in load_candidates(cases, bindings)],
                ["a", "b"],
            )

    def test_rotation_wraps_and_advances_without_reordering(self):
        candidates = [{"case_id": str(index)} for index in range(5)]
        self.assertEqual(
            [row["case_id"] for row in select_rotation(candidates, 0, 3)],
            ["0", "1", "2"],
        )
        self.assertEqual(
            [row["case_id"] for row in select_rotation(candidates, 1, 3)],
            ["3", "4", "0"],
        )

    def test_syntax_case_contains_no_execution_classification(self):
        row = syntax_case({"case_id":"x","source":"[[span]]x[[/span]]"})
        self.assertEqual(row["schema"], "wikijump_syntax_differential.syntax_case.v1")
        self.assertEqual(row["case_id"], "x")
        self.assertNotIn("execution_class", row)

    def test_jsonl_loader_does_not_split_unicode_line_separators(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cases = root / "cases.jsonl"
            bindings = root / "bindings.json"
            source = "before\u2028after\u2029end"
            cases.write_text(
                json.dumps(
                    {
                        "case_id": "unicode-separators",
                        "source": source,
                        "execution_class": "saved-page-batch",
                    },
                    ensure_ascii=False,
                )
                + "\n",
                encoding="utf-8",
            )
            bindings.write_text(
                json.dumps(
                    {"bindings": [{"case_id": "unicode-separators", "status": "match"}]}
                ),
                encoding="utf-8",
            )
            loaded = load_candidates(cases, bindings)
            self.assertEqual(loaded[0]["source"], source)


if __name__ == "__main__":
    unittest.main()
