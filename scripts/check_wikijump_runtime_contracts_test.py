import tempfile
import unittest
from pathlib import Path

from scripts.check_wikijump_runtime_contracts import declares_test, source_path_for_test


class WikijumpRuntimeContractsTest(unittest.TestCase):
    def test_service_and_nested_module_paths_are_resolved(self):
        root = Path("/tmp/wikijump")
        self.assertEqual(
            source_path_for_test(
                root,
                "services::render::service::tests::example",
            ),
            root / "deepwell/src/services/render/service/tests.rs",
        )
        self.assertEqual(
            source_path_for_test(
                root,
                "services::render::compat::wikidot_gallery::tests::example",
            ),
            root / "deepwell/src/services/render/compat/wikidot_gallery.rs",
        )

    def test_function_declaration_check_accepts_test_functions_only_by_name(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "tests.rs"
            path.write_text(
                "#[test]\nfn exact_contract() {}\nasync fn async_contract() {}\n",
                encoding="utf-8",
            )
            self.assertTrue(declares_test(path, "exact_contract"))
            self.assertTrue(declares_test(path, "async_contract"))
            self.assertFalse(declares_test(path, "missing_contract"))


if __name__ == "__main__":
    unittest.main()
