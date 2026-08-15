import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("wikidot_parity", Path(__file__).with_name("wikidot_parity.py"))
parity = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(parity)


class WikidotParityTest(unittest.TestCase):
    def test_registered_block_names_are_observed_from_stable_sources(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "conf").mkdir()
            (root / "conf/blocks.toml").write_text(
                '[alpha]\naliases = ["a"]\naccepts-star = true\nbody = "elements"\n'
                '[hidden-name]\nexclude-name = true\naliases = ["visible-alias"]\naccepts-score = true\n'
            )
            cases = {
                "one": {"source": "[[alpha*]] [[/a]] [[a]][[/alpha]] [[alpha]][[/alpha]] [[a]][[/a]]"},
                "two": {"source": "[[visible-alias_]]"},
            }

            self.assertEqual(
                parity.configured_block_names(root),
                {"alpha", "a", "visible-alias"},
            )
            self.assertEqual(
                parity.observed_block_names(cases),
                {"alpha", "a", "visible-alias"},
            )
            self.assertEqual(
                parity.missing_block_facets(root, cases),
                {"star-flag": set(), "score-flag": set(), "body-close": set()},
            )
            self.assertEqual(parity.missing_alias_pairs(root, cases), set())
            self.assertEqual(
                parity.missing_alias_pairs(
                    root,
                    {"separate": {"source": "[[alpha]][[/alpha]] [[a]][[/a]]"}},
                ),
                {"alpha:alpha->a", "alpha:a->alpha"},
            )

        stable_cases = parity.load_cases(
            ROOT / "tests/fixtures/wikidot-parity/cases.jsonl"
        )
        self.assertEqual(
            parity.configured_block_names(ROOT)
            - parity.observed_block_names(stable_cases),
            set(),
        )
        self.assertEqual(
            parity.missing_block_facets(ROOT, stable_cases),
            {"star-flag": set(), "score-flag": set(), "body-close": set()},
        )
        self.assertEqual(parity.missing_alias_pairs(ROOT, stable_cases), set())

    def test_real_live_case_projection_and_compact_binding(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, raw = "**alpha**", "<p><strong>alpha</strong></p>"
            case = {"schema": parity.LIVE_CASE_SCHEMA, "case_id": "bold", "source": source,
                    "source_sha256": parity.sha256(source), "execution_class": "saved-page-batch",
                    "source_origin": {"repository": "Rokurolize/ftml", "path": "test/bold/input.ftml"}}
            runtime = {**case, "case_id": "runtime", "execution_class": "wikijump-runtime"}
            reference = {"schema": parity.REFERENCE_SCHEMA, "syntax_case": parity.projected_case(case),
                         "source_sha256": parity.sha256(source), "raw_html": raw,
                         "raw_html_sha256": parity.sha256(raw),
                         "captured_at": "2026-08-15T00:00:00+00:00",
                         "provenance": {"module": "edit/PagePreviewModule", "authenticated": False,
                                        "mutated": False, "site": "sandbox-for-codex",
                                        "site_domain": "sandbox-for-codex.wikidot.com",
                                        "wikidot_py_version": "4.4.1", "wikidot_py_commit": "a" * 40,
                                        "requirements_sha256": "b" * 64}}
            comparison = {"schema": parity.COMPARISON_SCHEMA, "case_id": "bold", "status": "mismatch",
                          "checks": {name: {"status": "mismatch" if name == "dom_tree" else "match",
                                            "diagnostic": "not retained"} for name in parity.CHECKS},
                          "identities": {"source_sha256": parity.sha256(source),
                                         "wikidot_html_sha256": parity.sha256(raw),
                                         "ftml_html_sha256": parity.sha256("candidate")}}
            verdict = {"schema": parity.VERDICT_SCHEMA, "comparisons": [comparison],
                       "summary": {"total": 1, "match": 0, "mismatch": 1,
                                   "runner-error": 0, "not-applicable": 0}}
            (root / "cases.jsonl").write_text(f"{json.dumps(case)}\n{json.dumps(runtime)}\n")
            old_reference = json.loads(json.dumps(reference))
            old_reference["syntax_case"]["source"] = "**old**"
            old_reference["source_sha256"] = parity.sha256("**old**")
            (root / "references.json").write_text(
                f"{json.dumps(old_reference)}\n{json.dumps(reference)}\n")
            (root / "verdict.json").write_text(json.dumps(verdict))
            cases = parity.load_cases(root / "cases.jsonl")
            references = parity.load_references([root / "references.json"], cases, exact=True)
            bindings = parity.make_bindings(
                references, parity.load_verdicts([root / "verdict.json"], references))
            self.assertEqual([item["case_id"] for item in bindings["bindings"]], ["bold"])
            self.assertEqual(bindings["bindings"][0]["checks"], {
                "dom_tree": "mismatch", "dom_signature": "match", "visible_text": "match"})
            self.assertEqual(bindings["bindings"][0]["disposition"], "unresolved")
            (root / "bindings.json").write_text(json.dumps(bindings))
            with self.assertRaisesRegex(ValueError, "active functional issue"):
                parity.load_bindings(root / "bindings.json", cases, references)
            bindings["bindings"][0]["reason"] = "Active functional investigation: issue #123."
            (root / "bindings.json").write_text(json.dumps(bindings))
            parity.load_bindings(root / "bindings.json", cases, references)
            malformed = json.loads(json.dumps(bindings))
            malformed["bindings"][0]["reason"] = "Active functional investigation: issue #1abc"
            (root / "bindings.json").write_text(json.dumps(malformed))
            with self.assertRaisesRegex(ValueError, "active functional issue"):
                parity.load_bindings(root / "bindings.json", cases, references)
            contradictory = json.loads(json.dumps(bindings))
            contradictory["bindings"][0]["status"] = "match"
            del contradictory["bindings"][0]["disposition"]
            del contradictory["bindings"][0]["reason"]
            (root / "bindings.json").write_text(json.dumps(contradictory))
            with self.assertRaisesRegex(ValueError, "status does not match checks"):
                parity.load_bindings(root / "bindings.json", cases, references)
            invalid = json.loads(json.dumps(bindings))
            invalid["bindings"][0]["disposition"] = "arbitrary"
            (root / "bindings.json").write_text(json.dumps(invalid))
            with self.assertRaisesRegex(ValueError, "mismatch disposition is invalid"):
                parity.load_bindings(root / "bindings.json", cases, references)
            (root / "test/bold").mkdir(parents=True)
            (root / "test/bold/wikidot.html").touch()
            self.assertEqual(parity.snapshot_ids(cases, root), {"bold"})
            reference["raw_html_sha256"] = "0" * 64
            (root / "references.json").write_text(json.dumps(reference))
            with self.assertRaisesRegex(ValueError, "raw HTML hash"):
                parity.load_references([root / "references.json"], cases, exact=True)
            reference["raw_html_sha256"] = parity.sha256(raw)
            reference["provenance"]["mutated"] = True
            (root / "references.json").write_text(json.dumps(reference))
            with self.assertRaisesRegex(ValueError, "acquisition provenance"):
                parity.load_references([root / "references.json"], cases, exact=True)


if __name__ == "__main__":
    unittest.main()
