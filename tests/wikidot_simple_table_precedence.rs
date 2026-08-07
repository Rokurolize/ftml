use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::parsing::{ParseError, ParseErrorKind};
use ftml::render::{PageExistenceResolver, Render, html::HtmlRender, text::TextRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use ftml::tree::SyntaxTree;
use regex::Regex;
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

const LIVE_FIXTURE: &str =
    include_str!("fixtures/wikidot-simple-table-precedence-live-20260730.json");
const LIVE_FOLLOWUP: &str =
    include_str!("fixtures/wikidot-simple-table-precedence-followup-20260807.jsonl");

struct MissingPages;

impl PageExistenceResolver for MissingPages {
    fn page_exists(&self, _site: &str, _page: &str) -> bool {
        false
    }
}

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("simple-table-precedence"),
        category: None,
        site: Cow::Borrowed("syntax-differential"),
        title: Cow::Borrowed("Simple table precedence"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    }
}

fn parse(source: &str) -> (SyntaxTree<'static>, Vec<ParseError>) {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokens = ftml::tokenize(&source);
    let (tree, errors) = ftml::parse(&tokens, &page_info, &settings).into();
    (tree.to_owned(), errors)
}

fn render(source: &str) -> (String, String, Vec<ParseError>) {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let (tree, errors) = parse(source);
    let html = HtmlRender.render(&tree, &page_info, &settings).body;
    let text = TextRender.render(&tree, &page_info, &settings);
    (html, text, errors)
}

fn render_with_missing_pages(source: &str) -> (String, Vec<ParseError>) {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let (tree, errors) = parse(source);
    let html = HtmlRender
        .render_with_page_existence(&tree, &page_info, &settings, &MissingPages)
        .body;
    (html, errors)
}

fn live_html_as_ftml_body(raw_html: &str) -> String {
    static INTERTAG_WHITESPACE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r">\s+<").unwrap());

    let body = raw_html.strip_prefix("\n\n").unwrap_or(raw_html);
    let body = body.strip_suffix('\n').unwrap_or(body);
    let body = body
        .replace("<br />", "<br>")
        .replace("&#171;", "«")
        .replace("&#187;", "»")
        .replace("&nbsp;", "\u{a0}")
        .replace("</table>\r\n", "</table>")
        .replace("</table>\n", "</table>");
    INTERTAG_WHITESPACE.replace_all(&body, "><").into_owned()
}

fn table(cells: &[&str]) -> String {
    let cells = cells
        .iter()
        .map(|cell| format!("<td>{cell}</td>\n"))
        .collect::<String>();
    format!("<table class=\"wiki-content-table\">\n<tr>\n{cells}</tr>\n</table>")
}

#[test]
fn ordinary_inline_owners_stop_at_simple_table_cells() {
    for (name, source, first) in [
        ("bold", "|| **A||B** || C ||", "<strong>A</strong>"),
        ("italic", "|| //A||B// || C ||", "<em>A</em>"),
        (
            "strike",
            "|| --A||B-- || C ||",
            "<span style=\"text-decoration: line-through;\">A</span>",
        ),
        (
            "underline",
            "|| __A||B__ || C ||",
            "<span style=\"text-decoration: underline;\">A</span>",
        ),
        ("superscript", "|| ^^A||B^^ || C ||", "<sup>A</sup>"),
        ("subscript", "|| ,,A||B,, || C ||", "<sub>A</sub>"),
        ("monospace", "|| {{A||B}} || C ||", "<tt>A</tt>"),
        (
            "code-shaped monospace",
            "|| {{{A||B}}} || C ||",
            "<tt>{A</tt>",
        ),
        (
            "color",
            "|| ##red|A||B## || C ||",
            "<span style=\"color: red\">A</span>",
        ),
    ] {
        let (html, text, errors) = render(source);
        assert!(errors.is_empty(), "{name}: {errors:#?}");
        let second = if name == "code-shaped monospace" {
            "B}"
        } else {
            "B"
        };
        assert_eq!(html, table(&[first, second, "C"]), "{name}");
        let first_text = if name == "code-shaped monospace" {
            "{A"
        } else {
            "A"
        };
        assert_eq!(text, format!("{first_text}{second}C"), "{name}");
    }
}

#[test]
fn raw_comment_and_triple_link_labels_keep_table_delimiters() {
    for (source, first) in [
        (
            "|| @@A||B@@ || C ||",
            "<span style=\"white-space: pre-wrap;\">A||B</span>",
        ),
        ("|| A[!--X||Y--]B || C ||", "AB"),
        ("|| [[[page|A||B]]] || C ||", "<a href=\"/page\">A||B</a>"),
    ] {
        let (html, _, errors) = render(source);
        assert!(errors.is_empty(), "{source:?}: {errors:#?}");
        assert_eq!(html, table(&[first, "C"]), "{source:?}");
    }
}

