use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender, text::TextRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("line-ownership"),
        category: None,
        site: Cow::Borrowed("compatibility"),
        title: Cow::Borrowed("Line ownership"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![],
        language: Cow::Borrowed("en"),
    }
}

fn render(source: &str) -> (String, String) {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokenization = ftml::tokenize(&source);
    let (tree, _) = ftml::parse(&tokenization, &page_info, &settings).into();
    let html = HtmlRender.render(&tree, &page_info, &settings).body;
    let text = TextRender.render(&tree, &page_info, &settings);
    (html, text)
}

#[test]
fn code_and_math_require_a_physical_line_owner() {
    for block in ["code", "math"] {
        let source = format!("BEFORE|[[{block}]]\nX\n[[/{block}]]|AFTER");
        let (html, text) = render(&source);
        assert_eq!(text, source, "{block}: {html}");
        assert!(!html.contains("class=\"code\""), "{block}: {html}");
        assert!(!html.contains("math-equation"), "{block}: {html}");

        let source = format!("* [[{block}]]\nX\n[[/{block}]]");
        let (html, text) = render(&source);
        assert!(text.contains(&format!("[[{block}]]")), "{block}: {html}");
        assert!(text.contains(&format!("[[/{block}]]")), "{block}: {html}");
        assert!(!html.contains("class=\"code\""), "{block}: {html}");
        assert!(!html.contains("math-equation"), "{block}: {html}");
    }

    for prefix in ["", " ", "    ", "\t"] {
        let (html, _) = render(&format!("{prefix}[[code]]\nX\n[[/code]]"));
        assert!(html.contains("<div class=\"code\">"), "{prefix:?}: {html}");

        let (html, _) = render(&format!("{prefix}[[math]]\nX\n[[/math]]"));
        assert!(html.contains("math-equation"), "{prefix:?}: {html}");
    }
}

#[test]
fn modules_require_an_unprefixed_physical_line() {
    const MODULES: &str = concat!(
        "Rate CSS ListPages ListUsers MailForm Redirect CountPages Members PageTree ",
        "Comments NewPage PageCalendar TagCloud ThemePreviewer PagesByTag Categories ",
        "Feed Search Join Pages WhoInvited Backlinks FrontForum NextPage Password ",
        "PreviousPage RecentPosts RecentThreads SearchAll SiteChanges AdSenseUnit ",
        "ManageSite OrphanedPages RatedPages SearchUsers WantedPages Watchers Audio ",
        "CreateAccount CurrencyConvert FeaturedSite FlickrGallery ForumCategory ",
        "ForumNewThread ForumStart ForumThread ListDrafts MembershipApply ",
        "MiniActiveThreads MiniRecentPosts MiniRecentThreads NewSite SimpleToDo ",
        "SiteGrid UserInfo Ad AdModuleAboveContent AdModuleAboveSidebar ",
        "AdModuleBelowContent AdModuleBelowFooter AdModuleBelowSidebar ",
        "AnonymousNotificationsUnsubscribe ChildPages Clone Dashboard DeleteAccount ",
        "Files FooterBar FrontSpecialMini LoginStatus MembershipByPassword ",
        "MembershipEmailInvitation NaviBar PageOptionsBottom PetitionAdmin ",
        "SendInvitations SitesTagCloud",
    );

    for module in MODULES.split_whitespace() {
        let source = format!("BEGIN|[[module {module}]]|END");
        let (html, text) = render(&source);
        assert_eq!(text, source, "{module}: {html}");
        assert!(!html.contains("TODO: module"), "{module}: {html}");
        assert!(!html.contains("error-block"), "{module}: {html}");
    }

    let (html, text) = render(" [[module Rate]]");
    assert_eq!(text, "[[module Rate]]");
    assert!(!html.contains("TODO: module"), "{html}");
}

// Fixture: Rokurolize/ftml#478. Frozen Wikidot provenance: test--div--basic.
#[test]
fn same_line_div_after_heading_stays_outside_a_paragraph() {
    let (html, text) = render("++ Regular\n\n[[div]]inline[[/div]]");
    assert_eq!(text, "Regular\n\n[[div]]inline[[/div]]");
    assert_eq!(
        html,
        "<h2 id=\"toc0\"><span>Regular</span></h2>\n[[div]]inline[[/div]]",
    );
}

#[test]
fn same_line_div_literal_controls_follow_physical_line_ownership() {
    for (source_literal, html_literal) in [
        ("[[div]]inline[[/div]]", "[[div]]inline[[/div]]"),
        ("[[div_]]inline[[/div]]", "[[div_]]inline[[/div]]"),
        (
            "[[div =\"value\"]]inline[[/div]]",
            "[[div =&quot;value&quot;]]inline[[/div]]",
        ),
    ] {
        let (html, text) = render(&format!("BEFORE\n{source_literal}"));
        assert_eq!(html, format!("<p>BEFORE</p>\n{html_literal}"));
        assert_eq!(text, format!("BEFORE\n\n{source_literal}"));

        let (html, text) = render(source_literal);
        assert_eq!(html, format!("<p>{html_literal}</p>"));
        assert_eq!(text, source_literal);

        let source = format!("BEFORE|{source_literal}|AFTER");
        let (html, text) = render(&source);
        assert_eq!(html, format!("<p>BEFORE|{html_literal}|AFTER</p>"));
        assert_eq!(text, source);
    }
}

