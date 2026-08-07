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
    // Wikidot accepts /cell as the closer but renders the mismatched hcell as
    // a regular cell. Treating it as an unclosed block made four sibling
    // advanced tables exceed five seconds.
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
    assert_eq!(html.matches("<td").count(), TABLE_COUNT);
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

#[test]
fn crossed_and_extra_cell_closer_runs_stay_bounded() {
    const RESIDUAL_COUNT: usize = 2_048;

    let mut input = String::from("[[table]]\n[[row]]\n[[cell]]A[[/hcell]]\n");
    for index in 0..RESIDUAL_COUNT {
        input.push_str(if index % 2 == 0 {
            "[[/cell]]\n"
        } else {
            "[[/hcell]]\n"
        });
    }
    input.push_str("[[/row]]\n[[/table]]");

    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let started = Instant::now();
    let tokenization = ftml::tokenize(&input);
    let (tree, _) = ftml::parse(&tokenization, &page_info, &settings).into();
    let html = HtmlRender.render(&tree, &page_info, &settings).body;

    assert!(
        started.elapsed() < Duration::from_secs(5),
        "crossed/extra closer run took {:?}",
        started.elapsed(),
    );
    assert_eq!(html.matches("[[/cell]]").count(), RESIDUAL_COUNT / 2);
    assert_eq!(html.matches("[[/hcell]]").count(), RESIDUAL_COUNT / 2);
    assert_eq!(html.matches("<table>").count(), 1);
    assert_eq!(html.matches("<th>").count(), 1);
}

#[test]
fn large_malformed_advanced_table_body_stays_bounded() {
    const BODY_LEN: usize = 262_144;

    let input = format!(
        "[[table]]\n[[row]]\n[[cell]]{}[[/row]]\n[[/table]]",
        "A".repeat(BODY_LEN),
    );
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let started = Instant::now();
    let tokenization = ftml::tokenize(&input);
    let (tree, _) = ftml::parse(&tokenization, &page_info, &settings).into();
    let html = HtmlRender.render(&tree, &page_info, &settings).body;

    assert!(
        started.elapsed() < Duration::from_secs(5),
        "large malformed table took {:?}",
        started.elapsed(),
    );
    assert!(!html.contains("<table>"), "{html}");
    assert!(
        html.contains("[[cell]]"),
        "malformed source was not preserved"
    );
    assert!(html.len() >= BODY_LEN, "large cell body was truncated");
}

#[test]
fn many_paragraphs_and_nested_tables_stay_bounded() {
    const PARAGRAPH_COUNT: usize = 2_048;
    const NESTED_INTERVAL: usize = 64;

    let mut input = String::from("[[table]]\n[[row]]\n[[cell]]\n");
    for index in 0..PARAGRAPH_COUNT {
        input.push_str(&format!("P{index}\n\n"));
        if index % NESTED_INTERVAL == NESTED_INTERVAL - 1 {
            input.push_str(concat!(
                "[[table]]\n[[row]]\n[[cell]]I[[/cell]]\n",
                "[[/row]]\n[[/table]]\n\n",
            ));
        }
    }
    input.push_str("[[/cell]]\n[[/row]]\n[[/table]]");

    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let started = Instant::now();
    let tokenization = ftml::tokenize(&input);
    let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    let html = HtmlRender.render(&tree, &page_info, &settings).body;

    assert!(
        started.elapsed() < Duration::from_secs(5),
        "paragraph/table run took {:?}",
        started.elapsed(),
    );
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(html.matches("<p>").count(), PARAGRAPH_COUNT);
    assert_eq!(
        html.matches("<table>").count(),
        1 + PARAGRAPH_COUNT / NESTED_INTERVAL,
    );
}
