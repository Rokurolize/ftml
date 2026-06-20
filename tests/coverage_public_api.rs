use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::Render;
use ftml::render::text::TextRender;
use ftml::settings::{WikitextMode, WikitextSettings};
use ftml::tree::{Element, SyntaxTree};
use std::borrow::Cow;

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("coverage-page"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("coverage"),
        title: Cow::Borrowed("Coverage Page"),
        alt_title: None,
        score: ScoreValue::Integer(7),
        tags: vec![Cow::Borrowed("test")],
        language: Cow::Borrowed("default"),
    }
}

fn parse_and_render_text(input: &str) -> String {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
    let mut text = input.to_owned();

    ftml::preprocess(&mut text);
    let tokens = ftml::tokenize(&text);
    let result = ftml::parse(&tokens, &page_info, &settings);
    let (tree, errors) = result.into();

    assert!(errors.is_empty(), "{errors:?}");
    TextRender.render(&tree, &page_info, &settings)
}

#[test]
fn public_api_text_render_covers_mixed_block_elements() {
    let output = parse_and_render_text(
        r#"[[collapsible show="Show" hide="Hide"]]
Visible text
[[/collapsible]]

[[code type="rust"]]
fn main() {}
[[/code]]

[[module Rate]]

||~ Head || Cell ||
|| A || B ||

* one
* two
"#,
    );

    assert!(output.contains("Visible text"));
    assert!(output.contains("fn main() {}"));
    assert!(output.contains("HeadCell"));
    assert!(output.contains("one"));
    assert!(output.contains("two"));
}

#[test]
fn public_api_text_render_entrypoints_trim_outer_newlines() {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
    let elements = vec![
        Element::LineBreak,
        Element::Text(Cow::Borrowed("body")),
        Element::LineBreak,
    ];

    assert_eq!(
        TextRender.render_partial(&elements, &page_info, &settings, 9),
        "body",
    );

    let tree = SyntaxTree {
        elements,
        wikitext_len: 9,
        ..SyntaxTree::default()
    };
    assert_eq!(TextRender.render(&tree, &page_info, &settings), "body");
}
