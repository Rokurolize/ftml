import json
import tempfile
import unittest
from pathlib import Path

from scripts.check_wikijump_feature_contracts import check


class WikijumpFeatureContractsTest(unittest.TestCase):
    def test_missing_feature_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            ledger = root / "docs/wikidot-specifications/implementation-ledger.json"
            ledger.parent.mkdir(parents=True)
            ledger.write_text(json.dumps({"features": {}}), encoding="utf-8")
            contract = root / "contracts.json"
            contract.write_text(
                json.dumps(
                    {
                        "contracts": [
                            {
                                "id": "comments",
                                "wikijump_feature_id": "syntax-comments",
                                "cases": [],
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            errors = check(root, contract)
            self.assertTrue(any("syntax-comments" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
