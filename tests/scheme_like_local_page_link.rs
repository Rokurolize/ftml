use ftml::data::{PageInfo, ScoreValue};
use ftml::includes::NullIncluder;
use ftml::layout::Layout;
use ftml::render::Render;
use ftml::render::html::HtmlRender;
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;

fn render(source: &str) -> String {
    let info = PageInfo {
        page: Cow::Borrowed("scheme-link"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("ast-test"),
        title: Cow::Borrowed("Scheme link"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![],
        language: Cow::Borrowed("default"),
    };
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let (mut text, _p) =
        ftml::include(source, &settings, NullIncluder, || unreachable!())
            .unwrap_or_else(|x| match x {});
    ftml::preprocess(&mut text);
    let tokens = ftml::tokenize(&text);
    let (tree, _errors) = ftml::parse(&tokens, &info, &settings).into();
    HtmlRender.render(&tree, &info, &settings).body
}

#[test]
fn local_page_link_with_scheme_like_category_is_not_blocked() {
    let html = render("[[[data:example]]]");
    assert!(
        html.contains("href=\"/data:example\""),
        "scheme-like local page link must be a local path, got {html:?}"
    );
    assert!(
        !html.contains("#invalid-url"),
        "scheme-like local page link must not be blocked, got {html:?}"
    );
}

#[test]
fn dangerous_url_targets_stay_blocked() {
    for source in [
        r#"[[a href="javascript:alert(1)"]]XSS[[/a]]"#,
        r#"[[a href="data:text/plain;base64,SGVsbG8="]]XSS[[/a]]"#,
    ] {
        let html = render(source);
        assert!(
            html.contains("href=\"#invalid-url\""),
            "dangerous URL target must stay blocked for {source:?}, got {html:?}"
        );
    }
}