#[test]
fn same_line_div_nested_span_falls_back_to_inline_parsing() {
    const DIV: &str =
        "[[div]] [[span]] [[ruby]]語 [[rt]]go[[/rt]][[/ruby]] [[/span]] [[/div]]";
    const SPAN: &str =
        "[[div]] <span>[[ruby]]語 [[rt]]go[[/rt]][[/ruby]]</span> [[/div]]";
    const TEXT: &str = "[[div]] [[ruby]]語 [[rt]]go[[/rt]][[/ruby]] [[/div]]";

    for (source, expected_html, expected_text) in [
        (DIV.to_owned(), format!("<p>{SPAN}</p>"), TEXT.to_owned()),
        (
            format!("NESTED\n{DIV}"),
            format!("<p>NESTED<br>\n{SPAN}</p>"),
            format!("NESTED\n{TEXT}"),
        ),
    ] {
        let (html, text) = render(&source);
        assert_eq!(html, expected_html);
        assert_eq!(text, expected_text);
    }
}

#[test]
fn same_line_div_with_unmatched_span_close_stays_literal() {
    let source = "[[div]] literal [[/span]] [[/div]]";
    let (html, text) = render(source);
    assert_eq!(html, "<p>[[div]] literal [[/span]] [[/div]]</p>");
    assert_eq!(text, source);
}

#[test]
fn same_line_div_span_ownership_requires_a_real_bounded_opener() {
    for source in [
        "[[div]] span [[ruby]]x[[/ruby]] [[/span]] [[/div]]",
        "[[div]] [[span]]x[[/div]] [[/span]]",
        "[[div]] [[span]]@@[[/span]]@@ [[/div]]",
    ] {
        let (html, text) = render(source);
        assert_eq!(html, format!("<p>{source}</p>"));
        assert_eq!(text, source);
    }
}

#[test]
fn same_line_div_span_marker_controls_stay_clean() {
    for (name, source, expected_html, expected_text) in [
        (
            "raw",
            "[[div]] @@[[/span]]@@ [[/div]]",
            "<p>[[div]] @@[[/span]]@@ [[/div]]</p>",
            "[[div]] @@[[/span]]@@ [[/div]]",
        ),
        (
            "comment",
            "[[div]] [!-- [[span]]x[[/span]] --] [[/div]]",
            "<p>[[div]] [!-- [[span]]x[[/span]] --] [[/div]]</p>",
            "[[div]] [!-- [[span]]x[[/span]] --] [[/div]]",
        ),
        (
            "multiline",
            "[[div]]\n[[span]]x\n[[/span]]\n[[/div]]",
            "<div><p><span>x<br>\n</span></p></div>",
            "x",
        ),
    ] {
        let (html, text) = render(source);
        assert_eq!(html, expected_html, "{name}");
        assert_eq!(text, expected_text, "{name}");
    }
}

#[test]
fn css_requires_an_unprefixed_multiline_invocation() {
    for source in [
        "[[module CSS]]x{}[[/module]]",
        "P[[module CSS]]x{}[[/module]]S",
        " [[module CSS]]\nx{}\n[[/module]]",
        "* [[module CSS]]\n* x{}\n* [[/module]]",
    ] {
        let (html, text) = render(source);
        assert!(text.contains("[[module CSS]]"), "{source:?}: {html}");
        assert!(!html.is_empty(), "{source:?}");
    }

    let (html, text) = render("[[module CSS]]\nx{}\n[[/module]]");
    assert!(html.is_empty(), "{html}");
    assert!(text.is_empty(), "{text}");
}

#[test]
fn toc_and_bibliography_use_case_sensitive_line_ownership() {
    for source in ["[[TOC]]", "[[F<TOC]]", "[[F>TOC]]"] {
        let (html, text) = render(source);
        assert_eq!(text, source, "{html}");
        assert!(!html.contains("Table of Contents"), "{html}");
    }

    let (html, text) = render("start-[[toc]]-middle\n\n[[toc]]");
    assert_eq!(text, "start-[[toc]]-middle");
    assert_eq!(html.matches("Table of Contents").count(), 1, "{html}");

    let source = "[[BIBLIOGRAPHY]]\nv7 body\n[[/BIBLIOGRAPHY]]";
    let (html, text) = render(source);
    assert_eq!(text, source, "{html}");
    assert!(!html.contains("class=\"bibitems\""), "{html}");

    let source = "start-[[bibliography]]\nv7 body\n[[/bibliography]]-middle";
    let (html, text) = render(source);
    assert_eq!(text, source, "{html}");
    assert!(!html.contains("class=\"bibitems\""), "{html}");
}

#[test]
fn unquoted_structural_lines_end_native_blockquotes() {
    for source in [
        "> * A\n* B",
        "> # A\n# B",
        "> + A\n+ B",
        "> || A ||\n|| B ||",
    ] {
        let (html, _) = render(source);
        let close = html.find("</blockquote>").expect("blockquote closes");
        let outside = &html[close + "</blockquote>".len()..];
        assert!(!outside.is_empty(), "{source:?}: {html}");
        assert_eq!(
            html.matches("<blockquote>").count(),
            1,
            "{source:?}: {html}"
        );
    }
}
