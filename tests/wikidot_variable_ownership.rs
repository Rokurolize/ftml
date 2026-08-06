use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{PageExistenceResolver, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;

struct MissingPages;

impl PageExistenceResolver for MissingPages {
    fn page_exists(&self, _site: &str, _page: &str) -> bool {
        false
    }
}

fn render(source: &str) -> String {
    let page_info = PageInfo {
        page: Cow::Borrowed("variable-ownership"),
        category: None,
        site: Cow::Borrowed("syntax-differential"),
        title: Cow::Borrowed("Variable ownership"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    };
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokens = ftml::tokenize(&source);
    let (tree, errors) = ftml::parse(&tokens, &page_info, &settings).into();
    assert!(errors.is_empty(), "{source:?}: {errors:#?}");
    HtmlRender
        .render_with_page_existence(&tree, &page_info, &settings, &MissingPages)
        .body
}

#[test]
fn wikidot_variables_keep_text_authority_but_not_attribute_authority() {
    assert_eq!(
        render(r#"[[span title="{$x}"]]A[[/span]]"#),
        "<p><span>A</span></p>",
    );
    assert_eq!(render("A{$x}B"), "<p>A{$x}B</p>");
    assert_eq!(
        render("[[[scp-002|{$x}]]]"),
        "<p><a class=\"newpage\" href=\"/scp-002\">{$x}</a></p>"
    );
    assert_eq!(
        render("[[[{$target}|A]]]"),
        "<p><a class=\"newpage\" href=\"/target\">A</a></p>"
    );
    assert_eq!(render("{{$x}}"), "<p><tt>$x</tt></p>");
    assert_eq!(render(r"\{$x}"), r"<p>\{$x}</p>");
    assert_eq!(render("{$foo-bar}"), "<p>{$foo-bar}</p>");
    assert_eq!(render("{$foo_bar}"), "<p>{$foo_bar}</p>");
}
