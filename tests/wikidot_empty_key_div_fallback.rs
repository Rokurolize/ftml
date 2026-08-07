use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::parsing::{ParseError, ParseErrorKind};
use ftml::render::{Render, html::HtmlRender, text::TextRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use ftml::tree::{ContainerType, Element, SyntaxTree};
use serde_json::Value;
use std::borrow::Cow;
use std::time::{Duration, Instant};

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("issue-316-empty-key-div-fallback"),
        category: None,
        site: Cow::Borrowed("sandbox-for-codex"),
        title: Cow::Borrowed("Issue 316 empty-key div fallback"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    }
}

fn parse_render(
    source: &str,
    layout: Layout,
) -> (SyntaxTree<'static>, String, String, Vec<ParseError>) {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, layout);
    let tokenization = ftml::tokenize(&source);
    let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    let html = HtmlRender.render(&tree, &page_info, &settings).body;
    let text = TextRender.render(&tree, &page_info, &settings);
    (tree.to_owned(), html, text, errors)
}

fn render_wikidot(source: &str) -> (String, String, Vec<ParseError>) {
    let (_, html, text, errors) = parse_render(source, Layout::Wikidot);
    (html, text, errors)
}

fn malformed_errors(errors: &[ParseError]) -> Vec<(&str, std::ops::Range<usize>)> {
    errors
        .iter()
        .filter(|error| error.kind() == ParseErrorKind::BlockMalformedArguments)
        .map(|error| (error.rule(), error.span()))
        .collect()
}

fn live_fixture() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/wikidot-empty-key-div-live-20260731.json"
    ))
    .expect("valid issue 316 live fixture")
}

#[test]
fn live_backed_empty_key_div_is_one_literal_paragraph_with_a_diagnostic() {
    let fixture = live_fixture();
    assert_eq!(
        fixture["schema"],
        "ftml.wikidot_empty_key_div_live_observation.v1"
    );
    assert_eq!(fixture["transport"], "anonymous edit/PagePreviewModule");
    assert_eq!(fixture["mutated_server_state"], false);
    assert_eq!(
        fixture["reference_tree"],
        "59f18f01a270782e6ac6b36c569593ecbc051758"
    );

    let row = &fixture["rows"][0];
    assert_eq!(
        row["source_sha256"],
        "70c72841e3511ad3ea1bcec8601a4e7f1abc1c9880652a14d3eeb2e315320aaa",
    );
    let source = row["source"].as_str().expect("fixture source");
    let (tree, html, text, errors) = parse_render(source, Layout::Wikidot);

    assert_eq!(html, row["html"].as_str().expect("fixture HTML"));
    assert_eq!(text, row["text"].as_str().expect("fixture text"));
    assert_eq!(malformed_errors(&errors).len(), 1, "{errors:#?}");
    assert_eq!(malformed_errors(&errors)[0].0, "block-div");
    assert_eq!(malformed_errors(&errors)[0].1, 16..20);
    assert!(matches!(
        tree.elements.as_slice(),
        [Element::Container(container)] if container.ctype() == ContainerType::Paragraph
    ));
}

