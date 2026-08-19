import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from scripts.check_wikijump_full_pages import load_cases


class WikijumpFullPagesTest(unittest.TestCase):
    def test_manifest_requires_exact_sources_and_expands_both_layouts(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = "[[collapsible]]é[[/collapsible]]\n"
            page = root / "deepwell/seeder/example.ftml"
            page.parent.mkdir(parents=True)
            page.write_text(source, encoding="utf-8")
            digest = hashlib.sha256(source.encode()).hexdigest()
            manifest = root / "manifest.json"
            manifest.write_text(
                json.dumps(
                    {
                        "pages": [
                            {
                                "id": "example",
                                "path": "deepwell/seeder/example.ftml",
                                "sha256": digest,
                                "site": "scp-wiki",
                                "page": "example",
                                "title": "Example",
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            cases = load_cases(root, manifest)
            self.assertEqual([case["layout"] for case in cases], ["wikidot", "wikijump"])
            self.assertTrue(all(case["source"] == source for case in cases))

            page.write_text(source + "drift", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "source drift"):
                load_cases(root, manifest)


if __name__ == "__main__":
    unittest.main()
