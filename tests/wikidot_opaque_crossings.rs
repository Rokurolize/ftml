use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;

fn render(source: &str) -> String {
    let page_info = PageInfo {
        page: Cow::Borrowed("opaque-crossings"),
        category: Some(Cow::Borrowed("_default")),
        site: Cow::Borrowed("sandbox-for-codex"),
        title: Cow::Borrowed("Opaque crossings"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    };
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokenization = ftml::tokenize(&source);
    let (tree, _) = ftml::parse(&tokenization, &page_info, &settings).into();
    HtmlRender.render(&tree, &page_info, &settings).body
}

#[test]
fn inline_formats_literalize_across_module_blocks() {
    for (source, expected) in [
        (
            "**A\n[[module PageTree]]\nB**",
            "<p>**A</p><p>TODO: module PageTree</p><p>B**</p>",
        ),
        (
            "//A\n[[module PageTree]]\nB//",
            "<p>//A</p><p>TODO: module PageTree</p><p>B//</p>",
        ),
        (
            "{{A\n[[module PageTree]]\nB}}",
            "<p>{{A</p><p>TODO: module PageTree</p><p>B}}</p>",
        ),
    ] {
        assert_eq!(render(source), expected, "{source:?}");
    }
}

#[test]
fn span_size_and_anchor_around_module_block_boundaries() {
    assert_eq!(
        render("[[span class=\"p\"]]A\n[[module PageTree]]\nB[[/span]]"),
        "<p><span class=\"p\">A</span></p><p>TODO: module PageTree</p><p><span class=\"p\">B</span></p>",
    );
    assert_eq!(
        render("[[size 120%]]A\n[[module PageTree]]\nB[[/size]]"),
        "<p><span style=\"font-size:120%;\">A</span></p><p>TODO: module PageTree</p><p><span style=\"font-size:120%;\">B</span></p>",
    );
    assert_eq!(
        render("[[a href=\"/target\"]]A\n[[module PageTree]]\nB[[/a]]"),
        "<p><a href=\"/target\">A</a></p><p>TODO: module PageTree</p><p>B</p>",
    );
}

#[test]
fn span_and_size_precede_single_bracket_links_when_crossing() {
    assert_eq!(
        render("[[span class=\"p\"]]A [http://example.com B[[/span]] C]"),
        "<p><span class=\"p\">A [<a href=\"http://example.com\">http://example.com</a> B</span> C]</p>",
    );
    assert_eq!(
        render("[http://example.com A [[span class=\"p\"]]B] C[[/span]]"),
        "<p>[<a href=\"http://example.com\">http://example.com</a> A <span class=\"p\">B] C</span></p>",
    );
    assert_eq!(
        render("[[size 120%]]A [http://example.com B[[/size]] C]"),
        "<p><span style=\"font-size:120%;\">A [<a href=\"http://example.com\">http://example.com</a> B</span> C]</p>",
    );
    assert_eq!(
        render("[http://example.com A [[size 120%]]B] C[[/size]]"),
        "<p>[<a href=\"http://example.com\">http://example.com</a> A <span style=\"font-size:120%;\">B] C</span></p>",
    );
}

#[test]
fn span_size_and_anchor_suppress_inline_math_when_paired() {
    assert_eq!(
        render("[[span class=\"p\"]]A [[$x[[/span]]y$]]"),
        "<p><span class=\"p\">A <span class=\"math-inline\">$$</span></span></p>",
    );
    assert_eq!(
        render("[[$[[span class=\"p\"]]x[[/span]]$]]"),
        "<p><span class=\"math-inline\">$$</span></p>",
    );
    assert_eq!(
        render("[[size 120%]]A [[$x[[/size]]y$]]"),
        "<p><span style=\"font-size:120%;\">A <span class=\"math-inline\">$$</span></span></p>",
    );
    assert_eq!(
        render("[[$[[size 120%]]x[[/size]]$]]"),
        "<p><span class=\"math-inline\">$$</span></p>",
    );
}
