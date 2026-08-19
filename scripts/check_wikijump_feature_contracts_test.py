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
            ledger.write_text(
                json.dumps({"features": {"syntax-comments": {}}}), encoding="utf-8"
            )
            contract = root / "contracts.json"
            contract.write_text(
                json.dumps(
                    {
                        "schema": "ftml.wikijump_feature_contracts.v2",
                        "contracts": [
                            {
                                "id": "other",
                                "wikijump_feature_id": "syntax-other",
                                "cases": ["test--bold--native"],
                                "property_owners": {
                                    f"P{index}": "ftml" for index in range(1, 9)
                                },
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            errors = check(root, contract)
            self.assertTrue(any("missing Wikijump syntax" in error for error in errors))

    def test_missing_property_axis_is_reported(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            ledger = root / "docs/wikidot-specifications/implementation-ledger.json"
            ledger.parent.mkdir(parents=True)
            ledger.write_text(
                json.dumps({"features": {"syntax-comments": {}}}), encoding="utf-8"
            )
            contract = root / "contracts.json"
            contract.write_text(
                json.dumps(
                    {
                        "schema": "ftml.wikijump_feature_contracts.v2",
                        "contracts": [
                            {
                                "id": "comments",
                                "wikijump_feature_id": "syntax-comments",
                                "cases": ["test--bold--native"],
                                "property_owners": {"P1": "ftml"},
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            errors = check(root, contract)
            self.assertTrue(any("P1-P8" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
