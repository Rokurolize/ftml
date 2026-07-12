use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::time::{Duration, Instant};

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("advanced-table-performance"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("coverage"),
        title: Cow::Borrowed("Advanced table performance"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![Cow::Borrowed("test")],
        language: Cow::Borrowed("en"),
    }
}

#[test]
fn repeated_wikidot_header_cell_closers_stay_bounded() {
    const TABLE_COUNT: usize = 64;

    // Reduced from EN:scp-4354 (source SHA-256
    // ebcf9926f045d2aaa8f73596e5a256d12fe4d0ac3364eb7fb66cc7caa447f169).
    // Wikidot closes hcell blocks with /cell. Treating that closer as a
    // mismatch made four sibling advanced tables exceed five seconds.
    let table = "[[table]]\n[[row]]\n[[hcell]]Heading[[/cell]]\n[[/row]]\n[[/table]]\n";
    let input = table.repeat(TABLE_COUNT);
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let started = Instant::now();

    let tokenization = ftml::tokenize(&input);
    let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    let html = HtmlRender.render(&tree, &page_info, &settings).body;

    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(html.matches("<table").count(), TABLE_COUNT);
    assert_eq!(html.matches("<th").count(), TABLE_COUNT);
}

#[test]
fn quoted_advanced_tables_stay_bounded() {
    const TABLE_COUNT: usize = 16;

    // Reduced from EN:scp-2102 (source SHA-256
    // 474dd6012d711f976fa74eb34ad3f8ae0e4a5d37e00445ba74c09f6432c20f61).
    // Its advanced tables and every nested row/cell live on native blockquote
    // lines. Without the quote-aware block body cursor, three sibling tables
    // exceeded five seconds while the same unquoted table parsed in 19 ms.
    let table = concat!(
        "> [[table]]\n",
        "> [[row]]\n",
        "> [[cell]]Timestamp[[/cell]]\n",
        "> [[cell]]Message[[/cell]]\n",
        "> [[/row]]\n",
        "> [[/table]]\n",
    );
    let input = table.repeat(TABLE_COUNT);
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let started = Instant::now();

    let tokenization = ftml::tokenize(&input);
    let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    let html = HtmlRender.render(&tree, &page_info, &settings).body;

    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(html.matches("<table").count(), TABLE_COUNT);
    assert_eq!(html.matches("<td").count(), TABLE_COUNT * 2);
}
