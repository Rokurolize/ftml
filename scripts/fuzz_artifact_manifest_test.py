import hashlib
import tempfile
import unittest
from pathlib import Path

from scripts.fuzz_artifact_manifest import manifest


class FuzzArtifactManifestTest(unittest.TestCase):
    def test_manifest_is_sorted_hashed_and_marks_utf8(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "timeout-b").write_bytes(b"\xff\x00")
            (root / "crash-a").write_text("[[div]]", encoding="utf-8")
            value = manifest(root, "abc123")
            self.assertEqual(value["ftml_commit"], "abc123")
            self.assertEqual(
                [row["name"] for row in value["artifacts"]],
                ["crash-a", "timeout-b"],
            )
            first = value["artifacts"][0]
            self.assertEqual(first["sha256"], hashlib.sha256(b"[[div]]").hexdigest())
            self.assertTrue(first["valid_utf8"])
            self.assertFalse(value["artifacts"][1]["valid_utf8"])
            self.assertIn("-runs=1", first["reproduce"])

    def test_missing_directory_is_an_empty_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            value = manifest(Path(directory) / "missing", "deadbeef")
            self.assertEqual(value["artifacts"], [])


if __name__ == "__main__":
    unittest.main()