#[test]
fn empty_key_div_and_span_names_roll_back_separately_from_valid_attributes() {
    for (name, close, rule) in [
        ("div", "div", "block-div"),
        ("div_", "div", "block-div"),
        ("span", "span", "block-span"),
        ("span_", "span", "block-span"),
    ] {
        let source = format!(r#"[[{name} ="value"]]body[[/{close}]]"#);
        let (html, text, errors) = render_wikidot(&source);
        assert_eq!(
            html,
            format!("<p>{}</p>", source.replace('"', "&quot;")),
            "{name}",
        );
        assert_eq!(text, source, "{name}");
        assert_eq!(malformed_errors(&errors).len(), 1, "{name}: {errors:#?}");
        assert_eq!(malformed_errors(&errors)[0].0, rule, "{name}");
    }

    for argument in ["=\"value\"", "= \"value\"", "='value'", "=bare"] {
        let source = format!("[[span {argument}]]body[[/span]]");
        let (html, text, errors) = render_wikidot(&source);
        assert_eq!(
            html,
            format!(
                "<p>{}</p>",
                source.replace('"', "&quot;").replace('\'', "&#39;")
            ),
            "{argument}",
        );
        assert_eq!(text, source, "{argument}");
        assert_eq!(malformed_errors(&errors).len(), 1, "{errors:#?}");
    }

    for (source, expected) in [
        (
            "[[div class=\"value\"]]\nbody\n[[/div]]",
            "<div class=\"value\"><p>body</p></div>",
        ),
        (
            "[[div_ class=\"value\"]]\nbody\n[[/div]]",
            "<div class=\"value\">body</div>",
        ),
        (
            "[[span class=\"value\"]]body[[/span]]",
            "<p><span class=\"value\">body</span></p>",
        ),
        (
            "[[span_ class=\"value\"]]body[[/span]]",
            "<p><span class=\"value\">body</span></p>",
        ),
    ] {
        let (html, _, errors) = render_wikidot(source);
        assert_eq!(html, expected, "{source:?}");
        assert!(
            malformed_errors(&errors).is_empty(),
            "{source:?}: {errors:#?}"
        );
    }
}

#[test]
fn malformed_candidates_keep_exact_boundaries_and_do_not_activate_children() {
    let cases = [
        (
            "inline",
            "[[div =\"value\"]]body[[/div]]",
            "<p>[[div =&quot;value&quot;]]body[[/div]]</p>",
        ),
        (
            "own-line",
            "[[div =\"value\"]]\nbody\n[[/div]]",
            "<p>[[div =&quot;value&quot;]]<br>\nbody<br>\n[[/div]]</p>",
        ),
        (
            "prose-adjacent",
            "before [[span =\"value\"]]body[[/span]] after",
            "<p>before [[span =&quot;value&quot;]]body[[/span]] after</p>",
        ),
        (
            "nested-container",
            "[[div]]\n[[span =\"value\"]]body[[/span]]\n[[/div]]",
            "<div><p>[[span =&quot;value&quot;]]body[[/span]]</p></div>",
        ),
        (
            "nested-syntax",
            "[[span =\"value\"]]**body**[[/span]]",
            "<p>[[span =&quot;value&quot;]]**body**[[/span]]</p>",
        ),
        (
            "rejected-alias-heading-list-table",
            "[[div_ =\"value\"]]\n+ H\n* A\n|| B ||\n[[/div]]",
            "<p>[[div_ =&quot;value&quot;]]<br>\n+ H<br>\n* A<br>\n|| B ||<br>\n[[/div]]</p>",
        ),
    ];

    for (case_id, source, expected) in cases {
        let (html, text, errors) = render_wikidot(source);
        assert_eq!(html, expected, "{case_id}");
        let expected_text = if case_id == "nested-container" {
            "[[span =\"value\"]]body[[/span]]"
        } else {
            source
        };
        assert_eq!(text, expected_text.replace("\r\n", "\n"), "{case_id}");
        assert_eq!(malformed_errors(&errors).len(), 1, "{case_id}: {errors:#?}");
        assert!(!html.contains("<strong>"), "{case_id}: {html}");
        assert!(!html.contains("<h1"), "{case_id}: {html}");
        assert!(!html.contains("<table"), "{case_id}: {html}");
        assert!(!html.contains("<ul"), "{case_id}: {html}");
    }
}

#[test]
fn malformed_candidates_recover_inside_quote_list_and_table_owners() {
    for (case_id, source, expected, rule) in [
        (
            "quote",
            "> [[div =\"value\"]]body[[/div]]",
            "<blockquote><p>[[div =&quot;value&quot;]]body[[/div]]</p></blockquote>",
            "block-div",
        ),
        (
            "list",
            "* [[span =\"value\"]]body[[/span]]",
            "<ul>\n<li>[[span =&quot;value&quot;]]body[[/span]]</li>\n</ul>",
            "block-span",
        ),
        (
            "table",
            "|| [[span =\"value\"]]body[[/span]] ||",
            "<table class=\"wiki-content-table\">\n<tr>\n<td>[[span =&quot;value&quot;]]body[[/span]]</td>\n</tr>\n</table>",
            "block-span",
        ),
    ] {
        let (html, _, errors) = render_wikidot(source);
        assert_eq!(html, expected, "{case_id}");
        let malformed = malformed_errors(&errors);
        assert_eq!(malformed.len(), 1, "{case_id}: {errors:#?}");
        assert_eq!(malformed[0].0, rule, "{case_id}");
    }
}

#[test]
fn missing_and_extra_closers_recover_for_later_syntax() {
    let missing = "[[span =\"value\"]]body\n\n**later**";
    let (html, _, errors) = render_wikidot(missing);
    assert_eq!(
        html,
        "<p>[[span =&quot;value&quot;]]body</p><p><strong>later</strong></p>",
    );
    assert_eq!(malformed_errors(&errors).len(), 1, "{errors:#?}");

    let extra = "[[span =\"value\"]]body[[/span]][[/span]]\n\n**later**";
    let (html, _, errors) = render_wikidot(extra);
    assert_eq!(
        html,
        "<p>[[span =&quot;value&quot;]]body[[/span]][[/span]]</p><p><strong>later</strong></p>",
    );
    assert_eq!(malformed_errors(&errors).len(), 1, "{errors:#?}");
}

#[test]
fn literal_owners_keep_empty_key_candidates_inert() {
    for source in [
        "[[code]]\n[[span =\"value\"]]body[[/span]]\n[[/code]]",
        "@@[[span =\"value\"]]body[[/span]]@@",
        "[!-- [[span =\"value\"]]body[[/span]] --]sentinel",
        "[[html]]\n[[span =\"value\"]]body[[/span]]\n[[/html]]",
    ] {
        let (_, _, errors) = render_wikidot(source);
        assert!(
            malformed_errors(&errors).is_empty(),
            "{source:?}: {errors:#?}"
        );
    }
}

#[test]
fn malformed_empty_keys_create_no_structure_attributes_or_unsafe_html() {
    let source = concat!(
        "[[div =\"value\" onclick=\"alert(1)\"]]<script>alert(2)</script>",
        "[[span class=\"trusted\"]]nested[[/span]][[/div]]",
    );
    let (tree, html, text, errors) = parse_render(source, Layout::Wikidot);
    assert_eq!(malformed_errors(&errors).len(), 1, "{errors:#?}");
    assert_eq!(text, source);
    assert!(
        html.starts_with("<p>[[div =&quot;value&quot; onclick="),
        "{html}"
    );
    assert!(
        html.contains("&lt;script&gt;alert(2)&lt;/script&gt;"),
        "{html}"
    );
    assert!(!html.contains("<script>"), "{html}");
    assert!(!html.contains("<div "), "{html}");
    assert!(!html.contains("<span "), "{html}");
    assert!(!html.contains("class=\"trusted\""), "{html}");
    assert!(!tree.elements.iter().any(|element| {
        matches!(element, Element::Container(container) if container.ctype() == ContainerType::Div)
    }));

    let source = "[[span class=\"safe\" =\"bad\"]]body[[/span]]";
    let (_, html, text, errors) = parse_render(source, Layout::Wikidot);
    assert_eq!(text, source);
    assert_eq!(malformed_errors(&errors).len(), 1, "{errors:#?}");
    assert_eq!(
        html,
        "<p>[[span class=&quot;safe&quot; =&quot;bad&quot;]]body[[/span]]</p>",
    );
    assert!(!html.contains("<span "), "{html}");

    let source = "[[span class=\"safe\" =bare]]body[[/span]]";
    let (_, html, text, errors) = parse_render(source, Layout::Wikidot);
    assert_eq!(text, source);
    assert_eq!(malformed_errors(&errors).len(), 1, "{errors:#?}");
    assert_eq!(
        html,
        "<p>[[span class=&quot;safe&quot; =bare]]body[[/span]]</p>",
    );
    assert!(!html.contains("<span "), "{html}");
}

#[test]
fn unicode_and_crlf_empty_key_source_keeps_normalized_line_boundaries() {
    let source = "[[div =\"値😀\"]]\r\n雪\r\n[[/div]]";
    let (html, text, errors) = render_wikidot(source);
    assert_eq!(
        html,
        "<p>[[div =&quot;値😀&quot;]]<br>\n雪<br>\n[[/div]]</p>",
    );
    assert_eq!(text, "[[div =\"値😀\"]]\n雪\n[[/div]]");
    assert_eq!(malformed_errors(&errors).len(), 1, "{errors:#?}");
}

#[test]
fn v7_active_div_paragraph_rows_match_live_dom() {
    let fixture = live_fixture();
    for row in fixture["rows"]
        .as_array()
        .expect("fixture rows")
        .iter()
        .skip(1)
    {
        let case_id = row["case_id"].as_str().expect("case id");
        let source = row["source"].as_str().expect("source");
        let expected = row["html"].as_str().expect("HTML");
        let (html, _, _) = render_wikidot(source);
        assert_eq!(html, expected, "{case_id}");
    }
}

#[test]
fn repeated_empty_key_candidates_stay_bounded() {
    const CANDIDATE_COUNT: usize = 2_048;
    let unit = "[[span =\"value\"]]**literal**[[/span]] ";
    let source = unit.repeat(CANDIDATE_COUNT);
    let started = Instant::now();
    let (html, text, errors) = render_wikidot(&source);
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "empty-key recovery took {elapsed:?}",
    );
    assert_eq!(malformed_errors(&errors).len(), CANDIDATE_COUNT);
    assert_eq!(
        html.matches("[[span =&quot;value&quot;]]").count(),
        CANDIDATE_COUNT
    );
    assert_eq!(text.matches("[[span =\"value\"]]").count(), CANDIDATE_COUNT);
    assert!(!html.contains("<span"), "{html}");
    assert!(!html.contains("<strong>"), "{html}");

    let unclosed_unit = "[[span =\"value\"]] ";
    let unclosed_source = unclosed_unit.repeat(CANDIDATE_COUNT);
    let started = Instant::now();
    let (html, text, errors) = render_wikidot(&unclosed_source);
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "unclosed empty-key recovery took {elapsed:?}",
    );
    assert_eq!(malformed_errors(&errors).len(), CANDIDATE_COUNT);
    assert_eq!(text, unclosed_source.trim_end());
    assert!(!html.contains("<span"), "{html}");
}

#[test]
fn wikijump_layout_keeps_strict_empty_key_rejection() {
    let source = "[[span =\"value\"]]body[[/span]]";
    let (_, html, text, errors) = parse_render(source, Layout::Wikijump);
    assert_eq!(html, "<p>[[span =&quot;value&quot;]]body[[/span]]</p>");
    assert_eq!(text, source);
    assert!(
        errors
            .iter()
            .any(|error| error.kind() == ParseErrorKind::BlockMalformedArguments),
        "{errors:#?}",
    );
}
