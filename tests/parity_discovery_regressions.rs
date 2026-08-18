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
            [("references-20260816-", 4), ("references-20260817-", 1)]
                .into_iter()
                .any(|(prefix, minimum)| {
                    name.strip_prefix(prefix)
                        .and_then(|suffix| suffix.strip_suffix(".jsonl"))
                        .and_then(|suffix| suffix.parse::<u32>().ok())
                        .is_some_and(|suffix| suffix >= minimum)
                })
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
            let source_sha256 = reference["source_sha256"]
                .as_str()
                .expect("reference source_sha256 must be a string");
            assert!(
                seen.insert((case_id.to_owned(), source_sha256.to_owned())),
                "duplicate discovery source revision {case_id} {source_sha256}"
            );

            let case = inventory.get(case_id).unwrap_or_else(|| {
                panic!("frozen reference {case_id} is missing from inventory")
            });
            assert_eq!(reference["provenance"]["module"], "edit/PagePreviewModule");
            assert_eq!(reference["provenance"]["authenticated"], false);
            assert_eq!(reference["provenance"]["mutated"], false);

            if case["source_sha256"] != reference["source_sha256"] {
                continue;
            }

            let binding = bindings.get(case_id).unwrap_or_else(|| {
                panic!("frozen reference {case_id} is missing a binding")
            });
            assert_eq!(
                binding["source_sha256"], reference["source_sha256"],
                "binding/reference source drift for {case_id}"
            );
            assert_eq!(
                binding["wikidot_html_sha256"], reference["raw_html_sha256"],
                "binding/reference Wikidot HTML drift for {case_id}"
            );
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
            Some(path) if path.starts_with("tests/fixtures/parity-gaps/") => path,
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
                "unresolved"
                    | "caller-runtime"
                    | "comparison-normalization"
                    | "security-boundary"
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
            293,
            ["tests--fixtures--parity-gaps--image-link-uppercase-tld"].as_slice(),
        ),
        (
            297,
            [
                "tests--fixtures--parity-gaps--limit-parser-function-conditional-scan-35-control",
                "tests--fixtures--parity-gaps--limit-parser-function-conditional-scan-36-boundary",
                "tests--fixtures--parity-gaps--pf-html-code-tag-control",
                "tests--fixtures--parity-gaps--pf-html-code-tag-leading-space",
                "tests--fixtures--parity-gaps--pf-literal-code-leading-head-space",
                "tests--fixtures--parity-gaps--pf-literal-code-leading-line-space",
                "tests--fixtures--parity-gaps--pf-literal-raw-leading-head-space",
            ]
            .as_slice(),
        ),
        (
            304,
            ["tests--fixtures--parity-gaps--preproc-quoted-lowercase-collapsible-unquoted-close"]
                .as_slice(),
        ),
        (
            318,
            ["tests--fixtures--parity-gaps--recorder-audit-bibliography-rollback-metadata"]
                .as_slice(),
        ),
        (
            340,
            [
                "tests--fixtures--parity-gaps--definition-term-edge-nbsp",
                "tests--fixtures--parity-gaps--definition-value-edge-nbsp",
            ]
            .as_slice(),
        ),
        (
            547,
            [
                "tests--fixtures--parity-gaps--line-owner-nbsp-code",
                "tests--fixtures--parity-gaps--line-owner-nbsp-heading",
                "tests--fixtures--parity-gaps--line-owner-nbsp-list",
                "tests--fixtures--parity-gaps--leading-nbsp-definition-owner",
                "tests--fixtures--parity-gaps--leading-nbsp-heading-owner",
                "tests--fixtures--parity-gaps--leading-nbsp-list-owner",
                "tests--fixtures--parity-gaps--leading-nbsp-quote-owner",
                "tests--fixtures--parity-gaps--leading-nbsp-table-owner",
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
                "tests--fixtures--parity-gaps--style-div-custom-property",
            ]
            .as_slice(),
        ),
        (
            570,
            [
                "tests--fixtures--parity-gaps--recorded15-pf-abs-arity",
                "tests--fixtures--parity-gaps--recorded15-pf-unicode-error",
                "tests--fixtures--parity-gaps--pf-expr-angle-not-equal",
                "tests--fixtures--parity-gaps--pf-expr-bitwise-and",
                "tests--fixtures--parity-gaps--pf-expr-bitwise-not",
                "tests--fixtures--parity-gaps--pf-expr-double-star-power",
                "tests--fixtures--parity-gaps--pf-expr-exponent-literal",
                "tests--fixtures--parity-gaps--pf-expr-hex-literal",
                "tests--fixtures--parity-gaps--pf-expr-shift-left",
                "tests--fixtures--parity-gaps--pf-expr-shift-right",
                "tests--fixtures--parity-gaps--pf-expr-string-double",
                "tests--fixtures--parity-gaps--pf-expr-string-equality",
                "tests--fixtures--parity-gaps--pf-expr-string-plus",
                "tests--fixtures--parity-gaps--pf-expr-string-single",
                "tests--fixtures--parity-gaps--pf-expr-ternary-operator",
                "tests--fixtures--parity-gaps--pf-expr-triple-equals",
                "tests--fixtures--parity-gaps--pf-expr-word-and",
                "tests--fixtures--parity-gaps--pf-expr-word-div",
                "tests--fixtures--parity-gaps--pf-expr-word-mod",
                "tests--fixtures--parity-gaps--pf-expr-word-or",
                "tests--fixtures--parity-gaps--pf-expr-word-xor",
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
        (
            622,
            ["tests--fixtures--parity-gaps--limit-date-format-bytes-537-boundary"]
                .as_slice(),
        ),
        (
            623,
            ["tests--fixtures--parity-gaps--limit-date-format-directives-65-boundary"]
                .as_slice(),
        ),
        (
            624,
            ["tests--fixtures--parity-gaps--limit-cross-collapsible-div-body-511-boundary"]
                .as_slice(),
        ),
        (
            625,
            [
                "tests--fixtures--parity-gaps--whitespace-raw-formfeed",
                "tests--fixtures--parity-gaps--whitespace-raw-nul",
                "tests--fixtures--parity-gaps--whitespace-raw-vtab",
            ]
            .as_slice(),
        ),
        (
            626,
            [
                "tests--fixtures--parity-gaps--whitespace-autolink-nul",
                "tests--fixtures--parity-gaps--whitespace-single-link-nul",
                "tests--fixtures--parity-gaps--whitespace-video-url-nul",
            ]
            .as_slice(),
        ),
        (
            627,
            ["tests--fixtures--parity-gaps--whitespace-math-inline-nul"]
                .as_slice(),
        ),
        (
            629,
            [
                "tests--fixtures--parity-gaps--whitespace-single-link-label-nul",
                "tests--fixtures--parity-gaps--whitespace-triple-link-label-nul",
            ]
            .as_slice(),
        ),
        (
            630,
            [
                "tests--fixtures--parity-gaps--whitespace-image-link-target-nul",
                "tests--fixtures--parity-gaps--whitespace-image-link-target-vtab",
            ]
            .as_slice(),
        ),
        (
            631,
            [
                "tests--fixtures--parity-gaps--image-source-leading-nbsp",
                "tests--fixtures--parity-gaps--whitespace-iframe-url-nul",
                "tests--fixtures--parity-gaps--whitespace-image-source-nul",
                "tests--fixtures--parity-gaps--whitespace-image-source-vtab",
            ]
            .as_slice(),
        ),
        (
            632,
            [
                "tests--fixtures--parity-gaps--whitespace-button-set-tags-text-nul",
                "tests--fixtures--parity-gaps--whitespace-button-text-nul",
                "tests--fixtures--parity-gaps--whitespace-button-text-vtab",
            ]
            .as_slice(),
        ),
        (
            633,
            [
                "tests--fixtures--parity-gaps--date-hover-short-true",
                "tests--fixtures--parity-gaps--date-hover-yes",
            ]
            .as_slice(),
        ),
        (
            634,
            [
                "tests--fixtures--parity-gaps--date-timestamp-lower-overflow",
                "tests--fixtures--parity-gaps--date-timestamp-upper-overflow",
            ]
            .as_slice(),
        ),
        (
            635,
            ["tests--fixtures--parity-gaps--date-timezone-seconds-7200"]
                .as_slice(),
        ),
        (
            636,
            [
                "tests--fixtures--parity-gaps--size-exponent-percent",
                "tests--fixtures--parity-gaps--size-infinite-percent",
                "tests--fixtures--parity-gaps--size-nan-percent",
                "tests--fixtures--parity-gaps--size-negative-px",
                "tests--fixtures--parity-gaps--size-plus-px",
            ]
            .as_slice(),
        ),
        (
            331,
            ["tests--fixtures--parity-gaps--button-style-unlisted-property"]
                .as_slice(),
        ),
        (
            339,
            ["tests--fixtures--parity-gaps--color-spec-edge-nbsp"].as_slice(),
        ),
        (
            343,
            ["tests--fixtures--parity-gaps--horizontal-rule-leading-nbsp"]
                .as_slice(),
        ),
        (
            476,
            ["tests--fixtures--parity-gaps--anchor-block-href-edge-nbsp"]
                .as_slice(),
        ),
        (
            638,
            [
                "tests--fixtures--parity-gaps--clear-float-leading-space",
                "tests--fixtures--parity-gaps--clear-float-left-trailing-text",
                "tests--fixtures--parity-gaps--clear-float-trailing-space-text",
                "tests--fixtures--parity-gaps--clear-float-trailing-text",
            ]
            .as_slice(),
        ),
        (
            639,
            [
                "tests--fixtures--parity-gaps--date-timezone-hour-24",
                "tests--fixtures--parity-gaps--date-timezone-minute-60",
                "tests--fixtures--parity-gaps--date-timezone-negative-zero",
            ]
            .as_slice(),
        ),
        (
            640,
            ["tests--fixtures--parity-gaps--pf-expr-comparison-chain"]
                .as_slice(),
        ),
        (
            641,
            ["tests--fixtures--parity-gaps--pf-expr-float-remainder"]
                .as_slice(),
        ),
        (
            642,
            ["tests--fixtures--parity-gaps--pf-expr-near-equality"]
                .as_slice(),
        ),
        (
            643,
            [
                "tests--fixtures--parity-gaps--pf-expr-division-third",
                "tests--fixtures--parity-gaps--pf-expr-tiny-result",
            ]
            .as_slice(),
        ),
        (
            646,
            [
                "tests--fixtures--parity-gaps--anchor-link-label-edge-nbsp",
                "tests--fixtures--parity-gaps--single-link-label-edge-nbsp",
                "tests--fixtures--parity-gaps--triple-link-label-edge-nbsp",
            ]
            .as_slice(),
        ),
        (
            647,
            [
                "tests--fixtures--parity-gaps--iframe-uppercase-ftp",
                "tests--fixtures--parity-gaps--iframe-uppercase-gopher",
                "tests--fixtures--parity-gaps--iframe-uppercase-http-scheme",
                "tests--fixtures--parity-gaps--iframe-uppercase-https",
                "tests--fixtures--parity-gaps--iframe-uppercase-mailto",
            ]
            .as_slice(),
        ),
        (
            353,
            ["tests--fixtures--parity-gaps--math-inline-edge-nbsp"].as_slice(),
        ),
        (
            655,
            ["tests--fixtures--parity-gaps--size-argument-nbsp"].as_slice(),
        ),
        (
            351,
            ["tests--fixtures--parity-gaps--named-anchor-invalid-name-edge-nbsp"]
                .as_slice(),
        ),
        (
            666,
            [
                "tests--fixtures--parity-gaps--pf-expr-boolean-result",
                "tests--fixtures--parity-gaps--pf-expr-bool-equality",
                "tests--fixtures--parity-gaps--pf-expr-logical-and-truthy",
                "tests--fixtures--parity-gaps--pf-expr-logical-or-truthy",
                "tests--fixtures--parity-gaps--pf-expr-lowercase-false",
                "tests--fixtures--parity-gaps--pf-expr-lowercase-true",
            ]
            .as_slice(),
        ),
        (
            668,
            [
                "tests--fixtures--parity-gaps--date-timestamp-edge-nbsp",
                "tests--fixtures--parity-gaps--iframe-url-edge-nbsp",
            ]
            .as_slice(),
        ),
        (
            673,
            ["tests--fixtures--parity-gaps--preproc-crossed-bold-size-with-prior-raw"]
                .as_slice(),
        ),
        (
            660,
            [
                "tests--fixtures--parity-gaps--date-duplicate-timezone",
                "tests--fixtures--parity-gaps--date-timezone-compact-hour-minute",
                "tests--fixtures--parity-gaps--date-timezone-name-est",
                "tests--fixtures--parity-gaps--date-timezone-name-gmt",
                "tests--fixtures--parity-gaps--date-timezone-name-utc",
                "tests--fixtures--parity-gaps--date-timezone-name-z",
                "tests--fixtures--parity-gaps--date-timezone-one-digit-hour",
                "tests--fixtures--parity-gaps--date-timezone-short-hour",
            ]
            .as_slice(),
        ),
        (
            674,
            [
                "tests--fixtures--parity-gaps--preproc-root-collapsible-quoted-inline-close-control",
                "tests--fixtures--parity-gaps--preproc-root-collapsible-quoted-inline-close-trailing-space",
                "tests--fixtures--parity-gaps--preproc-root-uppercase-collapsible-inline-quoted-close",
            ]
            .as_slice(),
        ),
        (
            675,
            ["tests--fixtures--parity-gaps--preproc-quoted-crossed-center-uppercase-collapsible-open"]
                .as_slice(),
        ),
        (
            676,
            ["tests--fixtures--parity-gaps--preproc-crossed-collapsible-div-uppercase-open"]
                .as_slice(),
        ),
        (
            677,
            [
                "tests--fixtures--parity-gaps--block-collapsible-nbsp-closer",
                "tests--fixtures--parity-gaps--block-div-ascii-space-closer-control",
                "tests--fixtures--parity-gaps--block-div-nbsp-closer",
                "tests--fixtures--parity-gaps--block-span-nbsp-closer",
                "tests--fixtures--parity-gaps--preproc-quoted-crossed-center-collapsible-space-closer",
            ]
            .as_slice(),
        ),
        (
            678,
            [
                "tests--fixtures--parity-gaps--attribute-advanced-table-hidden",
                "tests--fixtures--parity-gaps--attribute-anchor-hidden",
                "tests--fixtures--parity-gaps--attribute-boolean-hidden-garbage-control",
                "tests--fixtures--parity-gaps--attribute-boolean-hidden-yes-control",
                "tests--fixtures--parity-gaps--attribute-explicit-list-hidden",
            ]
            .as_slice(),
        ),
        (
            680,
            [
                "tests--fixtures--parity-gaps--pf-expr-caret-parenthesized-negative",
                "tests--fixtures--parity-gaps--pf-expr-caret-power",
                "tests--fixtures--parity-gaps--pf-expr-caret-power-chain",
                "tests--fixtures--parity-gaps--pf-expr-caret-unary-precedence",
            ]
            .as_slice(),
        ),
        (
            681,
            [
                "tests--fixtures--parity-gaps--pf-expr-uppercase-abs-control",
                "tests--fixtures--parity-gaps--pf-expr-uppercase-min-control",
                "tests--fixtures--parity-gaps--pf-expr-uppercase-true-control",
            ]
            .as_slice(),
        ),
        (
            682,
            [
                "tests--fixtures--parity-gaps--image-source-data",
                "tests--fixtures--parity-gaps--image-source-dns",
                "tests--fixtures--parity-gaps--image-source-mailto",
            ]
            .as_slice(),
        ),
        (
            683,
            [
                "tests--fixtures--parity-gaps--date-timestamp-plus-one",
                "tests--fixtures--parity-gaps--date-timestamp-plus-zero",
            ]
            .as_slice(),
        ),
        (
            684,
            [
                "tests--fixtures--parity-gaps--eref-case-title",
                "tests--fixtures--parity-gaps--eref-case-uppercase",
            ]
            .as_slice(),
        ),
        (
            685,
            [
                "tests--fixtures--parity-gaps--block-head-extra-close-bibliography",
                "tests--fixtures--parity-gaps--block-head-extra-close-collapsible",
                "tests--fixtures--parity-gaps--block-head-extra-close-date",
                "tests--fixtures--parity-gaps--block-head-extra-close-footnote",
                "tests--fixtures--parity-gaps--block-head-extra-close-iframe",
                "tests--fixtures--parity-gaps--block-head-extra-close-image",
                "tests--fixtures--parity-gaps--block-head-extra-close-size",
                "tests--fixtures--parity-gaps--block-head-extra-close-toc",
                "tests--fixtures--parity-gaps--eref-extra-close",
            ]
            .as_slice(),
        ),
        (
            686,
            [
                "tests--fixtures--parity-gaps--iframe-blob-scheme",
                "tests--fixtures--parity-gaps--iframe-chrome-scheme",
                "tests--fixtures--parity-gaps--iframe-content-scheme",
                "tests--fixtures--parity-gaps--iframe-dns-scheme",
                "tests--fixtures--parity-gaps--iframe-feed-scheme",
                "tests--fixtures--parity-gaps--iframe-file-scheme",
                "tests--fixtures--parity-gaps--iframe-git-scheme",
                "tests--fixtures--parity-gaps--iframe-sftp-scheme",
            ]
            .as_slice(),
        ),
        (
            687,
            [
                "tests--fixtures--parity-gaps--block-name-case-date-title",
                "tests--fixtures--parity-gaps--block-name-case-date-uppercase",
            ]
            .as_slice(),
        ),
        (
            688,
            ["tests--fixtures--parity-gaps--pf-expr-single-equals-control"].as_slice(),
        ),
        (
            689,
            ["tests--fixtures--parity-gaps--iftags-bare-minus-context-free"].as_slice(),
        ),
        (
            690,
            ["tests--fixtures--parity-gaps--entity-block-head-image-colon"].as_slice(),
        ),
        (
            691,
            [
                "tests--fixtures--parity-gaps--block-head-leading-space-button",
                "tests--fixtures--parity-gaps--block-head-leading-space-file",
                "tests--fixtures--parity-gaps--block-head-leading-space-bibliography",
                "tests--fixtures--parity-gaps--block-head-leading-space-date",
                "tests--fixtures--parity-gaps--block-head-leading-space-eref",
                "tests--fixtures--parity-gaps--block-head-leading-space-footnote",
                "tests--fixtures--parity-gaps--block-head-leading-space-gallery",
                "tests--fixtures--parity-gaps--block-head-leading-space-iframe",
                "tests--fixtures--parity-gaps--block-head-leading-space-image",
                "tests--fixtures--parity-gaps--block-head-leading-space-math",
                "tests--fixtures--parity-gaps--block-head-leading-space-size",
                "tests--fixtures--parity-gaps--block-head-leading-space-toc",
            ]
            .as_slice(),
        ),
        (
            692,
            [
                "tests--fixtures--parity-gaps--pf-expr-null-equality",
                "tests--fixtures--parity-gaps--pf-expr-null-literal",
                "tests--fixtures--parity-gaps--pf-expr-uppercase-null",
            ]
            .as_slice(),
        ),
        (
            693,
            [
                "tests--fixtures--parity-gaps--image-quoted-encoded-traversal",
                "tests--fixtures--parity-gaps--image-quoted-fragment",
                "tests--fixtures--parity-gaps--image-quoted-local-files-path",
                "tests--fixtures--parity-gaps--image-quoted-page-file",
            ]
            .as_slice(),
        ),
        (
            342,
            [
                "tests--fixtures--parity-gaps--file-fields-nbsp",
                "tests--fixtures--parity-gaps--file-name-edge-nbsp",
                "tests--fixtures--parity-gaps--file-quoted-fragment",
                "tests--fixtures--parity-gaps--file-quoted-page",
            ]
            .as_slice(),
        ),
    ];

    for (issue, case_ids) in families {
        for case_id in case_ids {
            let binding = bindings
                .get(*case_id)
                .unwrap_or_else(|| panic!("missing root-cause witness {case_id}"));
            if binding["status"] == "match" {
                continue;
            }
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
        "tests--fixtures--parity-gaps--definition-ascii-edge-space-control",
        "tests--fixtures--parity-gaps--attribute-boolean-hidden-false",
        "tests--fixtures--parity-gaps--attribute-div-class-literal-ampersand-control",
        "tests--fixtures--parity-gaps--attribute-div-class-named-entity",
        "tests--fixtures--parity-gaps--recorder-audit-alias-div-rejected-reparse",
        "tests--fixtures--parity-gaps--recorder-audit-anchor-literal-nested-span",
        "tests--fixtures--parity-gaps--recorder-audit-collapsible-empty-folded",
        "tests--fixtures--parity-gaps--recorder-audit-collapsible-uppercase-show-attribute",
        "tests--fixtures--parity-gaps--recorder-audit-css-module-empty-body",
        "tests--fixtures--parity-gaps--recorder-audit-explicit-list-unsafe-attributes",
        "tests--fixtures--parity-gaps--recorder-audit-footnote-uppercase-owner",
        "tests--fixtures--parity-gaps--recorder-audit-lexical-double-angle",
        "tests--fixtures--parity-gaps--recorder-audit-math-single-quoted-name",
        "tests--fixtures--parity-gaps--recorder-audit-orphan-tab-nested-bold",
        "tests--fixtures--parity-gaps--recorder-audit-raw-entity",
        "tests--fixtures--parity-gaps--recorder-audit-simple-table-malformed-bold-cell",
        "tests--fixtures--parity-gaps--recorder-audit-span-unmatched-opener",
        "tests--fixtures--parity-gaps--pf-literal-code-control",
        "tests--fixtures--parity-gaps--pf-literal-raw-leading-line-space",
        "tests--fixtures--parity-gaps--recorded17-triple-anchor-separated-control",
        "tests--fixtures--parity-gaps--recorded18-code-crossed-only-control",
        "tests--fixtures--parity-gaps--recorded19-center-div-126-control",
        "tests--fixtures--parity-gaps--limit-blockquote-depth-31-control",
        "tests--fixtures--parity-gaps--limit-center-collapsible-prelude-31-control",
        "tests--fixtures--parity-gaps--preproc-crossed-collapsible-div-space-closer",
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
        "tests--fixtures--parity-gaps--autourl-angle-termination",
        "tests--fixtures--parity-gaps--typography-ellipsis-run-five",
        "tests--fixtures--parity-gaps--typography-ellipsis-overlap-mixed",
        "tests--fixtures--parity-gaps--limit-date-format-normal-control",
        "tests--fixtures--parity-gaps--limit-date-format-directives-64-control",
        "tests--fixtures--parity-gaps--limit-cross-collapsible-div-body-510-control",
        "tests--fixtures--parity-gaps--whitespace-raw-cr",
        "tests--fixtures--parity-gaps--whitespace-math-block-nul",
        "tests--fixtures--parity-gaps--whitespace-collapsible-label-control",
        "test--date--hover",
        "test--date--timezone",
        "test--size--basic",
        "tests--fixtures--parity-gaps--entity-decimal-zero",
        "tests--fixtures--parity-gaps--entity-hex-zero",
        "tests--fixtures--parity-gaps--entity-missing-semicolon",
        "tests--fixtures--parity-gaps--entity-surrogate",
        "tests--fixtures--parity-gaps--entity-unicode-overflow",
        "tests--fixtures--parity-gaps--entity-uppercase-named",
        "tests--fixtures--parity-gaps--table-colspan-long-digits",
        "tests--fixtures--parity-gaps--table-colspan-u32-overflow",
        "tests--fixtures--parity-gaps--char-html5-apos",
        "tests--fixtures--parity-gaps--char-uppercase-hex-prefix",
        "tests--fixtures--parity-gaps--limit-lines-100-control",
        "tests--fixtures--parity-gaps--limit-lines-101-boundary",
        "tests--fixtures--parity-gaps--lines-exponent-count",
        "tests--fixtures--parity-gaps--media-align-unknown",
        "tests--fixtures--parity-gaps--media-align-uppercase",
        "tests--fixtures--parity-gaps--bibliography-hide-short-true",
        "tests--fixtures--parity-gaps--bibliography-hide-yes",
        "tests--fixtures--parity-gaps--button-duplicate-text",
        "tests--fixtures--parity-gaps--collapsible-label-edge-nbsp",
        "tests--fixtures--parity-gaps--entity-c1-decimal-128",
        "tests--fixtures--parity-gaps--entity-c1-hex-80",
        "tests--fixtures--parity-gaps--entity-noncharacter-fffe",
        "tests--fixtures--parity-gaps--footnoteblock-hide-short-true",
        "tests--fixtures--parity-gaps--heading-level-seven",
        "tests--fixtures--parity-gaps--horizontal-rule-trailing-nbsp",
        "tests--fixtures--parity-gaps--iframe-ftp-scheme",
        "tests--fixtures--parity-gaps--iframe-protocol-relative",
        "tests--fixtures--parity-gaps--lines-count-nbsp",
        "tests--fixtures--parity-gaps--lines-count-over-100",
        "tests--fixtures--parity-gaps--math-block-edge-nbsp",
        "tests--fixtures--parity-gaps--pf-expr-ceil-function",
        "tests--fixtures--parity-gaps--pf-expr-bool-add",
        "tests--fixtures--parity-gaps--pf-expr-double-unary-minus",
        "tests--fixtures--parity-gaps--pf-expr-leading-decimal",
        "tests--fixtures--parity-gaps--pf-expr-min-three",
        "tests--fixtures--parity-gaps--pf-expr-remainder-negative-left",
        "tests--fixtures--parity-gaps--pf-expr-negative-zero",
        "tests--fixtures--parity-gaps--pf-expr-short-circuit-and-undefined",
        "tests--fixtures--parity-gaps--pf-expr-short-circuit-or",
        "tests--fixtures--parity-gaps--pf-expr-short-circuit-or-undefined",
        "tests--fixtures--parity-gaps--pf-expr-trailing-decimal",
        "tests--fixtures--parity-gaps--pf-expr-unary-not",
        "tests--fixtures--parity-gaps--ruby2-fields-nbsp",
        "tests--fixtures--parity-gaps--size-decimal-em-control",
        "tests--fixtures--parity-gaps--size-pt-unit",
        "tests--fixtures--parity-gaps--size-uppercase-px",
        "tests--fixtures--parity-gaps--image-source-ftp",
        "tests--fixtures--parity-gaps--image-source-protocol-relative",
        "tests--fixtures--parity-gaps--image-source-uppercase-https",
        "tests--fixtures--parity-gaps--date-timestamp-decimal",
        "tests--fixtures--parity-gaps--date-timestamp-leading-zero-control",
        "tests--fixtures--parity-gaps--date-timestamp-negative-zero-control",
        "tests--fixtures--parity-gaps--eref-lower-control",
        "tests--fixtures--parity-gaps--eref-nbsp-name",
        "tests--fixtures--parity-gaps--iframe-compact-class-control",
        "tests--fixtures--parity-gaps--iframe-title-attribute",
        "tests--fixtures--parity-gaps--single-link-target-trailing-nbsp",
        "tests--fixtures--parity-gaps--span-duplicate-class",
        "tests--fixtures--parity-gaps--table-colspan-negative-one",
        "tests--fixtures--parity-gaps--table-colspan-plus-zero",
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
