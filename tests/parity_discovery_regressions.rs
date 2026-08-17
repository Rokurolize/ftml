use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn parity_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("wikidot-parity")
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line).unwrap_or_else(|error| {
                panic!("invalid JSONL row in {}: {error}", path.display())
            })
        })
        .collect()
}

fn bindings() -> HashMap<String, Value> {
    let path = parity_root().join("bindings.json");
    let document: Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("failed to read {}: {error}", path.display())
        }))
        .unwrap_or_else(|error| panic!("invalid {}: {error}", path.display()));

    document["bindings"]
        .as_array()
        .expect("bindings must be an array")
        .iter()
        .map(|binding| {
            (
                binding["case_id"]
                    .as_str()
                    .expect("binding case_id must be a string")
                    .to_owned(),
                binding.clone(),
            )
        })
        .collect()
}

fn inventory() -> HashMap<String, Value> {
    read_jsonl(&parity_root().join("cases.jsonl"))
        .into_iter()
        .map(|case| {
            (
                case["case_id"]
                    .as_str()
                    .expect("case_id must be a string")
                    .to_owned(),
                case,
            )
        })
        .collect()
}

#[test]
fn recorder_discovery_batches_remain_bound_to_their_frozen_observations() {
    let inventory = inventory();
    let bindings = bindings();
    let mut seen = HashSet::new();
    let mut paths = fs::read_dir(parity_root())
        .expect("failed to list parity fixtures")
        .map(|entry| entry.expect("failed to read parity fixture entry").path())
        .filter(|path| {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                return false;
            };
            let Some(suffix) = name
                .strip_prefix("references-20260816-")
                .and_then(|suffix| suffix.strip_suffix(".jsonl"))
                .and_then(|suffix| suffix.parse::<u32>().ok())
            else {
                return false;
            };
            suffix >= 4
        })
        .collect::<Vec<_>>();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "discovery campaign has no frozen reference batches"
    );

    for path in paths {
        for reference in read_jsonl(&path) {
            let case_id = reference["syntax_case"]["case_id"]
                .as_str()
                .expect("reference case_id must be a string");
            assert!(
                seen.insert(case_id.to_owned()),
                "duplicate discovery case {case_id}"
            );

            let case = inventory.get(case_id).unwrap_or_else(|| {
                panic!("frozen reference {case_id} is missing from inventory")
            });
            let binding = bindings.get(case_id).unwrap_or_else(|| {
                panic!("frozen reference {case_id} is missing a binding")
            });

            assert_eq!(
                case["source_sha256"], reference["source_sha256"],
                "inventory/reference source drift for {case_id}"
            );
            assert_eq!(
                binding["source_sha256"], reference["source_sha256"],
                "binding/reference source drift for {case_id}"
            );
            assert_eq!(
                binding["wikidot_html_sha256"], reference["raw_html_sha256"],
                "binding/reference Wikidot HTML drift for {case_id}"
            );
            assert_eq!(reference["provenance"]["module"], "edit/PagePreviewModule");
            assert_eq!(reference["provenance"]["authenticated"], false);
            assert_eq!(reference["provenance"]["mutated"], false);
        }
    }

    assert!(
        !seen.is_empty(),
        "discovery batches unexpectedly contain no cases"
    );
}

#[test]
fn discovery_mismatches_keep_an_explicit_disposition_and_no_snapshot() {
    let inventory = inventory();
    let bindings = bindings();

    for (case_id, case) in &inventory {
        let source_path = match case["source_origin"]["path"].as_str() {
            Some(path)
                if path.contains("/recorded")
                    || path.contains("/limit-")
                    || path.contains("/list-depth-")
                    || path.contains("/line-owner-nbsp-")
                    || path.contains("/typography-")
                    || path.contains("/whitespace-") =>
            {
                path
            }
            _ => continue,
        };
        let execution_class = case["execution_class"].as_str().unwrap_or_default();
        if !matches!(
            execution_class,
            "saved-page-batch" | "page-preview-isolated"
        ) {
            continue;
        }

        let binding = bindings.get(case_id).unwrap_or_else(|| {
            panic!("recorded discovery case {case_id} has no binding")
        });
        if binding["status"] == "match" {
            continue;
        }

        let disposition = binding["disposition"]
            .as_str()
            .unwrap_or_else(|| panic!("mismatch {case_id} has no disposition"));
        assert!(
            matches!(
                disposition,
                "unresolved" | "caller-runtime" | "comparison-normalization"
            ),
            "unexpected mismatch disposition {disposition:?} for {case_id}"
        );
        if disposition == "unresolved" {
            assert!(
                binding["reason"]
                    .as_str()
                    .unwrap_or_default()
                    .contains("issue #"),
                "unresolved mismatch {case_id} lost its issue ownership"
            );
        }

        let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(source_path)
            .parent()
            .expect("fixture source must have a parent")
            .to_path_buf();
        assert!(
            !fixture_dir.join("wikidot.html").exists(),
            "mismatch {case_id} must not have a passing wikidot.html snapshot"
        );
    }
}

