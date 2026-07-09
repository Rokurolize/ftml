use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::Render;
use ftml::render::html::HtmlRender;
use ftml::render::text::TextRender;
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("canonical-source"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("coverage"),
        title: Cow::Borrowed("Canonical Source"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![Cow::Borrowed("test")],
        language: Cow::Borrowed("en"),
    }
}

fn render_text_and_html(input: &str) -> (String, String) {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut text = input.to_owned();
    ftml::preprocess(&mut text);
    let tokens = ftml::tokenize(&text);
    let result = ftml::parse(&tokens, &page_info, &settings);
    let (tree, errors) = result.into();

    assert!(errors.is_empty(), "{errors:?}");

    let text_output = TextRender.render(&tree, &page_info, &settings);
    let html_output = HtmlRender.render(&tree, &page_info, &settings).body;
    (text_output, html_output)
}

#[test]
fn wikidot_content_section_marker_lines_do_not_render() {
    let (text, html) = render_text_and_html("before\n====\nmiddle\n=====\nafter");

    assert!(text.contains("before"));
    assert!(text.contains("middle"));
    assert!(text.contains("after"));
    assert!(!text.contains("===="), "{text}");
    assert!(!html.contains("===="), "{html}");
}

#[test]
fn wikidot_section_marker_rule_preserves_existing_equal_and_literal_contexts() {
    let (text, html) = render_text_and_html(
        "= centered\n++ Heading\n[[code]]\n====\n[[/code]]\n@@====@@",
    );

    assert!(text.contains("centered"));
    assert!(text.contains("Heading"));
    assert!(text.contains("===="), "{text}");
    assert!(html.contains("text-align: center"), "{html}");
    assert!(html.contains("<h2"), "{html}");
    assert!(html.contains("===="), "{html}");
}

#[test]
fn wikidot_canonical_unclosed_block_markers_do_not_render_as_text() {
    let (text, html) = render_text_and_html(
        r#"[[iftags +test]]
[[div_ class="authorlink-wrapper"]]
Calibold"#,
    );

    assert_eq!(text, "Calibold");
    assert!(!html.contains("[[iftags"), "{html}");
    assert!(!html.contains("[[div_"), "{html}");
    assert!(
        html.contains(r#"<div class="authorlink-wrapper">Calibold</div>"#),
        "{html}"
    );
}
