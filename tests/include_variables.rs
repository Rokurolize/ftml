use ftml::data::{PageInfo, PageRef, ScoreValue};
use ftml::includes::{FetchedPage, IncludeRef, Includer};
use ftml::layout::Layout;
use ftml::render::Render;
use ftml::render::html::HtmlRender;
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::collections::HashMap;

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("include-variables"),
        category: None,
        site: Cow::Borrowed("test"),
        title: Cow::Borrowed("Include Variables"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![],
        language: Cow::Borrowed("default"),
    }
}

fn page_settings() -> WikitextSettings {
    WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot)
}

#[derive(Debug)]
struct StaticIncluder {
    pages: HashMap<&'static str, &'static str>,
}

impl StaticIncluder {
    fn new(pages: impl IntoIterator<Item = (&'static str, &'static str)>) -> Self {
        Self {
            pages: pages.into_iter().collect(),
        }
    }
}

impl<'t> Includer<'t> for StaticIncluder {
    type Error = String;

    fn include_pages(
        &mut self,
        includes: &[IncludeRef<'t>],
    ) -> Result<Vec<FetchedPage<'t>>, Self::Error> {
        Ok(includes
            .iter()
            .map(|include| {
                let page_ref = include.page_ref().clone();
                let key = page_ref.to_string();
                let content = self
                    .pages
                    .get(key.as_str())
                    .map(|content| Cow::Borrowed(*content));

                FetchedPage { page_ref, content }
            })
            .collect())
    }

    fn no_such_include(
        &mut self,
        page_ref: &PageRef,
    ) -> Result<Cow<'t, str>, Self::Error> {
        Err(format!("missing include fixture for {page_ref}"))
    }
}

fn expand(
    source: &str,
    pages: impl IntoIterator<Item = (&'static str, &'static str)>,
) -> String {
    let (expanded, included_pages) =
        ftml::include(source, &page_settings(), StaticIncluder::new(pages), || {
            "invalid include result".to_owned()
        })
        .expect("include expansion should succeed");

    assert_eq!(included_pages, vec![PageRef::page_only("component:card")]);
    expanded
}

fn render_html(source: &str) -> String {
    let mut text = source.to_owned();
    ftml::preprocess(&mut text);
    let tokens = ftml::tokenize(&text);
    let page_info = page_info();
    let settings = page_settings();
    let result = ftml::parse(&tokens, &page_info, &settings);
    let (tree, errors) = result.into();

    assert!(errors.is_empty(), "{errors:?}");
    HtmlRender.render(&tree, &page_info, &settings).body
}

#[test]
fn legacy_include_variables_expand_inside_quoted_div_attributes() {
    let expanded = expand(
        "[[include component:card class=highlight|data_value=42]]",
        [(
            "component:card",
            r#"[[div class="card {$class}" data-value="{$data_value}"]]Body[[/div]]"#,
        )],
    );

    assert!(expanded.contains(r#"class="card highlight""#), "{expanded}");
    assert!(expanded.contains(r#"data-value="42""#), "{expanded}");

    let html = render_html(&expanded);

    assert!(
        html.contains(r#"<div class="card highlight" data-value="42">"#),
        "{html}"
    );
}

#[test]
fn include_argument_variable_pass_through_uses_fallback_value() {
    let expanded = expand(
        "[[include component:card class={$class}|class=default]]",
        [("component:card", "{$class}")],
    );

    assert_eq!(expanded, "default");
}

#[test]
fn include_argument_first_concrete_value_wins_over_fallback() {
    let expanded = expand(
        "[[include component:card class=selected|class=default]]",
        [("component:card", "{$class}")],
    );

    assert_eq!(expanded, "selected");
}

#[test]
fn rendered_div_and_span_keep_safe_attributes_and_filter_unsafe_attributes() {
    let html = render_html(
        r#"[[div class="card" id="panel" style="color: red" data-value="42" aria-label="Panel" onclick="alert(1)" data-="bad"]]
[[span class="badge" id="tag" style="font-weight: bold" data-kind="status" aria-live="polite" onmouseover="alert(1)" aria-="bad"]]Open[[/span]]
[[/div]]"#,
    );

    for expected in [
        r#"class="card""#,
        r#"id="u-panel""#,
        r#"style="color: red""#,
        r#"data-value="42""#,
        r#"aria-label="Panel""#,
        r#"class="badge""#,
        r#"id="u-tag""#,
        r#"style="font-weight: bold""#,
        r#"data-kind="status""#,
        r#"aria-live="polite""#,
    ] {
        assert!(html.contains(expected), "missing {expected} in {html}");
    }

    for forbidden in [
        "onclick",
        "onmouseover",
        r#"data-=""#,
        r#"aria-=""#,
        "alert(1)",
    ] {
        assert!(!html.contains(forbidden), "found {forbidden} in {html}");
    }
}