#[test]
fn inline_literal_div_does_not_steal_protected_owner_delimiters() {
    for source in [
        "|| [[div]]@@A||B@@[[/div]] || C ||",
        "|| [[div]]A[!--X||Y--]B[[/div]] || C ||",
        "|| [[div]][[[page|A||B]]][[/div]] || C ||",
    ] {
        let (html, _, errors) = render(source);
        assert!(errors.is_empty(), "{source:?}: {errors:#?}");
        assert_eq!(html.matches("<td>").count(), 2, "{source:?}: {html}");
        assert!(html.contains("<td>C</td>"), "{source:?}: {html}");
    }
}

#[test]
fn pure_even_pipe_runs_and_repeated_prefixes_match_wikidot() {
    for pipes in (4..=12).step_by(2) {
        let source = "|".repeat(pipes);
        let (html, text, errors) = render(&source);
        assert!(errors.is_empty(), "{source:?}: {errors:#?}");
        assert_eq!(html, table(&[""]), "{source:?}");
        assert_eq!(text, "", "{source:?}");
        assert!(!html.contains("colspan"), "{source:?}: {html}");
    }

    for marker in ['~', '='] {
        for count in 2..=5 {
            let prefix = marker.to_string().repeat(count);
            let source = format!("||{prefix} A ||");
            let (html, text, errors) = render(&source);
            assert!(errors.is_empty(), "{source:?}: {errors:#?}");
            assert_eq!(html, table(&[&format!("{prefix} A")]), "{source:?}");
            assert_eq!(text, format!("{prefix} A"), "{source:?}");
        }
    }
}

#[test]
fn malformed_inline_controls_keep_exact_residual_source() {
    for source in [
        "|| **A||B || C ||",
        "|| //A||B || C ||",
        "|| {{A||B || C ||",
    ] {
        let (html, text, errors) = render(source);
        assert!(
            html.contains("<table class=\"wiki-content-table\">"),
            "{html}"
        );
        assert!(
            text.contains('A') && text.contains('B') && text.contains('C'),
            "{text:?}"
        );
        assert!(
            errors.iter().all(|error| matches!(
                error.kind(),
                ParseErrorKind::EndOfInput
                    | ParseErrorKind::RuleFailed
                    | ParseErrorKind::NoRulesMatch
            )),
            "{source:?}: {errors:#?}"
        );
    }
}

#[test]
fn explicit_links_parser_functions_and_right_marker_runs_keep_exact_ownership() {
    for (source, expected_html, expected_text) in [
        (
            "|| [https://example.com A||B] || C ||",
            table(&[
                "[<a href=\"https://example.com\">https://example.com</a> A",
                "B]",
                "C",
            ]),
            "[https://example.com AB]C".to_owned(),
        ),
        (
            "|| [[#if 1|A||B|C]] || C ||",
            table(&["", "C"]),
            "C".to_owned(),
        ),
        ("||>>>>> A ||", table(&["»»&gt; A"]), "»»> A".to_owned()),
    ] {
        let (html, text, errors) = render(source);
        assert!(errors.is_empty(), "{source:?}: {errors:#?}");
        assert_eq!(html, expected_html, "{source:?}");
        assert_eq!(text, expected_text, "{source:?}");
    }
}

#[test]
fn inline_span_is_clipped_at_the_cell_boundary() {
    let source = "|| [[span]]A||B[[/span]] || C ||";
    let (html, text, errors) = render(source);
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(html, table(&["<span>A</span>", "B", "C"]));
    assert_eq!(text, "ABC");
}

