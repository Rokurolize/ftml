use std::borrow::Cow;

use crate::data::PageInfo;
use crate::layout::Layout;
use crate::render::Render;
use crate::settings::{WikitextMode, WikitextSettings};
use crate::tree::{
    Bibliography, BibliographyList, Element, FootnoteRef, SyntaxTree, Tab, TabViewItem,
};

use super::HtmlRender;

fn page(layout: Layout, tree: &SyntaxTree) -> String {
    let info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
    HtmlRender.render(tree, &info, &settings).body
}

fn text(value: &'static str) -> Element<'static> {
    Element::Text(Cow::Borrowed(value))
}

fn math(name: &'static str, source: &'static str) -> Element<'static> {
    Element::Math {
        name: Some(Cow::Borrowed(name)),
        latex_source: Cow::Borrowed(source),
        environment: None,
        wikidot_closer_line_br_prefix: false,
        wikidot_block_open_elements: Vec::new(),
        wikidot_label_spans: Vec::new(),
        wikidot_capture_eq: false,
    }
}

fn tab(label: &str, elements: Vec<Element<'static>>) -> TabViewItem<'static> {
    TabViewItem::Tab(Tab {
        label: Cow::Owned(label.to_string()),
        elements,
        wikidot_label_spans: Vec::new(),
        wikidot_row_cut: None,
    })
}

fn yui_navs(html: &str) -> Vec<String> {
    let mut navs = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<ul class=\"yui-nav\">") {
        let after = &rest[start..];
        let end = after
            .find("</ul>")
            .map(|index| index + "</ul>".len())
            .unwrap_or(after.len());
        navs.push(after[..end].to_string());
        rest = &after[end..];
    }
    navs
}

#[test]
fn footnote_self_cite_stays_bounded_and_releases_the_guard() {
    let tree = SyntaxTree {
        elements: vec![
            Element::Footnote(FootnoteRef::new(1)),
            Element::Footnote(FootnoteRef::new(1)),
        ],
        footnotes: vec![vec![text("alpha"), Element::Footnote(FootnoteRef::new(1))]],
        ..SyntaxTree::default()
    };
    let html = page(Layout::Wikijump, &tree);
    assert_eq!(
        html.matches("alpha").count(),
        2,
        "each citation renders the body once, {html}"
    );
    assert!(
        html.matches("wj-error-inline").count() >= 2,
        "self-cite renders the missing footnote, {html}"
    );
}

#[test]
fn bibliography_self_cite_stays_bounded_and_releases_the_guard() {
    let mut bibliography = Bibliography::new();
    bibliography.add(
        Cow::Borrowed("a"),
        vec![
            text("beta"),
            Element::BibliographyCite {
                label: Cow::Borrowed("a"),
                brackets: false,
                inline: true,
            },
        ],
    );
    let mut bibliographies = BibliographyList::new();
    bibliographies.push(bibliography);
    let tree = SyntaxTree {
        elements: vec![
            Element::BibliographyCite {
                label: Cow::Borrowed("a"),
                brackets: false,
                inline: true,
            },
            Element::BibliographyCite {
                label: Cow::Borrowed("a"),
                brackets: false,
                inline: true,
            },
        ],
        bibliographies,
        ..SyntaxTree::default()
    };
    let html = page(Layout::Wikijump, &tree);
    assert_eq!(
        html.matches("beta").count(),
        2,
        "each citation renders the entry once, {html}"
    );
    assert!(
        html.contains("not found"),
        "self-cite renders the missing entry, {html}"
    );
}

#[test]
fn legacy_equation_numbers_follow_footnote_references() {
    let tree = SyntaxTree {
        elements: vec![
            Element::EquationReference(Cow::Borrowed("a")),
            Element::Footnote(FootnoteRef::new(1)),
            math("b", "y"),
        ],
        footnotes: vec![vec![math("a", "x")]],
        needs_footnote_block: true,
        ..SyntaxTree::default()
    };
    let html = page(Layout::Wikidot, &tree);
    let footer = html.find("footnotes-footer").expect("footnote footer");
    let head = &html[..footer];
    let tail = &html[footer..];
    assert!(
        head.contains("class=\"eref\""),
        "forward eref is in the body, {html}"
    );
    assert!(
        head.contains(">1</a>"),
        "eref resolves to the footnote equation, {html}"
    );
    assert!(
        head.contains("id=\"equation-2\""),
        "page equation keeps the later number, {html}"
    );
    assert!(
        !head.contains("id=\"equation-1\""),
        "the first number is not spent before the footnote, {html}"
    );
    assert!(
        tail.contains("id=\"equation-1\""),
        "footnote equation keeps the reference number, {html}"
    );
    assert!(tail.contains("(1)"), "{html}");
    assert!(head.contains("(2)"), "{html}");
}

