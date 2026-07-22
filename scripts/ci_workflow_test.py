import fnmatch
import pathlib
import re
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github" / "workflows" / "build.yaml"


def classifier_rules(workflow):
    classifier = workflow.split("\n  classify:\n", maxsplit=1)[1].split("\n  library_build_and_test:\n", maxsplit=1)[0]
    case_body = classifier.split('case "${path}" in', maxsplit=1)[1].split("              esac", maxsplit=1)[0]
    rules = []
    lines = iter(case_body.splitlines())

    for line in lines:
        stripped = line.strip()
        if not stripped.endswith(")"):
            continue
        patterns = stripped[:-1].split("|")
        assignments = set()
        for body_line in lines:
            body = body_line.strip()
            if body == ";;":
                break
            if body in {"rust=true", "configuration=true"}:
                assignments.add(body)
        rules.append((patterns, assignments))

    return rules


def classify_paths(workflow, paths):
    rust = False
    configuration = False
    rules = classifier_rules(workflow)

    for path in paths:
        for patterns, assignments in rules:
            if any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns):
                rust |= "rust=true" in assignments
                configuration |= "configuration=true" in assignments
                break
        else:
            raise AssertionError(f"classifier has no terminal rule for {path}")

    return rust, configuration


def job_section(workflow, job):
    match = re.search(rf"^  {re.escape(job)}:\n(.*?)(?=^  [a-z][a-z0-9_]*:\n|\Z)", workflow, flags=re.MULTILINE | re.DOTALL)
    if match is None:
        raise AssertionError(f"workflow job not found: {job}")
    return match.group(1)


def classify_event(event_name, action, draft, labels=(), changed_label=None, base_changed=False):
    label_event = event_name == "pull_request" and action in {"labeled", "unlabeled"}
    edit_event = event_name == "pull_request" and action == "edited"
    full_ci_label_event = action == "labeled" and changed_label == "full-ci"
    run_ci = not label_event and (not edit_event or base_changed)
    candidate = event_name == "push" or run_ci and not draft
    full_coverage = event_name == "push" or not draft and "full-ci" in labels and (run_ci or full_ci_label_event)

    if event_name != "pull_request":
        gate_name = "CI / PR gate (inactive)"
    elif not run_ci and not draft:
        gate_name = "CI / gate"
    elif not run_ci:
        gate_name = "CI / draft gate"
    elif not draft and candidate:
        gate_name = "CI / gate"
    else:
        gate_name = "CI / draft gate"

    return run_ci, candidate, full_coverage, gate_name


class CiWorkflowPolicyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")

    def test_pull_request_workflow_has_distinct_draft_candidate_and_inactive_contexts(self):
        header = self.workflow.split("\njobs:\n", maxsplit=1)[0]
        gate = job_section(self.workflow, "pr_gate")
        main_gate = job_section(self.workflow, "main_gate")

        self.assertIn("\n  pull_request:\n", header)
        self.assertNotIn("paths:", header)
        for activity in ("edited", "ready_for_review", "converted_to_draft", "labeled", "unlabeled"):
            self.assertIn(f"      - {activity}\n", header)
        self.assertIn("github.event.pull_request.number || github.run_id", header)
        self.assertIn("&& 'active' || github.run_id", header)
        self.assertIn("github.event.action != 'labeled'", header)
        self.assertIn("github.event.action != 'unlabeled'", header)
        self.assertIn("github.event.changes.base.ref.from != ''", header)
        self.assertIn("'CI / PR gate (inactive)'", gate)
        self.assertIn("'CI / draft gate'", gate)
        self.assertIn("github.event.pull_request.draft == false && (needs.classify.outputs.run_ci != 'true' || needs.classify.outputs.candidate == 'true') && 'CI / gate'", gate)
        self.assertIn("if: ${{ always() }}", gate)
        self.assertIn("if: ${{ github.event_name == 'pull_request' && needs.classify.outputs.run_ci == 'true' }}", gate)
        self.assertIn("Preserve required PR gate on metadata-only event", gate)
        self.assertIn("check_name='CI / gate'", gate)
        self.assertIn('.app.slug == \"github-actions\" and .conclusion == \"success\"', gate)
        self.assertIn("Record inactive PR gate", gate)
        for required_job in ("classify", "rust_unit", "library_build_and_test", "wasm", "coverage", "clippy_lint", "configuration_check"):
            self.assertIn(f"      - {required_job}\n", gate)
        for main_only_job in ("upload_coverage", "upload_test_results"):
            self.assertNotIn(f"      - {main_only_job}\n", gate)

        self.assertIn("'CI / gate' || 'CI / main gate (inactive)'", main_gate)
        self.assertIn("if: ${{ always() }}", main_gate)
        self.assertIn("if: ${{ github.event_name == 'push' && github.ref == 'refs/heads/main' }}", main_gate)
        self.assertIn("if: ${{ github.event_name != 'push' || github.ref != 'refs/heads/main' }}", main_gate)
        self.assertIn("Record inactive main gate", main_gate)
        for required_job in ("coverage", "upload_coverage", "upload_test_results"):
            self.assertIn(f"      - {required_job}\n", main_gate)

    def test_event_policy_isolates_label_and_non_base_edit_changes(self):
        self.assertEqual(classify_event("pull_request", "synchronize", True), (True, False, False, "CI / draft gate"))
        self.assertEqual(classify_event("pull_request", "ready_for_review", False), (True, True, False, "CI / gate"))
        self.assertEqual(classify_event("pull_request", "labeled", True, labels=("landing",), changed_label="landing"), (False, False, False, "CI / draft gate"))
        self.assertEqual(classify_event("pull_request", "labeled", False, labels=("landing",), changed_label="landing"), (False, False, False, "CI / gate"))
        self.assertEqual(classify_event("pull_request", "labeled", True, labels=("full-ci",), changed_label="full-ci"), (False, False, False, "CI / draft gate"))
        self.assertEqual(classify_event("pull_request", "labeled", False, labels=("full-ci",), changed_label="full-ci"), (False, False, True, "CI / gate"))
        self.assertEqual(classify_event("pull_request", "labeled", False, labels=("landing", "full-ci"), changed_label="landing"), (False, False, False, "CI / gate"))
        self.assertEqual(classify_event("pull_request", "labeled", False, labels=("landing", "full-ci", "documentation"), changed_label="documentation"), (False, False, False, "CI / gate"))
        self.assertEqual(classify_event("pull_request", "unlabeled", False, labels=("landing",), changed_label="documentation"), (False, False, False, "CI / gate"))
        self.assertEqual(classify_event("pull_request", "synchronize", False, labels=("full-ci",)), (True, True, True, "CI / gate"))
        self.assertEqual(classify_event("pull_request", "edited", False), (False, False, False, "CI / gate"))
        self.assertEqual(classify_event("pull_request", "edited", False, base_changed=True), (True, True, False, "CI / gate"))
        self.assertEqual(classify_event("push", "push", False), (True, True, True, "CI / PR gate (inactive)"))

        classifier = job_section(self.workflow, "classify")
        self.assertIn("LABEL_EVENT:", classifier)
        self.assertIn("FULL_CI_LABEL_EVENT:", classifier)
        self.assertNotIn("CONTROL_LABEL:", classifier)
        self.assertNotIn("'landing'", classifier)
        self.assertIn("github.event.changes.base.ref.from != ''", classifier)
        self.assertIn("github.event.label.name == 'full-ci'", classifier)
        self.assertIn("github.event.pull_request.draft == false", classifier)
        self.assertIn('if [[ "${LABEL_EVENT}" == "true" ]]', classifier)
        self.assertIn("run_ci=false", classifier)
        self.assertIn('echo "run_ci=${run_ci}"', classifier)

        for job in ("rust_unit", "library_build_and_test", "wasm", "clippy_lint", "configuration_check"):
            self.assertIn("needs.classify.outputs.run_ci == 'true'", job_section(self.workflow, job))
        self.assertNotIn("needs.classify.outputs.run_ci", job_section(self.workflow, "coverage"))
        self.assertIn("needs.classify.outputs.run_ci == 'true'", job_section(self.workflow, "pr_gate"))
        self.assertIn("full-ci label run is coverage-only", self.workflow)

    def test_classifier_owns_all_build_and_configuration_inputs(self):
        classifier = self.workflow.split("\n  classify:\n", maxsplit=1)[1].split("\n  library_build_and_test:\n", maxsplit=1)[0]
        for path_pattern in ("Cargo.toml", "Cargo.lock", "build.rs", "rust-toolchain.toml", ".tarpaulin.toml", ".config/nextest.toml", "conf/*.toml", "src/*", "test/*", "tests/*", "docs/*.md", "scripts/*", ".github/workflows/build.yaml"):
            self.assertIn(path_pattern, classifier)
        self.assertIn("git diff --no-renames --name-only -z", classifier)

    def test_classifier_is_fail_closed_and_has_no_file_count_limit(self):
        self.assertEqual(classify_paths(self.workflow, ["README.md"]), (False, False))
        self.assertEqual(classify_paths(self.workflow, ["docs/Blocks.md"]), (False, True))
        self.assertEqual(classify_paths(self.workflow, [".gitattributes"]), (False, True))
        self.assertEqual(classify_paths(self.workflow, ["new-build-system.nix"]), (True, True))
        self.assertEqual(classify_paths(self.workflow, [".github/security/scanner.yml"]), (True, True))

        documentation_paths = [f"docs/generated-{index}.md" for index in range(350)]
        self.assertEqual(classify_paths(self.workflow, documentation_paths), (False, True))
        self.assertEqual(classify_paths(self.workflow, [*documentation_paths, "tests/late-change.rs"]), (True, True))

    def test_classifier_and_jobs_define_fast_candidate_and_coverage_tiers(self):
        classifier = job_section(self.workflow, "classify")
        rust_unit = job_section(self.workflow, "rust_unit")
        library = job_section(self.workflow, "library_build_and_test")
        wasm = job_section(self.workflow, "wasm")
        lint = job_section(self.workflow, "clippy_lint")
        coverage = job_section(self.workflow, "coverage")
        gate = job_section(self.workflow, "pr_gate")
        main_gate = job_section(self.workflow, "main_gate")

        self.assertIn("github.event.pull_request.draft == false", classifier)
        self.assertIn("contains(github.event.pull_request.labels.*.name, 'full-ci')", classifier)
        self.assertNotIn("'landing'", classifier)
        for output in ("run_ci", "candidate", "full_coverage", "cache_epoch"):
            self.assertIn(f"      {output}: ${{{{ steps.changes.outputs.{output} }}}}\n", classifier)
        self.assertIn("needs.classify.outputs.candidate != 'true'", rust_unit)
        self.assertIn("cargo test --lib --all-features", rust_unit)
        self.assertIn("needs.classify.outputs.candidate == 'true'", library)
        self.assertIn("needs.classify.outputs.candidate == 'true'", wasm)
        self.assertNotIn("needs.classify.outputs.candidate", lint)
        self.assertIn("needs.classify.outputs.full_coverage == 'true'", coverage)
        self.assertNotIn("needs.classify.outputs.rust == 'true'", coverage.split("\n    runs-on:", maxsplit=1)[0])
        self.assertIn('if [[ "${CANDIDATE_REQUIRED}" == "true" ]]', gate)
        self.assertIn('require_success "Rust Unit" "${RUST_UNIT_RESULT}"', gate)
        self.assertIn('require_success "Library" "${LIBRARY_RESULT}"', gate)
        self.assertIn('if [[ "${FULL_COVERAGE}" == "true" ]]', gate)
        self.assertIn('require_success "Coverage" "${COVERAGE_RESULT}"', gate)
        self.assertIn('require_skipped "Coverage" "${COVERAGE_RESULT}"', gate)
        self.assertIn('require_success "Test Results OIDC Export" "${TEST_RESULTS_RESULT}"', main_gate)
        self.assertIn('require_success "Coverage OIDC Export" "${COVERAGE_UPLOAD_RESULT}"', main_gate)
        self.assertIn('require_true "CI event classification" "${RUN_CI}"', main_gate)

    def test_external_actions_are_commit_pinned(self):
        actions = re.findall(r"^[ \t]*uses: ([^\s#]+)[ \t]*(?:#.*)?$", self.workflow, flags=re.MULTILINE)

        self.assertTrue(actions)
        for action in actions:
            revision = action.rsplit("@", maxsplit=1)[1]
            self.assertRegex(revision, r"^[0-9a-f]{40}$")

    def test_cache_writes_are_main_only_and_coverage_target_is_not_cached(self):
        self.assertNotIn("uses: actions/cache@", self.workflow)
        save_steps = re.findall(r"^[ \t]*- name: Save ", self.workflow, flags=re.MULTILINE)
        save_conditions = re.findall(r"^[ \t]*- name: Save [^\n]+\n[ \t]+if: ([^\n]+)\n[ \t]+uses: actions/cache/save@([0-9a-f]{40})[ \t]*(?:#.*)?$", self.workflow, flags=re.MULTILINE)
        coverage = job_section(self.workflow, "coverage")
        target_keys = re.findall(r"^[ \t]+key: ([^\n]*-target-v2-[^\n]*)$", self.workflow, flags=re.MULTILINE)

        self.assertEqual(len(save_steps), 5)
        self.assertEqual(len(save_conditions), 5)
        for condition, _ in save_conditions:
            self.assertIn("github.event_name == 'push'", condition)
            self.assertIn("github.ref == 'refs/heads/main'", condition)
            self.assertIn("outputs.cache-hit != 'true'", condition)
        self.assertEqual(coverage.count("uses: actions/cache/restore@"), 1)
        self.assertNotIn("uses: actions/cache/save@", coverage)
        self.assertIn("target is intentionally not cached", coverage)
        self.assertIn("cache_epoch=$(date -u +'%G-W%V')", self.workflow)
        self.assertEqual(len(target_keys), 4)
        for key in target_keys:
            self.assertIn("needs.classify.outputs.cache_epoch", key)
            self.assertNotIn("github.sha", key)
            dependency_prefix = key.rsplit("-${{ needs.classify.outputs.cache_epoch }}", maxsplit=1)[0] + "-"
            self.assertGreaterEqual(self.workflow.count(dependency_prefix), 2)


if __name__ == "__main__":
    unittest.main()
