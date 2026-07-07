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


if __name__ == "__main__":
    unittest.main()
