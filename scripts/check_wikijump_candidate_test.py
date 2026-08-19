import tempfile
import unittest
from pathlib import Path

from scripts.check_wikijump_candidate import (
    FULL_PAGE_TEST,
    candidate_test_ids,
    local_ftml_dependency_line,
    patch_ftml_dependency,
    source_path_for_test,
    verify_test_declarations,
)


class WikijumpCandidateTest(unittest.TestCase):
    def test_candidate_suite_includes_full_page_and_runtime_contracts_once(self):
        tests = candidate_test_ids()
        self.assertEqual(tests[0], FULL_PAGE_TEST)
        self.assertEqual(len(tests), 7)
        self.assertEqual(len(tests), len(set(tests)))

    def test_local_dependency_preserves_non_source_fields(self):
        line = (
            'ftml = { git = "https://github.com/Rokurolize/ftml", '
            'rev = "abc", features = ["compiled"] }'
        )
        patched = local_ftml_dependency_line(line, Path("/tmp/ftml"))
        self.assertIn('path = "/tmp/ftml"', patched)
        self.assertIn('features = ["compiled"]', patched)
        self.assertNotIn("git =", patched)
        self.assertNotIn("rev =", patched)

    def test_patch_requires_one_dependency_and_rewrites_only_that_line(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "Cargo.toml"
            path.write_text(
                '[dependencies]\nftml = { git = "x", rev = "y" }\nserde = "1"\n',
                encoding="utf-8",
            )
            original = patch_ftml_dependency(path, Path("/tmp/candidate"))
            self.assertIn('ftml = { git = "x", rev = "y" }', original)
            updated = path.read_text(encoding="utf-8")
            self.assertIn('ftml = { path = "/tmp/candidate" }', updated)
            self.assertIn('serde = "1"', updated)

    def test_test_path_resolution_covers_service_and_nested_modules(self):
        root = Path("/tmp/wikijump")
        self.assertEqual(
            source_path_for_test(root, FULL_PAGE_TEST),
            root / "deepwell/src/services/render/service/tests.rs",
        )
        self.assertEqual(
            source_path_for_test(
                root,
                "services::render::compat::wikidot_gallery::tests::example",
            ),
            root / "deepwell/src/services/render/compat/wikidot_gallery.rs",
        )

    def test_declaration_check_rejects_missing_candidate_tests(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "deepwell/src/services/render/service/tests.rs"
            path.parent.mkdir(parents=True)
            path.write_text(
                "#[test]\nfn scp9506_full_seed_survives_the_ftml_wikidot_pipeline() {}\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(RuntimeError, "missing Wikijump candidate tests"):
                verify_test_declarations(root, candidate_test_ids())


if __name__ == "__main__":
    unittest.main()
