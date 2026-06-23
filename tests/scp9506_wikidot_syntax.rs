use ftml::data::{PageInfo, PageRef, ScoreValue};
use ftml::includes::{FetchedPage, IncludeRef, Includer};
use ftml::layout::Layout;
use ftml::render::Render;
use ftml::render::text::TextRender;
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::collections::HashMap;

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("scp-9506"),
        category: Some(Cow::Borrowed("_default")),
        site: Cow::Borrowed("scp-wiki"),
        title: Cow::Borrowed("National Fog Safety Initiative"),
        alt_title: None,
        score: ScoreValue::Integer(388),
        tags: vec![Cow::Borrowed("scp"), Cow::Borrowed("theme")],
        language: Cow::Borrowed("en"),
    }
}

fn page_settings() -> WikitextSettings {
    WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump)
}

fn assert_parses_without_errors(input: &str) {
    let page_info = page_info();
    let settings = page_settings();
    let mut text = input.to_owned();

    ftml::preprocess(&mut text);
    let tokens = ftml::tokenize(&text);
    let result = ftml::parse(&tokens, &page_info, &settings);
    let (_tree, errors) = result.into();

    assert!(errors.is_empty(), "unexpected parse errors: {errors:?}");
}

fn render_text(input: &str) -> String {
    let page_info = page_info();
    let settings = page_settings();
    let mut text = input.to_owned();

    ftml::preprocess(&mut text);
    let tokens = ftml::tokenize(&text);
    let result = ftml::parse(&tokens, &page_info, &settings);
    let (tree, errors) = result.into();

    assert!(errors.is_empty(), "unexpected parse errors: {errors:?}");
    TextRender.render(&tree, &page_info, &settings)
}

struct StaticIncluder {
    pages: HashMap<String, &'static str>,
}

impl StaticIncluder {
    fn new(pages: impl IntoIterator<Item = (&'static str, &'static str)>) -> Self {
        Self {
            pages: pages
                .into_iter()
                .map(|(key, value)| (key.to_owned(), value))
                .collect(),
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
                let content = self.pages.get(&key).map(|content| Cow::Borrowed(*content));

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

#[test]
fn scp9506_theme_and_component_includes_expand_with_variables() {
    let source = r#"[[include :scp-wiki:theme:basalt | hidetitle=true]]
[[include :scp-wiki:component:acs-animation]]
[[include :scp-wiki:component:mega-cool-author-page-tool
 |inc-custom-list= --]
 |category=SCPs
 |tags=+scp -co-authored
 |order=
 |perpage=250
 |shadow=no
]]
"#;

    let includer = StaticIncluder::new([
        (":scp-wiki:theme:basalt", "BASALT THEME {$hidetitle}"),
        (":scp-wiki:component:acs-animation", "ACS ANIMATION CSS"),
        (
            ":scp-wiki:component:mega-cool-author-page-tool",
            r#"AUTHOR TOOL {$category}
{$inc-custom-list}
[[module ListPages created_by="=" order="{$order}" category="-fragment" tag="{$tags}" perPage="{$perpage}"]]
[[div class="content-box {$shadow}"]]
%%title_linked%%
%%content%%
%%content{1}%%
[[/div]]
[[/module]]
"#,
        ),
    ]);

    let (expanded, pages) = ftml::include(source, &page_settings(), includer, || {
        "invalid include result".to_owned()
    })
    .expect("SCP-9506 include dependencies should parse and expand");

    let page_refs: Vec<String> = pages.into_iter().map(|page| page.to_string()).collect();
    assert_eq!(
        page_refs,
        vec![
            ":scp-wiki:theme:basalt",
            ":scp-wiki:component:acs-animation",
            ":scp-wiki:component:mega-cool-author-page-tool",
        ],
    );

    assert!(expanded.contains("BASALT THEME true"));
    assert!(expanded.contains("ACS ANIMATION CSS"));
    assert!(expanded.contains("AUTHOR TOOL SCPs"));
    assert!(expanded.contains("[[module ListPages"));
    assert!(expanded.contains("order=\"\""));
    assert!(expanded.contains("+scp -co-authored"));
    assert!(expanded.contains("perPage=\"250"));
    assert!(expanded.contains("content-box no"));
    assert!(expanded.contains("%%content%%"));
    assert!(expanded.contains("%%content{1}%%"));
}

#[test]
#[ignore = "RED: FTML does not currently support the ListPages module syntax needed by SCP-9506 author-tool components"]
fn scp9506_expanded_dependency_syntax_parses_without_errors() {
    let expanded_source = r#"[[module CSS]]
@import url(https://scp-wiki.wdfiles.com/local--code/theme%3Abasalt/3);
.nfsi-grid { display: grid; }
[[/module]]

[[div class="nfsi-alert"]]
National Fog Safety Initiative local compatibility page.
[[/div]]

[[image fog-green.svg]]
[[image fog-map.svg]]
[[image alert-card.svg]]

[[module ListPages created_by="=" order="" category="-fragment" tag="+scp -co-authored" perPage="250"]]
[[div class="content-box no"]]
%%title_linked%%
%%content%%
%%content{1}%%
[[/div]]
[[/module]]

====
Section one
====
Section two

[[iframe https://scp-wiki.wikidot.com/common--theme/base/css/style.css]]
"#;

    assert_parses_without_errors(expanded_source);

    let rendered = render_text(expanded_source);
    assert!(
        rendered.contains("National Fog Safety Initiative local compatibility page.")
    );
    assert!(rendered.contains("Section one"));
    assert!(rendered.contains("Section two"));
}
