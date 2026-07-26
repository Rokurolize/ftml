import unittest

import check_conf


class CompareModuleDataTests(unittest.TestCase):
    def test_alias_drift_fails(self):
        module_conf = {
            "listpages": {
                "aliases": frozenset(["listpages", "pages"]),
            },
        }
        module_rules = {
            "listpages": {
                "aliases": frozenset(["listpages"]),
            },
        }

        self.assertFalse(check_conf.compare_module_data(module_conf, module_rules))


class RuleRegexTests(unittest.TestCase):
    def test_wikidot_image_alignment_aliases_do_not_require_separate_docs(self):
        for alias in ["f<image", "f=image", "f>image"]:
            self.assertFalse(check_conf.check_block_alias_in_doc(alias))

    def test_block_aliases_accept_rustfmt_multiline_arrays(self):
        source = """pub const BLOCK_IMAGE: BlockRule = BlockRule {
    name: "block-image",
    accepts_names: &[
        "image", "=image", "f=image",
    ],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: false,
    parse_fn,
};"""

        match = check_conf.BLOCK_RULE_REGEX.search(source)

        self.assertIsNotNone(match)
        self.assertEqual(eval(match[2]), ["image", "=image", "f=image"])


if __name__ == "__main__":
    unittest.main()
