use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::time::{Duration, Instant};

fn render(source: &str) -> String {
    let page_info = PageInfo {
        page: Cow::Borrowed("line-owner"),
        category: None,
        site: Cow::Borrowed("syntax-differential"),
        title: Cow::Borrowed("List line ownership"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    };
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokenization = ftml::tokenize(&source);
    let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    assert!(errors.is_empty(), "{source:?}: {errors:#?}");
    HtmlRender.render(&tree, &page_info, &settings).body
}

#[test]
fn wikidot_root_list_ownership_matrix_stays_fixed() {
    for (case_id, source, expected) in [
        (
            "scout-line-owner-root-ul-none",
            "* A",
            "<ul>\n<li>A</li>\n</ul>",
        ),
        (
            "scout-line-owner-root-ul-none-then-root",
            "* A\n* B",
            "<ul>\n<li>A</li>\n<li>B</li>\n</ul>",
        ),
        (
            "scout-line-owner-root-ul-root-then-none",
            "* A\n* B",
            "<ul>\n<li>A</li>\n<li>B</li>\n</ul>",
        ),
        ("scout-line-owner-root-ul-space1", " * A", "<p>* A</p>"),
        (
            "scout-line-owner-root-ul-space1-then-root",
            " * A\n* B",
            "<p>* A</p><ul>\n<li>B</li>\n</ul>",
        ),
        (
            "scout-line-owner-root-ul-root-then-space1",
            "* A\n * B",
            "<ul>\n<li>A\n<ul>\n<li>B</li>\n</ul>\n</li>\n</ul>",
        ),
        ("scout-line-owner-root-ul-space2", "  * A", "<p>* A</p>"),
        (
            "scout-line-owner-root-ul-space2-then-root",
            "  * A\n* B",
            "<p>* A</p><ul>\n<li>B</li>\n</ul>",
        ),
        (
            "scout-line-owner-root-ul-root-then-space2",
            "* A\n  * B",
            "<ul>\n<li>A\n<ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li>B</li>\n</ul></li></ul>\n</li>\n</ul>",
        ),
        ("scout-line-owner-root-ul-space3", "   * A", "<p>* A</p>"),
        (
            "scout-line-owner-root-ul-space3-then-root",
            "   * A\n* B",
            "<p>* A</p><ul>\n<li>B</li>\n</ul>",
        ),
        (
            "scout-line-owner-root-ul-root-then-space3",
            "* A\n   * B",
            "<ul>\n<li>A\n<ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li>B</li>\n</ul></li></ul></li></ul>\n</li>\n</ul>",
        ),
        ("scout-line-owner-root-ul-tab1", "\t* A", "<p>* A</p>"),
        (
            "scout-line-owner-root-ul-tab1-then-root",
            "\t* A\n* B",
            "<p>* A</p><ul>\n<li>B</li>\n</ul>",
        ),
        (
            "scout-line-owner-root-ul-root-then-tab1",
            "* A\n\t* B",
            "<ul>\n<li>A\n<ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li>B</li>\n</ul></li></ul></li></ul></li></ul>\n</li>\n</ul>",
        ),
        ("scout-line-owner-root-ul-tab2", "\t\t* A", "<p>* A</p>"),
        (
            "scout-line-owner-root-ul-tab2-then-root",
            "\t\t* A\n* B",
            "<p>* A</p><ul>\n<li>B</li>\n</ul>",
        ),
        (
            "scout-line-owner-root-ul-root-then-tab2",
            "* A\n\t\t* B",
            "<ul>\n<li>A\n<ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li>B</li>\n</ul></li></ul></li></ul></li></ul></li></ul></li></ul></li></ul></li></ul>\n</li>\n</ul>",
        ),
        ("scout-line-owner-root-ul-space-tab", " \t* A", "<p>* A</p>"),
        (
            "scout-line-owner-root-ul-space-tab-then-root",
            " \t* A\n* B",
            "<p>* A</p><ul>\n<li>B</li>\n</ul>",
        ),
        (
            "scout-line-owner-root-ul-root-then-space-tab",
            "* A\n \t* B",
            "<ul>\n<li>A\n<ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li>B</li>\n</ul></li></ul></li></ul></li></ul></li></ul>\n</li>\n</ul>",
        ),
        ("scout-line-owner-root-ul-tab-space", "\t * A", "<p>* A</p>"),
        (
            "scout-line-owner-root-ul-tab-space-then-root",
            "\t * A\n* B",
            "<p>* A</p><ul>\n<li>B</li>\n</ul>",
        ),
        (
            "scout-line-owner-root-ul-root-then-tab-space",
            "* A\n\t * B",
            "<ul>\n<li>A\n<ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li style=\"list-style: none; display: inline\"><ul>\n<li>B</li>\n</ul></li></ul></li></ul></li></ul></li></ul>\n</li>\n</ul>",
        ),
        (
            "scout-line-owner-root-ol-none",
            "# A",
            "<ol>\n<li>A</li>\n</ol>",
        ),
        (
            "scout-line-owner-root-ol-none-then-root",
            "# A\n# B",
            "<ol>\n<li>A</li>\n<li>B</li>\n</ol>",
        ),
        (
            "scout-line-owner-root-ol-root-then-none",
            "# A\n# B",
            "<ol>\n<li>A</li>\n<li>B</li>\n</ol>",
        ),
        ("scout-line-owner-root-ol-space1", " # A", "<p># A</p>"),
        (
            "scout-line-owner-root-ol-space1-then-root",
            " # A\n# B",
            "<p># A</p><ol>\n<li>B</li>\n</ol>",
        ),
        (
            "scout-line-owner-root-ol-root-then-space1",
            "# A\n # B",
            "<ol>\n<li>A\n<ol>\n<li>B</li>\n</ol>\n</li>\n</ol>",
        ),
        ("scout-line-owner-root-ol-space2", "  # A", "<p># A</p>"),
        (
            "scout-line-owner-root-ol-space2-then-root",
            "  # A\n# B",
            "<p># A</p><ol>\n<li>B</li>\n</ol>",
        ),
        (
            "scout-line-owner-root-ol-root-then-space2",
            "# A\n  # B",
            "<ol>\n<li>A\n<ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li>B</li>\n</ol></li></ol>\n</li>\n</ol>",
        ),
        ("scout-line-owner-root-ol-space3", "   # A", "<p># A</p>"),
        (
            "scout-line-owner-root-ol-space3-then-root",
            "   # A\n# B",
            "<p># A</p><ol>\n<li>B</li>\n</ol>",
        ),
        (
            "scout-line-owner-root-ol-root-then-space3",
            "# A\n   # B",
            "<ol>\n<li>A\n<ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li>B</li>\n</ol></li></ol></li></ol>\n</li>\n</ol>",
        ),
        ("scout-line-owner-root-ol-tab1", "\t# A", "<p># A</p>"),
        (
            "scout-line-owner-root-ol-tab1-then-root",
            "\t# A\n# B",
            "<p># A</p><ol>\n<li>B</li>\n</ol>",
        ),
        (
            "scout-line-owner-root-ol-root-then-tab1",
            "# A\n\t# B",
            "<ol>\n<li>A\n<ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li>B</li>\n</ol></li></ol></li></ol></li></ol>\n</li>\n</ol>",
        ),
        ("scout-line-owner-root-ol-tab2", "\t\t# A", "<p># A</p>"),
        (
            "scout-line-owner-root-ol-tab2-then-root",
            "\t\t# A\n# B",
            "<p># A</p><ol>\n<li>B</li>\n</ol>",
        ),
        (
            "scout-line-owner-root-ol-root-then-tab2",
            "# A\n\t\t# B",
            "<ol>\n<li>A\n<ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li>B</li>\n</ol></li></ol></li></ol></li></ol></li></ol></li></ol></li></ol></li></ol>\n</li>\n</ol>",
        ),
        ("scout-line-owner-root-ol-space-tab", " \t# A", "<p># A</p>"),
        (
            "scout-line-owner-root-ol-space-tab-then-root",
            " \t# A\n# B",
            "<p># A</p><ol>\n<li>B</li>\n</ol>",
        ),
        (
            "scout-line-owner-root-ol-root-then-space-tab",
            "# A\n \t# B",
            "<ol>\n<li>A\n<ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li>B</li>\n</ol></li></ol></li></ol></li></ol></li></ol>\n</li>\n</ol>",
        ),
        ("scout-line-owner-root-ol-tab-space", "\t # A", "<p># A</p>"),
        (
            "scout-line-owner-root-ol-tab-space-then-root",
            "\t # A\n# B",
            "<p># A</p><ol>\n<li>B</li>\n</ol>",
        ),
        (
            "scout-line-owner-root-ol-root-then-tab-space",
            "# A\n\t # B",
            "<ol>\n<li>A\n<ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li style=\"list-style: none; display: inline\"><ol>\n<li>B</li>\n</ol></li></ol></li></ol></li></ol></li></ol>\n</li>\n</ol>",
        ),
    ] {
        assert_eq!(render(source), expected, "{case_id}: {source:?}");
    }
}

#[test]
fn indented_document_markers_remain_literal_before_a_root_run() {
    assert_eq!(
        render(" 	* literal\n* root"),
        "<p>* literal</p><ul>\n<li>root</li>\n</ul>",
    );
    assert_eq!(
        render("\t # literal\n# root"),
        "<p># literal</p><ol>\n<li>root</li>\n</ol>",
    );
}

#[test]
fn tabs_count_as_four_columns_only_for_native_list_indentation() {
    let one_tab = render("* root\n\t* child");
    assert_eq!(
        one_tab.matches("list-style: none; display: inline").count(),
        3
    );
    assert!(one_tab.contains("<li>child</li>"));

    let two_tabs = render("* root\n\t\t* child");
    assert_eq!(
        two_tabs
            .matches("list-style: none; display: inline")
            .count(),
        7
    );
    assert!(two_tabs.contains("<li>child</li>"));

    let code = render("[[code]]\n\t* literal\n[[/code]]");
    assert_eq!(
        code,
        "<div class=\"code\"><pre><code>    * literal</code></pre></div>",
    );
}

#[test]
fn repeated_tab_indented_list_runs_stay_bounded() {
    let source = "* root\n\t* child\n\n".repeat(2_048);
    let started = Instant::now();
    let html = render(&source);
    let elapsed = started.elapsed();

    assert_eq!(html.matches("<li>root").count(), 2_048);
    assert_eq!(html.matches("<li>child</li>").count(), 2_048);
    assert_eq!(
        html.matches("list-style: none; display: inline").count(),
        6_144,
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "tab-indented list runs took {elapsed:?}",
    );
}
