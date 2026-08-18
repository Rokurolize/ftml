import json
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/build_ftml_fuzz_corpus.py"


class BuildFtmlFuzzCorpusTest(unittest.TestCase):
    def test_stable_inventory_is_deduplicated_by_source_hash(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "corpus"
            completed = subprocess.run(
                ["python3", str(SCRIPT), "--output", str(output)],
                cwd=ROOT,
                check=True,
                text=True,
                capture_output=True,
            )
            summary = json.loads(completed.stdout)
            self.assertGreater(summary["stable_cases_added"], 0)
            self.assertEqual(summary["wikijump_seeds_added"], 0)
            self.assertEqual(summary["unique_seed_files"], len(list(output.iterdir())))

            stable_sources = {
                json.loads(line)["source"]
                for line in (ROOT / "tests/fixtures/wikidot-parity/cases.jsonl")
                .read_text(encoding="utf-8")
                .split("\n")
                if line.strip()
            }
            self.assertEqual(summary["unique_seed_files"], len(stable_sources))


if __name__ == "__main__":
    unittest.main()
