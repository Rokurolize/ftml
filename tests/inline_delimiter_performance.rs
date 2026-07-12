use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use ftml::tree::Element;
use std::borrow::Cow;
use std::time::{Duration, Instant};

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("inline-delimiter-performance"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("coverage"),
        title: Cow::Borrowed("Inline delimiter performance"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![Cow::Borrowed("test")],
        language: Cow::Borrowed("en"),
    }
}

#[test]
fn padded_inline_openers_inside_list_items_stay_literal_in_bounded_time() {
    const ROW_COUNT: usize = 128;

    for marker in ["**", "//", "__", "^^", ",,"] {
        let row = format!("# [[size 0%]]{marker} [[/size]]\n");
        let input = row.repeat(ROW_COUNT);
        let page_info = page_info();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        let started = Instant::now();
        let tokenization = ftml::tokenize(&input);
        let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        let elapsed = started.elapsed();

        assert!(elapsed < Duration::from_secs(5), "{marker:?}: {elapsed:?}");
        assert_eq!(tree.elements.len(), 1, "{marker:?}: {:#?}", tree.elements);
        let Element::List { items, .. } = &tree.elements[0] else {
            panic!("{marker:?}: expected one list, got {:#?}", tree.elements);
        };
        assert_eq!(items.len(), ROW_COUNT, "{marker:?}");
        assert_eq!(html.matches(marker).count(), ROW_COUNT, "{marker:?}");
        assert!(errors.is_empty(), "{marker:?}: {errors:#?}");
    }
}

#[test]
fn repeated_underline_spacer_run_stays_linear_and_literal() {
    const MARKER_COUNT: usize = 16_384;

    let input = format!("{} ", "__".repeat(MARKER_COUNT));
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let started = Instant::now();

    let tokenization = ftml::tokenize(&input);
    let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    let html = HtmlRender.render(&tree, &page_info, &settings).body;

    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(html.matches("__").count(), MARKER_COUNT);
}

#[test]
fn quoted_multiline_inline_delimiter_pairs_stay_bounded() {
    const PAIR_COUNT: usize = 64;

    // Reduced from EN:indigo-eyes (source SHA-256
    // cfa1824195734d22ce15ee525deba2a48f5bbcc5824ee86646c24cee72011e97),
    // where repeated journal paragraphs open italics on one quoted line and
    // close them on the next. Twelve pairs exceeded the replay's five-second
    // parse budget before the quote cursor was carried into inline collectors.
    for (marker, tag) in [
        ("**", "strong"),
        ("//", "em"),
        ("__", "u"),
        ("^^", "sup"),
        (",,", "sub"),
    ] {
        let input = format!("> {marker}\n> quoted text{marker}\n").repeat(PAIR_COUNT);
        let page_info = page_info();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let started = Instant::now();

        let tokenization = ftml::tokenize(&input);
        let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(started.elapsed() < Duration::from_secs(5), "{marker}");
        assert!(errors.is_empty(), "{marker}: {errors:#?}");
        assert_eq!(html.matches(&format!("<{tag}>")).count(), PAIR_COUNT);
        assert_eq!(html.matches("quoted text").count(), PAIR_COUNT);
    }
}