#[test]
fn newly_discovered_root_cause_families_stay_owned_by_their_issues() {
    let bindings = bindings();
    let families = [
        (
            547,
            [
                "tests--fixtures--parity-gaps--line-owner-nbsp-code",
                "tests--fixtures--parity-gaps--line-owner-nbsp-heading",
                "tests--fixtures--parity-gaps--line-owner-nbsp-list",
            ]
            .as_slice(),
        ),
        (
            564,
            [
                "tests--fixtures--parity-gaps--recorded2-pf-empty-hash-name",
                "tests--fixtures--parity-gaps--recorded4-pf-time",
                "tests--fixtures--parity-gaps--recorded6-pf-unclosed-unknown",
            ]
            .as_slice(),
        ),
        (
            565,
            [
                "tests--fixtures--parity-gaps--recorded6-pf-branch-span",
                "tests--fixtures--parity-gaps--recorded6-pf-nested-if",
            ]
            .as_slice(),
        ),
        (
            566,
            [
                "tests--fixtures--parity-gaps--recorded9-empty-span-bare",
                "tests--fixtures--parity-gaps--recorded10-div-empty-key-multiline-code",
                "tests--fixtures--parity-gaps--recorded10-span-empty-key-nested-bold",
            ]
            .as_slice(),
        ),
        (
            567,
            [
                "tests--fixtures--parity-gaps--recorded9-empty-div-unsafe",
                "tests--fixtures--parity-gaps--recorded10-div-empty-key-nested-link",
            ]
            .as_slice(),
        ),
        (
            570,
            [
                "tests--fixtures--parity-gaps--recorded15-pf-abs-arity",
                "tests--fixtures--parity-gaps--recorded15-pf-unicode-error",
            ]
            .as_slice(),
        ),
        (
            571,
            ["tests--fixtures--parity-gaps--recorded15-pf-parentheses-depth-33"]
                .as_slice(),
        ),
        (
            573,
            [
                "tests--fixtures--parity-gaps--recorded16-pf-min-one",
                "tests--fixtures--parity-gaps--recorded16-pf-max-one",
            ]
            .as_slice(),
        ),
        (
            574,
            ["tests--fixtures--parity-gaps--recorded17-triple-anchor-adjacent-run5"]
                .as_slice(),
        ),
        (
            575,
            [
                "tests--fixtures--parity-gaps--recorded18-code-valid-after-crossed",
                "tests--fixtures--parity-gaps--recorded18-math-valid-after-crossed",
            ]
            .as_slice(),
        ),
        (
            576,
            ["tests--fixtures--parity-gaps--recorded19-center-div-127-boundary"]
                .as_slice(),
        ),
        (
            577,
            ["tests--fixtures--parity-gaps--limit-center-collapsible-prelude-32-boundary"]
                .as_slice(),
        ),
        (
            578,
            ["tests--fixtures--parity-gaps--limit-single-link-scheme-65-boundary"]
                .as_slice(),
        ),
        (
            579,
            ["tests--fixtures--parity-gaps--limit-blockquote-depth-32-boundary"]
                .as_slice(),
        ),
        (
            580,
            ["tests--fixtures--parity-gaps--limit-inline-scope-depth-101-boundary"]
                .as_slice(),
        ),
        (
            581,
            [
                "tests--fixtures--parity-gaps--typography-generic-attribute-owner",
                "tests--fixtures--parity-gaps--typography-generic-attribute-unit-space",
                "tests--fixtures--parity-gaps--typography-single-link-label-number-space",
            ]
            .as_slice(),
        ),
        (
            582,
            [
                "tests--fixtures--parity-gaps--typography-generic-attribute-ellipsis",
                "tests--fixtures--parity-gaps--typography-generic-attribute-angle-quotes",
                "tests--fixtures--parity-gaps--typography-getattrs-attribute-ellipsis",
                "tests--fixtures--parity-gaps--typography-getattrs-attribute-angle-quotes",
                "tests--fixtures--parity-gaps--typography-single-link-label-ellipsis",
                "tests--fixtures--parity-gaps--typography-autourl-ellipsis",
                "tests--fixtures--parity-gaps--typography-single-link-target-ellipsis",
            ]
            .as_slice(),
        ),
        (
            583,
            ["tests--fixtures--parity-gaps--list-depth-21-boundary"]
                .as_slice(),
        ),
        (
            618,
            [
                "tests--fixtures--parity-gaps--typography-code-body-number-space",
                "tests--fixtures--parity-gaps--typography-raw-body-number-space",
                "tests--fixtures--parity-gaps--typography-code-body-unit-space",
                "tests--fixtures--parity-gaps--typography-code-body-quotes",
            ]
            .as_slice(),
        ),
        (
            619,
            ["tests--fixtures--parity-gaps--whitespace-raw-body-tab"]
                .as_slice(),
        ),
        (
            620,
            ["tests--fixtures--parity-gaps--whitespace-replacement-marker-eof"]
                .as_slice(),
        ),
        (
            621,
            ["tests--fixtures--parity-gaps--autourl-angle-at-termination"]
                .as_slice(),
        ),
    ];

    for (issue, case_ids) in families {
        for case_id in case_ids {
            let binding = bindings
                .get(*case_id)
                .unwrap_or_else(|| panic!("missing root-cause witness {case_id}"));
            assert_eq!(
                binding["status"], "mismatch",
                "{case_id} unexpectedly stopped being a mismatch without review"
            );
            assert_eq!(
                binding["disposition"], "unresolved",
                "{case_id} lost unresolved ownership"
            );
            assert!(
                binding["reason"]
                    .as_str()
                    .unwrap_or_default()
                    .contains(&format!("issue #{issue}")),
                "{case_id} is no longer owned by issue #{issue}"
            );
        }
    }
}

