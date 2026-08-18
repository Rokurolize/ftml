use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::time::{Duration, Instant};

fn render(source: &str) -> String {
    let page_info = PageInfo {
        page: Cow::Borrowed("syntax-differential"),
        category: None,
        site: Cow::Borrowed("syntax-differential"),
        title: Cow::Borrowed("Lexical ownership"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    };
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokens = ftml::tokenize(&source);
    let (tree, errors) = ftml::parse(&tokens, &page_info, &settings).into();
    assert!(errors.is_empty(), "{source:?}: {errors:#?}");
    HtmlRender.render(&tree, &page_info, &settings).body
}

#[test]
fn definition_list_horizontal_rule_and_underscore_residuals_match_wikidot() {
    assert_eq!(render(":  : Value"), "<dl>\n<dd>Value</dd>\n</dl>\n");
    assert_eq!(render("---- "), "<p>——</p>");
    assert_eq!(render("----\n----"), "<hr>\n<hr>");
    assert_eq!(
        render("+ A _\nB"),
        "<h1 id=\"toc0\"><span>A _</span></h1><p>B</p>",
    );
    assert_eq!(render("A _\n\nB"), "<p>A<br>\n<br>\nB</p>");
}

#[test]
fn double_angle_typography_respects_literal_link_and_url_owners() {
    assert_eq!(render("<<A>>"), "<p>«A»</p>");
    assert_eq!(
        render("[[code]]\n<<A>>\n[[/code]]"),
        "<div class=\"code\"><pre><code>&lt;&lt;A&gt;&gt;</code></pre></div>",
    );
    assert!(
        render("[[math]]\n<<A>>\n[[/math]]")
            .contains(r#"\begin{equation} &lt;&lt;A&gt;&gt; \end{equation}"#),
    );
    assert_eq!(
        render("@@<<A>>@@"),
        "<p>@<span style=\"white-space: pre-wrap;\">&lt;A&gt;</span>@</p>",
    );
    assert_eq!(
        render("[[[scp-002|<<A>>]]]"),
        "<p><a href=\"/scp-002\">&lt;&lt;A&gt;&gt;</a></p>",
    );
    assert_eq!(
        render("[https://example.com/x <<A>>]"),
        "<p><a href=\"https://example.com/x\">&lt;&lt;A&gt;&gt;</a></p>",
    );
    assert_eq!(
        render("https://example.com/a>>b"),
        "<p><a href=\"https://example.com/a&gt;&gt;b\">https://example.com/a&gt;&gt;b</a></p>",
    );
}

#[test]
fn file_block_preserves_opaque_path_and_label_data() {
    assert!(
        render("[[file path with spaces/elements.tsv]]").contains(concat!(
            "/local--files/path%20with%20spaces/elements.tsv\">",
            "path with spaces/elements.tsv</a>",
        )),
    );
    assert!(
        render("[[file 日本語.txt]]")
            .contains("/%E6%97%A5%E6%9C%AC%E8%AA%9E.txt\">日本語.txt</a>"),
    );
    assert!(
        render("[[file ../elements.tsv | Download]]")
            .contains("/local--files/../elements.tsv\">Download</a>"),
    );
    assert!(
        render("[[file elements.tsv | label | extra]]")
            .ends_with(">label | extra</a></p>"),
    );
    assert!(render("[[file elements.tsv]]]").ends_with(">elements.tsv</a>]</p>"),);
}

#[test]
fn dense_angle_owner_scanning_stays_bounded() {
    let unit = concat!(
        "<<PROSE>> ",
        "[[[scp-002|<<LINK>>]]] ",
        "https://example.com/a>>b ",
        "@@<<RAW>>@@ ",
    );
    let source = unit.repeat(2_048);
    let started = Instant::now();
    let html = render(&source);
    let elapsed = started.elapsed();

    assert_eq!(html.matches('«').count(), 2_048, "{html}");
    assert_eq!(
        html.matches("&lt;&lt;LINK&gt;&gt;").count(),
        2_048,
        "{html}"
    );
    assert_eq!(html.matches("&lt;RAW&gt;").count(), 2_048, "{html}");
    assert!(
        elapsed < Duration::from_secs(5),
        "angle owner scan took {elapsed:?}",
    );
}
