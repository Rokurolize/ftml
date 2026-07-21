use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::parsing::ParseError;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::time::{Duration, Instant};

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("raw-url-delimiter"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("coverage"),
        title: Cow::Borrowed("Raw URL delimiter"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![Cow::Borrowed("test")],
        language: Cow::Borrowed("en"),
    }
}

fn render_wikidot(input: &str) -> (String, Vec<ParseError>) {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let tokenization = ftml::tokenize(input);
    let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    let html = HtmlRender.render(&tree, &page_info, &settings).body;
    (html, errors)
}

#[test]
fn yossistyle_raw_import_nests_inside_monospace_and_bold_without_errors() {
    let input = "**{{@import url(@<https://scpwiki.com/theme:yossistyle/code/1>@);}}**";
    let (html, errors) = render_wikidot(input);

    assert!(
        errors.is_empty(),
        "tokens: {:#?}\nerrors: {errors:#?}",
        ftml::tokenize(input).tokens(),
    );
    assert_eq!(
        html,
        concat!(
            "<p><strong><tt>@import url(",
            "<span style=\"white-space: pre-wrap;\">",
            "https://scpwiki.com/theme:yossistyle/code/1",
            "</span>);</tt></strong></p>",
        ),
    );
}

#[test]
fn url_terminated_raw_spans_work_at_each_inline_nesting_depth() {
    let cases = [
        (
            "@<https://example.com/raw>@",
            concat!(
                "<p><span style=\"white-space: pre-wrap;\">",
                "https://example.com/raw</span></p>",
            ),
        ),
        (
            "{{@<https://example.com/raw>@}}",
            concat!(
                "<p><tt>",
                "<span style=\"white-space: pre-wrap;\">",
                "https://example.com/raw</span></tt></p>",
            ),
        ),
        (
            "**{{@<https://example.com/raw>@}}**",
            concat!(
                "<p><strong><tt>",
                "<span style=\"white-space: pre-wrap;\">",
                "https://example.com/raw</span></tt></strong></p>",
            ),
        ),
    ];

    for (input, expected) in cases {
        let (html, errors) = render_wikidot(input);
        assert!(errors.is_empty(), "{input:?}: {errors:#?}");
        assert_eq!(html, expected, "{input:?}");
    }
}

#[test]
fn malformed_and_crossed_inline_delimiters_remain_fail_closed() {
    let (unclosed_html, unclosed_errors) = render_wikidot("@<https://example.com/raw");
    assert!(!unclosed_errors.is_empty());
    assert!(unclosed_html.contains("@&lt;"), "{unclosed_html}");

    let crossed = "{{@<https://example.com/raw}}>@";
    let (crossed_html, crossed_errors) = render_wikidot(crossed);
    assert!(!crossed_errors.is_empty());
    assert!(!crossed_html.contains("<tt>"), "{crossed_html}");
    assert!(
        crossed_html.contains("https://example.com/raw}}"),
        "{crossed_html}",
    );
}

#[test]
fn adjacent_raw_spans_split_at_their_own_url_closers() {
    let input = "@<https://a.example/raw>@@<https://b.example/raw>@";
    let (html, errors) = render_wikidot(input);

    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(html.matches("white-space: pre-wrap;").count(), 2, "{html}");
    assert!(html.contains("https://a.example/raw"), "{html}");
    assert!(html.contains("https://b.example/raw"), "{html}");

    let (ordinary_html, ordinary_errors) = render_wikidot("https://example.com/a>b");
    assert!(ordinary_errors.is_empty(), "{ordinary_errors:#?}");
    assert!(
        ordinary_html.contains("https://example.com/a&gt;b"),
        "{ordinary_html}"
    );

    let (reserved_html, reserved_errors) = render_wikidot("https://example.com/a>@b");
    assert!(reserved_errors.is_empty(), "{reserved_errors:#?}");
    let reserved_link = "href=\"https://example.com/a\">https://example.com/a</a>&gt;@b";
    assert!(reserved_html.contains(reserved_link), "{reserved_html}",);
}

#[test]
fn repeated_nested_raw_url_delimiters_parse_in_bounded_time() {
    const ROW_COUNT: usize = 4_096;
    let row = "**{{@import url(@<https://scpwiki.com/theme:yossistyle/code/1>@);}}**\n";
    let input = row.repeat(ROW_COUNT);
    let started = Instant::now();

    let (html, errors) = render_wikidot(&input);

    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(html.matches("white-space: pre-wrap;").count(), ROW_COUNT);
    assert_eq!(html.matches("<strong>").count(), ROW_COUNT);
}