#[test]
fn independent_discovery_controls_remain_exact_matches() {
    let bindings = bindings();
    for case_id in [
        "tests--fixtures--parity-gaps--recorded5-raw-six-ats",
        "tests--fixtures--parity-gaps--recorded7-link-comment-target",
        "tests--fixtures--parity-gaps--recorded8-cross-bold-italics-underline",
        "tests--fixtures--parity-gaps--recorded11-div-stripslashes",
        "tests--fixtures--parity-gaps--recorded14-invalid-code-link",
        "tests--fixtures--parity-gaps--recorded14-invalid-math-footnote",
        "tests--fixtures--parity-gaps--recorded15-pf-parentheses-depth-32-control",
        "tests--fixtures--parity-gaps--recorded16-pf-min-two-control",
        "tests--fixtures--parity-gaps--recorded17-triple-anchor-separated-control",
        "tests--fixtures--parity-gaps--recorded18-code-crossed-only-control",
        "tests--fixtures--parity-gaps--recorded19-center-div-126-control",
        "tests--fixtures--parity-gaps--limit-blockquote-depth-31-control",
        "tests--fixtures--parity-gaps--limit-center-collapsible-prelude-31-control",
        "tests--fixtures--parity-gaps--limit-inline-scope-depth-100-control",
        "tests--fixtures--parity-gaps--limit-single-link-scheme-64-control",
        "tests--fixtures--parity-gaps--nameless-attribute-anchor-consumer",
        "tests--fixtures--parity-gaps--nameless-attribute-table-consumer",
        "tests--fixtures--parity-gaps--rollback-hidden-unclosed-footnote",
        "tests--fixtures--parity-gaps--rollback-naked-cell-bold",
        "tests--fixtures--parity-gaps--single-link-closer-run-named-anchor-three",
        "tests--fixtures--parity-gaps--typography-getattrs-attribute-owner",
        "tests--fixtures--parity-gaps--typography-single-link-label-angle-quotes",
        "tests--fixtures--parity-gaps--typography-prose-number-space",
        "tests--fixtures--parity-gaps--typography-mono-body-number-space",
        "tests--fixtures--parity-gaps--typography-code-body-ellipsis",
        "tests--fixtures--parity-gaps--typography-mono-body-angle-quotes",
        "tests--fixtures--parity-gaps--typography-mono-body-ellipsis",
        "tests--fixtures--parity-gaps--whitespace-prose-tab",
        "tests--fixtures--parity-gaps--whitespace-code-body-tab",
        "tests--fixtures--parity-gaps--whitespace-mono-body-tab",
        "tests--fixtures--parity-gaps--whitespace-code-body-nbsp",
        "tests--fixtures--parity-gaps--whitespace-replacement-marker-mid",
        "tests--fixtures--parity-gaps--whitespace-replacement-marker-line",
        "tests--fixtures--parity-gaps--whitespace-terminal-backslash-eof",
        "tests--fixtures--parity-gaps--whitespace-terminal-backslash-double-eof",
        "tests--fixtures--parity-gaps--whitespace-continuation-joins",
        "tests--fixtures--parity-gaps--whitespace-continued-span-opener",
        "tests--fixtures--parity-gaps--whitespace-document-leading-indented-quote",
        "tests--fixtures--parity-gaps--whitespace-nul-prose",
        "tests--fixtures--parity-gaps--whitespace-crlf-code-body",
        "tests--fixtures--parity-gaps--whitespace-mac-newline",
        "tests--fixtures--parity-gaps--autourl-double-bracket-prefix",
        "tests--fixtures--parity-gaps--module-keyword-uppercase",
        "tests--fixtures--parity-gaps--module-closer-uppercase",
        "tests--fixtures--parity-gaps--code-raw-body-markers",
        "tests--fixtures--parity-gaps--raw-alt-url-body",
    ] {
        let binding = bindings
            .get(case_id)
            .unwrap_or_else(|| panic!("missing discovery control {case_id}"));
        assert_eq!(
            binding["status"], "match",
            "discovery control {case_id} regressed"
        );
        assert_eq!(binding["checks"]["dom_tree"], "match");
        assert_eq!(binding["checks"]["dom_signature"], "match");
        assert_eq!(binding["checks"]["visible_text"], "match");
    }
}