#[test]
fn frozen_live_matrices_match_the_public_wikidot_render_seam() {
    let fixture: serde_json::Value =
        serde_json::from_str(LIVE_FIXTURE).expect("valid frozen simple-table fixture");
    assert_eq!(
        fixture["schema"],
        "ftml.wikidot_simple_table_precedence_evidence.v1"
    );
    assert_eq!(fixture["broad"].as_array().unwrap().len(), 75);
    assert_eq!(fixture["focused"].as_array().unwrap().len(), 82);
    assert_eq!(
        fixture["supplemental_comments"].as_array().unwrap().len(),
        2
    );

    let provenance = fixture["provenance"].as_object().unwrap();
    for field in [
        "ftml_commit",
        "ftml_tree",
        "renderer_sha256",
        "broad_cases_sha256",
        "broad_live_sha256",
        "focused_cases_sha256",
        "focused_live_sha256",
        "supplemental_cases_sha256",
    ] {
        assert_eq!(
            provenance[field].as_str().unwrap().len(),
            40.max(if field.contains("sha256") { 64 } else { 40 })
        );
    }

    let runtime_owned = BTreeSet::from(["scout-table-in-html"]);
    let mut mismatches = Vec::new();
    for family in ["broad", "focused", "supplemental_comments"] {
        for case in fixture[family].as_array().unwrap() {
            let id = case["id"].as_str().unwrap();
            let source = case["source"].as_str().unwrap();
            assert_eq!(case["source_sha256"].as_str().unwrap().len(), 64, "{id}");
            assert_eq!(case["raw_html_sha256"].as_str().unwrap().len(), 64, "{id}");
            if runtime_owned.contains(id) {
                continue;
            }
            let expected = live_html_as_ftml_body(case["raw_html"].as_str().unwrap());
            let (actual, _) = render_with_missing_pages(source);
            let actual = live_html_as_ftml_body(&actual);
            if actual != expected {
                mismatches.push(format!(
                    "{family}/{id}: {source:?}\nexpected {expected:?}\nactual   {actual:?}"
                ));
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "live matrix mismatches:\n{}",
        mismatches.join("\n\n"),
    );
}

#[test]
fn live_followup_freezes_malformed_rows_and_advanced_table_controls() {
    for line in LIVE_FOLLOWUP.lines().filter(|line| !line.is_empty()) {
        let record: serde_json::Value =
            serde_json::from_str(line).expect("valid follow-up row");
        let id = record["syntax_case"]["case_id"].as_str().unwrap();
        let source = record["syntax_case"]["source"].as_str().unwrap();
        let expected = live_html_as_ftml_body(record["raw_html"].as_str().unwrap());
        let (actual, errors) = render_with_missing_pages(source);
        let actual = live_html_as_ftml_body(&actual);

        assert_eq!(record["source_sha256"].as_str().unwrap().len(), 64, "{id}");
        assert_eq!(
            record["raw_html_sha256"].as_str().unwrap().len(),
            64,
            "{id}"
        );
        assert_eq!(actual, expected, "{id}: {source:?}");

        let kinds = errors.iter().map(ParseError::kind).collect::<Vec<_>>();
        match id {
            "issue-295-unclosed-bold-row" | "issue-295-unclosed-bold-one-row" => {
                assert_eq!(
                    kinds,
                    vec![ParseErrorKind::EndOfInput, ParseErrorKind::NoRulesMatch],
                    "{id}",
                );
            }
            "issue-295-advanced-nested-simple-control" => {
                assert_eq!(
                    kinds,
                    vec![
                        ParseErrorKind::NotStartOfLine,
                        ParseErrorKind::NoRulesMatch,
                        ParseErrorKind::NotStartOfLine,
                        ParseErrorKind::NoRulesMatch,
                        ParseErrorKind::NotStartOfLine,
                        ParseErrorKind::NoRulesMatch,
                    ],
                    "{id}",
                );
            }
            "issue-295-inline-advanced-in-simple-control" => {
                assert!(errors.is_empty(), "{id}: {errors:#?}");
            }
            _ => panic!("unknown follow-up case {id}"),
        }
    }
}

#[test]
fn rows_cells_and_closers_recover_without_crossing_physical_rows() {
    for (source, expected) in [
        (
            "|| **A||B** || C ||\n|| D || E ||",
            concat!(
                "<table class=\"wiki-content-table\">\n",
                "<tr>\n<td><strong>A</strong></td>\n<td>B</td>\n<td>C</td>\n</tr>\n",
                "<tr>\n<td>D</td>\n<td>E</td>\n</tr>\n",
                "</table>"
            ),
        ),
        (
            "|| || A || ||\n|| B || C ||",
            concat!(
                "<table class=\"wiki-content-table\">\n",
                "<tr>\n<td></td>\n<td>A</td>\n<td></td>\n</tr>\n",
                "<tr>\n<td>B</td>\n<td>C</td>\n</tr>\n",
                "</table>"
            ),
        ),
        (
            "|| **A||B\n|| C || D ||",
            concat!(
                "<table class=\"wiki-content-table\">\n",
                "<tr>\n<td>**A</td>\n</tr>\n",
                "<tr>\n<td>C</td>\n<td>D</td>\n</tr>\n",
                "</table>"
            ),
        ),
    ] {
        let (html, _, errors) = render(source);
        assert!(
            errors.iter().all(|error| matches!(
                error.kind(),
                ParseErrorKind::EndOfInput
                    | ParseErrorKind::RuleFailed
                    | ParseErrorKind::NoRulesMatch
            )),
            "{source:?}: {errors:#?}"
        );
        assert_eq!(html, expected, "{source:?}");
    }
}

#[test]
fn crlf_rows_keep_one_simple_table() {
    let source = "|| **A||B** || C ||\r\n|| D || E ||\r\n";
    let (html, _, errors) = render(source);
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(
        html.matches("<table class=\"wiki-content-table\">").count(),
        1
    );
    assert_eq!(html.matches("<tr>").count(), 2);
}

#[test]
fn dense_delimiter_and_inline_rows_remain_bounded_and_non_recursive() {
    const CELLS: usize = 4_096;
    let source = format!("||{}", "**A||B** ||".repeat(CELLS));
    let started = Instant::now();
    let (html, _, errors) = render(&source);
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(html.matches("<strong>A</strong>").count(), CELLS);
    assert_eq!(html.matches("<td>B</td>").count(), CELLS);
}