#[test]
fn nested_tabview_nav_is_the_flat_assignment() {
    let tree = SyntaxTree {
        elements: vec![Element::TabView(vec![
            tab(
                "Apple",
                vec![Element::TabView(vec![tab("Banana", vec![text("panel")])])],
            ),
            tab("Cherry", vec![text("later")]),
        ])],
        ..SyntaxTree::default()
    };
    let html = page(Layout::Wikidot, &tree);
    let navs = yui_navs(&html);
    assert!(navs.len() >= 2, "outer and inner navs, {html}");
    assert!(
        navs[0].contains("Apple") && !navs[0].contains("Cherry"),
        "outer nav is the flat assignment, {}",
        navs[0]
    );
    assert!(
        navs[1].contains("Banana") && navs[1].contains("Cherry"),
        "inner nav receives the following label, {}",
        navs[1]
    );
}

#[test]
fn remote_tab_keeps_hierarchical_navs() {
    let tree = SyntaxTree {
        elements: vec![Element::TabView(vec![
            tab(
                "Apple",
                vec![Element::TabView(vec![tab("Banana", vec![text("panel")])])],
            ),
            TabViewItem::Remote(Tab {
                label: Cow::Borrowed("Cherry"),
                elements: Vec::new(),
                wikidot_label_spans: Vec::new(),
                wikidot_row_cut: None,
            }),
        ])],
        ..SyntaxTree::default()
    };
    let html = page(Layout::Wikidot, &tree);
    let navs = yui_navs(&html);
    assert!(navs.len() >= 2, "{html}");
    assert!(
        navs[0].contains("Apple") && navs[0].contains("Cherry"),
        "a remote tab disables the permutation, {}",
        navs[0]
    );
    assert!(
        navs[1].contains("Banana") && !navs[1].contains("Cherry"),
        "the inner view keeps its own label, {}",
        navs[1]
    );
}

fn stray_view(label: &str) -> Element<'static> {
    Element::TabView(vec![tab(
        label,
        vec![Element::TabView(vec![
            tab("Inner", vec![text("panel")]),
            TabViewItem::Content(vec![text("[[/tab]]")]),
        ])],
    )])
}

#[test]
fn each_outer_tabview_consumes_its_own_lift() {
    let tree = SyntaxTree {
        elements: vec![stray_view("First"), stray_view("Second")],
        ..SyntaxTree::default()
    };
    let html = page(Layout::Wikidot, &tree);
    let navs = yui_navs(&html);
    assert!(navs.len() >= 2, "{html}");
    let second = html
        .find(&navs[navs.len() / 2])
        .or_else(|| html.find("<em>Second</em>"))
        .expect("second view");
    let (left, right) = html.split_at(second);
    assert_eq!(
        left.matches("<br>[[/tab]]").count(),
        1,
        "first view emits its lift, {html}"
    );
    assert_eq!(
        right.matches("<br>[[/tab]]").count(),
        1,
        "second view emits its own lift, {html}"
    );
}

#[test]
fn content_only_outer_does_not_leak_a_lift() {
    let tree = SyntaxTree {
        elements: vec![
            Element::TabView(vec![TabViewItem::Content(vec![Element::TabView(vec![
                tab("Inner", vec![text("panel")]),
                TabViewItem::Content(vec![text("[[/tab]]")]),
            ])])]),
            stray_view("After"),
        ],
        ..SyntaxTree::default()
    };
    let html = page(Layout::Wikidot, &tree);
    let after = html.find("<em>After</em>").expect("following view");
    let head = &html[..after];
    assert_eq!(
        head.matches("<br>[[/tab]]").count(),
        0,
        "content-only view does not emit or leave a lift, {html}"
    );
    assert_eq!(
        html[after..].matches("<br>[[/tab]]").count(),
        1,
        "the next outermost view still lifts its own stray, {html}"
    );
}
