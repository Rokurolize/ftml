//! Timing regressions for adversarial inputs that once took super-linear
//! (including exponential) time. Budgets are generous for debug builds; the
//! pre-fix behaviour exceeded them by orders of magnitude.

use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::Render;
use ftml::render::html::HtmlRender;
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::time::{Duration, Instant};

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("super-linear"),
        category: None,
        site: Cow::Borrowed("test"),
        title: Cow::Borrowed("Super linear"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![],
        language: Cow::Borrowed("default"),
    }
}

fn render_timed(input: &str) -> (String, Duration) {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let started = Instant::now();
    let mut text = input.to_owned();
    ftml::preprocess_for_layout(&mut text, Layout::Wikidot);
    let tokens = ftml::tokenize(&text);
    let (tree, _errors) = ftml::parse(&tokens, &page_info, &settings).into();
    let html = HtmlRender.render(&tree, &page_info, &settings).body;
    (html, started.elapsed())
}

#[test]
fn unclosed_inline_marker_runs_past_the_recursion_limit_stay_bounded() {
    // ~1,100 unclosed openers overrun the default recursion limit. Every
    // level used to retry the remaining rules, doubling the work per level.
    for unit in ["**//__--^^,,{{a ", "**//__--^^,,{{a\n"] {
        let (html, elapsed) = render_timed(&unit.repeat(160));
        assert!(
            elapsed < Duration::from_secs(3),
            "{unit:?} took {elapsed:?}"
        );
        assert!(html.contains('a'), "{unit:?}");
    }
}

#[test]
fn comment_opened_in_a_div_and_closed_after_it_stays_bounded() {
    let (html, elapsed) = render_timed(&"[[div]]\n[!--\n[[/div]]\n--]\n".repeat(256));
    assert!(elapsed < Duration::from_secs(3), "took {elapsed:?}");
    let _ = html;
}

#[test]
fn repeated_unclosed_tab_inside_tabview_stays_bounded() {
    let (html, elapsed) = render_timed(&"[[tabview]]\n[[tab x]]\ny\n".repeat(128));
    assert!(elapsed < Duration::from_secs(3), "took {elapsed:?}");
    let _ = html;
}

#[test]
fn repeated_closed_iftags_gates_stay_bounded() {
    for (condition, shown) in [("-x", true), ("+x", false)] {
        let unit = format!("[[iftags {condition}]]\nbody-text\n[[/iftags]]\n");
        let (html, elapsed) = render_timed(&unit.repeat(128));
        assert!(elapsed < Duration::from_secs(3), "{condition}: {elapsed:?}");
        assert_eq!(html.contains("body-text"), shown, "{condition}");
    }
}
