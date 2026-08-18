import unittest

from scripts.check_robustness_branch_coverage import MINIMUM_BRANCH_PERCENT, check


def report_with(percentages):
    return {
        "data": [
            {
                "files": [
                    {
                        "filename": f"/checkout/{path}",
                        "summary": {"branches": {"percent": percent}},
                    }
                    for path, percent in percentages.items()
                ]
            }
        ]
    }


class RobustnessBranchCoverageTest(unittest.TestCase):
    def test_accepts_all_required_files_at_their_floor(self):
        self.assertEqual(check(report_with(MINIMUM_BRANCH_PERCENT)), [])

    def test_rejects_missing_and_undercovered_files(self):
        values = dict(MINIMUM_BRANCH_PERCENT)
        missing = next(iter(values))
        values.pop(missing)
        under = next(iter(values))
        values[under] -= 0.01
        errors = check(report_with(values))
        self.assertTrue(any(missing in error and "missing" in error for error in errors))
        self.assertTrue(any(under in error and "<" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
