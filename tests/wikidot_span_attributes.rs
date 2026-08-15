use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;

fn render(source: &str) -> String {
    render_with_layout(source, Layout::Wikidot)
}

fn render_with_layout(source: &str, layout: Layout) -> String {
    let page_info = PageInfo {
        page: Cow::Borrowed("span-attributes"),
        category: None,
        site: Cow::Borrowed("syntax-differential"),
        title: Cow::Borrowed("Span attributes"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    };
    let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokenization = ftml::tokenize(&source);
    let (tree, _errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    HtmlRender.render(&tree, &page_info, &settings).body
}

#[test]
fn wikidot_scored_span_residual_opener_owns_the_root_paragraph() {
    let source = "[[span_]]]X[[/span]]";

    assert_eq!(render(source), "<span>]X</span>");
    assert_eq!(render("[[span]]X[[/span]]"), "<p><span>X</span></p>",);
    assert_eq!(
        render("[[span_]]X[[/span_]]"),
        "<p>[[span_]]X[[/span_]]</p>",
    );
    assert_eq!(
        render_with_layout(source, Layout::Wikijump),
        "<p>[[span_]]]X[[/span]]</p>",
    );
}

#[test]
fn wikidot_span_attribute_matrix_matches_live_dom() {
    for (source, expected) in [
        (
            "BEGIN|[[span]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span class=\"c\"]]X[[/span]]|END",
            "<p>BEGIN|<span class=\"c\">X</span>|END</p>",
        ),
        (
            "BEGIN|[[span id=\"i\"]]X[[/span]]|END",
            "<p>BEGIN|<span id=\"u-i\">X</span>|END</p>",
        ),
        (
            "BEGIN|[[span style=\"color:red\"]]X[[/span]]|END",
            "<p>BEGIN|<span style=\"color:red\">X</span>|END</p>",
        ),
        (
            "BEGIN|[[span title=\"tip\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span lang=\"en\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span dir=\"rtl\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span role=\"note\"]]X[[/span]]|END",
            "<p>BEGIN|<span role=\"note\">X</span>|END</p>",
        ),
        (
            "BEGIN|[[span tabindex=\"0\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span aria-label=\"label\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span data-x=\"value\"]]X[[/span]]|END",
            "<p>BEGIN|<span data-x=\"value\">X</span>|END</p>",
        ),
        (
            "BEGIN|[[span onclick=\"alert(1)\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span onmouseover=\"alert(1)\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span href=\"https://example.com\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span src=\"https://example.com/x\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span frobnicate=\"yes\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span name=\"n\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span rel=\"nofollow\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span class=\"c\" title=\"tip\"]]X[[/span]]|END",
            "<p>BEGIN|<span class=\"c\">X</span>|END</p>",
        ),
        (
            "BEGIN|[[span style=\"color:red\" title=\"tip\"]]X[[/span]]|END",
            "<p>BEGIN|<span style=\"color:red\">X</span>|END</p>",
        ),
        (
            "BEGIN|[[span id=\"i\" title=\"tip\"]]X[[/span]]|END",
            "<p>BEGIN|<span id=\"u-i\">X</span>|END</p>",
        ),
        (
            "BEGIN|[[span title=\"tip\" class=\"c\"]]X[[/span]]|END",
            "<p>BEGIN|<span class=\"c\">X</span>|END</p>",
        ),
        (
            "BEGIN|[[span title=\"first\" title=\"second\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span TITLE=\"tip\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span title='tip']]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span title=tip]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span title=\"a &amp; b\"]]X[[/span]]|END",
            "<p>BEGIN|<span>X</span>|END</p>",
        ),
        (
            "BEGIN|[[span title=\"[[x]]\"]]X[[/span]]|END",
            "<p>BEGIN|<span>&quot;]]X</span>|END</p>",
        ),
    ] {
        assert_eq!(render(source), expected, "{source}");
    }
}
