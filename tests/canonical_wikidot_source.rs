use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::Render;
use ftml::render::html::HtmlRender;
use ftml::render::text::TextRender;
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::time::{Duration, Instant};

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("canonical-source"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("coverage"),
        title: Cow::Borrowed("Canonical Source"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![Cow::Borrowed("test")],
        language: Cow::Borrowed("en"),
    }
}

fn render_text_and_html(input: &str) -> (String, String) {
    render_text_and_html_with_layout(input, Layout::Wikidot)
}

fn render_text_and_html_with_layout(input: &str, layout: Layout) -> (String, String) {
    render_text_and_html_with_layout_and_errors(input, layout, false)
}

fn render_text_and_html_with_layout_and_errors(
    input: &str,
    layout: Layout,
    allow_literal_marker_errors: bool,
) -> (String, String) {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
    let mut text = input.to_owned();
    ftml::preprocess(&mut text);
    let tokens = ftml::tokenize(&text);
    let result = ftml::parse(&tokens, &page_info, &settings);
    let (tree, errors) = result.into();

    assert!(
        allow_literal_marker_errors || errors.is_empty(),
        "{input:?}: {errors:?}"
    );

    let text_output = TextRender.render(&tree, &page_info, &settings);
    let html_output = HtmlRender.render(&tree, &page_info, &settings).body;
    (text_output, html_output)
}

#[test]
fn wikidot_saved_pages_strip_only_document_leading_ascii_whitespace() {
    // Live sandbox provenance:
    // ftml-oracle-20260712T214547Z/run-quote-indentation and
    // ftml-oracle-20260712T215005Z/run-quote-document-leading-whitespace.
    let (text, html) =
        render_text_and_html("\n\t  > OMEGA_FIRST\n  > OMEGA_SECOND\nOMEGA_AFTER");

    assert!(text.contains("OMEGA_FIRST"), "{text}");
    assert!(text.contains("> OMEGA_SECOND"), "{text}");
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");
    assert!(html.contains("&gt; OMEGA_SECOND"), "{html}");

    let (text, html) = render_text_and_html("[!-- comment --]\n  > OMEGA_AFTER_COMMENT");
    assert!(text.contains("> OMEGA_AFTER_COMMENT"), "{text}");
    assert!(!html.contains("<blockquote>"), "{html}");
}

#[test]
fn wikidot_closed_quote_prefixed_iftags_evaluate_before_native_quotes() {
    // Live sandbox provenance: run-quoted-conditionals, 2026-07-13.
    for (input, body, included) in [
        (
            "> [[iftags -codex-never]]\n> OMEGA_D1_TRUE\n> [[/iftags]]\nOMEGA_AFTER",
            "OMEGA_D1_TRUE",
            true,
        ),
        (
            "> [[iftags +codex-never]]\n> OMEGA_D1_FALSE\n> [[/iftags]]\nOMEGA_AFTER",
            "OMEGA_D1_FALSE",
            false,
        ),
        (
            ">> [[iftags +codex-never]]\n>> OMEGA_D2_FALSE\n>> [[/iftags]]\nOMEGA_AFTER",
            "OMEGA_D2_FALSE",
            false,
        ),
        (
            ">[[iftags +codex-never]]\nOMEGA_TIGHT_FALSE\n>[[/iftags]]\nOMEGA_AFTER",
            "OMEGA_TIGHT_FALSE",
            false,
        ),
        (
            ">[[iftags -codex-never]]\nOMEGA_TIGHT_TRUE\n>[[/iftags]]\nOMEGA_AFTER",
            "OMEGA_TIGHT_TRUE",
            true,
        ),
        (
            ">[[iftags]]\nOMEGA_TIGHT_NOARG\n>[[/iftags]]\nOMEGA_AFTER",
            "OMEGA_TIGHT_NOARG",
            false,
        ),
        (
            "[[iftags]]\nOMEGA_ROOT_NOARG\n[[/iftags]]\nOMEGA_AFTER",
            "OMEGA_ROOT_NOARG",
            false,
        ),
        (
            ">> [[iftags +codex-never]]\n>> OMEGA_SHALLOW_FALSE\n> [[/iftags]]\nOMEGA_AFTER",
            "OMEGA_SHALLOW_FALSE",
            false,
        ),
    ] {
        let (text, html) = render_text_and_html(input);

        assert_eq!(text.contains(body), included, "{input:?}: {text}");
        assert!(text.contains("OMEGA_AFTER"), "{input:?}: {text}");
        assert!(!html.contains("[[iftags"), "{input:?}: {html}");
        assert!(!html.contains("[[/iftags"), "{input:?}: {html}");
        if included && !input.starts_with(">[[") {
            assert!(html.contains("<blockquote>"), "{input:?}: {html}");
        }
    }
}

#[test]
fn wikidot_false_quoted_iftags_does_not_leave_an_empty_quote_row() {
    let (text, html) = render_text_and_html(
        "> [[iftags +codex-never]]\n> OMEGA_HIDDEN\n> [[/iftags]]\nOMEGA_AFTER",
    );

    assert_eq!(text, "OMEGA_AFTER", "{text}");
    assert!(!html.contains("<blockquote>"), "{html}");
    assert!(!html.contains("OMEGA_HIDDEN"), "{html}");
    assert!(!html.contains("[[iftags"), "{html}");
}

#[test]
fn wikidot_false_quoted_iftags_between_visible_rows_adds_no_blank_row() {
    let (text, html) = render_text_and_html(concat!(
        "> OMEGA_BEFORE\n",
        "> [[iftags +codex-never]]\n",
        "> OMEGA_HIDDEN\n",
        "> [[/iftags]]\n",
        "> OMEGA_AFTER",
    ));

    assert!(text.contains("OMEGA_BEFORE"), "{text}");
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    assert!(!text.contains("OMEGA_HIDDEN"), "{text}");
    assert!(html.contains("OMEGA_BEFORE<br>OMEGA_AFTER"), "{html}");
    assert!(!html.contains("<br><br>"), "{html}");
}

#[test]
fn wikidot_empty_native_quote_lines_do_not_create_empty_blockquotes() {
    // Live sandbox provenance:
    // ftml-oracle-20260713T112423Z/run-iftags-quoted-partial/empty-quote-controls.
    for empty_quote in [">", "> ", ">> "] {
        let input = format!("OMEGA_BEGIN\n{empty_quote}\nOMEGA_AFTER");
        let (text, html) = render_text_and_html(&input);

        assert!(text.contains("OMEGA_BEGIN"), "{empty_quote:?}: {text}");
        assert!(text.contains("OMEGA_AFTER"), "{empty_quote:?}: {text}");
        assert_eq!(
            html.matches("<blockquote>").count(),
            0,
            "{empty_quote:?}: {html}"
        );
    }
}

#[test]
fn wikidot_unquoted_blank_line_starts_a_sibling_native_quote_run() {
    // Live sandbox provenance:
    // ftml-oracle-20260713T143737Z/run-include-expansion-boundary-separators/
    // direct-unquoted-blank-quote-control.
    let (text, html) = render_text_and_html("> OMEGA_QUOTE_A\n\n> OMEGA_QUOTE_B");

    assert!(text.contains("OMEGA_QUOTE_A"), "{text}");
    assert!(text.contains("OMEGA_QUOTE_B"), "{text}");
    assert_eq!(html.matches("<blockquote>").count(), 2, "{html}");
    assert_eq!(html.matches("</blockquote>").count(), 2, "{html}");
    assert!(!html.contains("OMEGA_QUOTE_A<br>OMEGA_QUOTE_B"), "{html}");
}

#[test]
fn wikidot_unquoted_blank_line_starts_a_sibling_native_list_run() {
    // Live sandbox provenance:
    // ftml-oracle-20260713T154248Z/run-include-structural-boundaries.
    let (text, html) = render_text_and_html("* OMEGA_LIST_A\n\n* OMEGA_LIST_B");

    assert!(text.contains("OMEGA_LIST_A"), "{text}");
    assert!(text.contains("OMEGA_LIST_B"), "{text}");
    assert_eq!(html.matches("<ul>").count(), 2, "{html}");
    assert_eq!(html.matches("</ul>").count(), 2, "{html}");
}

#[test]
fn repeated_sibling_native_list_runs_stay_bounded() {
    let source = (0..2_048)
        .map(|index| format!("* OMEGA_LIST_{index}\n\n"))
        .collect::<String>();
    let started = Instant::now();
    let (_, html) = render_text_and_html(&source);

    assert_eq!(html.matches("<ul>").count(), 2_048);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn wikidot_empty_native_quote_lines_preserve_surrounding_run_semantics() {
    // Live sandbox provenance:
    // ftml-oracle-20260713T112423Z/run-iftags-quoted-partial/empty-quote-run-controls.
    for (input, depth, separated_paragraphs) in [
        ("> OMEGA_A\n> \n> OMEGA_B", 1, true),
        ("> OMEGA_A\n>\n> OMEGA_B", 1, false),
        (">> OMEGA_A\n>>\n>> OMEGA_B", 2, false),
        ("> OMEGA_A\n>>\n> OMEGA_B", 1, false),
    ] {
        let (text, html) = render_text_and_html(input);

        assert!(text.contains("OMEGA_A"), "{input:?}: {text}");
        assert!(text.contains("OMEGA_B"), "{input:?}: {text}");
        assert_eq!(
            html.matches("<blockquote>").count(),
            depth,
            "{input:?}: {html}"
        );
        assert_eq!(
            html.matches("<p>").count(),
            if separated_paragraphs { 2 } else { 1 },
            "{input:?}: {html}"
        );
        assert_eq!(
            html.contains("OMEGA_A<br>OMEGA_B"),
            !separated_paragraphs,
            "{input:?}: {html}"
        );
    }
}

#[test]
fn wikidot_discarded_tight_quote_row_does_not_split_the_active_paragraph() {
    // Live sandbox provenance: discarded-tight-middle in
    // ftml-oracle-20260713T112423Z/run-iftags-quoted-partial/empty-quote-run-controls.
    let (text, html) = render_text_and_html("> OMEGA_A\n>OMEGA_DROP\n> OMEGA_B");

    assert!(text.contains("OMEGA_A"), "{text}");
    assert!(text.contains("OMEGA_B"), "{text}");
    assert!(!text.contains("OMEGA_DROP"), "{text}");
    assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");
    assert_eq!(html.matches("<p>").count(), 1, "{html}");
    assert!(html.contains("OMEGA_A<br>OMEGA_B"), "{html}");
}

#[test]
fn wikidot_spaced_inner_iftags_preserves_the_residual_quote_marker() {
    // Live sandbox provenance: iftags-spaced-inner-false.
    let (text, html) = render_text_and_html(
        "> > [[iftags +codex-never]]\n> > OMEGA_SPACED_FALSE\n> > [[/iftags]]\nOMEGA_AFTER",
    );

    assert!(!text.contains("OMEGA_SPACED_FALSE"), "{text}");
    assert!(text.contains('>'), "{text}");
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");
    assert_eq!(html.matches("&gt;").count(), 1, "{html}");
    assert!(html.contains("<p>&gt;</p>"), "{html}");
}

#[test]
fn wikidot_nested_tight_iftags_pair_without_crossing() {
    // Frozen theme sources use adjacent tight gates; pairing must remain LIFO.
    let (text, html) = render_text_and_html(concat!(
        ">[[iftags -codex-never]]\n",
        ">[[iftags]]\n",
        "OMEGA_NESTED_TIGHT_BODY\n",
        ">[[/iftags]]\n",
        ">[[/iftags]]\n",
        "OMEGA_NESTED_TIGHT_AFTER",
    ));

    assert!(!text.contains("OMEGA_NESTED_TIGHT_BODY"), "{text}");
    assert!(text.contains("OMEGA_NESTED_TIGHT_AFTER"), "{text}");
    assert!(!html.contains("[[iftags"), "{html}");
    assert!(!html.contains("[[/iftags"), "{html}");
}

#[test]
fn wikidot_true_spaced_inner_iftags_preserves_all_three_residual_quote_markers() {
    // Live sandbox provenance: run-iftags-spaced-inner-true, 2026-07-13.
    let (text, html) = render_text_and_html(
        "> > [[iftags -codex-never]]\n> > OMEGA_SPACED_TRUE\n> > [[/iftags]]\nOMEGA_AFTER",
    );

    assert!(text.contains("OMEGA_SPACED_TRUE"), "{text}");
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    assert!(!html.contains("[[iftags"), "{html}");
    assert!(!html.contains("[[/iftags"), "{html}");
    assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");
    assert_eq!(html.matches("&gt;").count(), 3, "{html}");
    assert!(
        html.contains("&gt;<br>&gt; OMEGA_SPACED_TRUE<br>&gt;</p>"),
        "{html}"
    );
}

#[test]
fn wikidot_unclosed_quoted_iftags_opener_remains_literal() {
    // Live sandbox provenance: iftags-unclosed-false.
    let (text, html) = render_text_and_html_with_layout_and_errors(
        "> [[iftags +codex-never]]\n> OMEGA_UNCLOSED_BODY\nOMEGA_AFTER",
        Layout::Wikidot,
        true,
    );

    assert!(text.contains("[[iftags +codex-never]]"), "{text}");
    assert!(text.contains("OMEGA_UNCLOSED_BODY"), "{text}");
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    assert!(html.contains("<blockquote>"), "{html}");
}

#[test]
fn wikidot_unclosed_tight_iftags_line_is_consumed_without_gating() {
    // Live sandbox provenance: unclosed tight opener control.
    let (text, html) = render_text_and_html(
        ">[[iftags +codex-never]]\nOMEGA_TIGHT_UNCLOSED_BODY\nOMEGA_AFTER",
    );

    assert!(!text.contains("[[iftags"), "{text}");
    assert!(text.contains("OMEGA_TIGHT_UNCLOSED_BODY"), "{text}");
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    assert!(!html.contains("[[iftags"), "{html}");
}

#[test]
fn ifcategory_is_wikidot_literal_but_remains_a_wikijump_conditional() {
    // Live sandbox provenance: ifcategory unquoted/depth-one cases.
    for (condition, wikijump_includes_body) in [("test", true), ("other", false)] {
        let input = format!(
            "[[ifcategory {condition}]]\nOMEGA_CATEGORY_BODY\n[[/ifcategory]]\nOMEGA_AFTER"
        );
        let (wikidot_text, wikidot_html) =
            render_text_and_html_with_layout_and_errors(&input, Layout::Wikidot, true);
        let (wikijump_text, wikijump_html) =
            render_text_and_html_with_layout(&input, Layout::Wikijump);

        assert!(wikidot_text.contains("[[ifcategory"), "{wikidot_text}");
        assert!(wikidot_text.contains("[[/ifcategory]]"), "{wikidot_text}");
        assert!(
            wikidot_text.contains("OMEGA_CATEGORY_BODY"),
            "{wikidot_text}"
        );
        assert!(wikidot_html.contains("[[ifcategory"), "{wikidot_html}");

        assert!(!wikijump_text.contains("[[ifcategory"), "{wikijump_text}");
        assert!(!wikijump_text.contains("[[/ifcategory"), "{wikijump_text}");
        assert_eq!(
            wikijump_text.contains("OMEGA_CATEGORY_BODY"),
            wikijump_includes_body,
            "{wikijump_text}"
        );
        assert!(!wikijump_html.contains("[[ifcategory"), "{wikijump_html}");
    }
}

#[test]
fn wikidot_quote_prefixed_ifcategory_stays_literal_across_closer_depths() {
    // Live sandbox provenance: ifcategory depth-one and deeper-close cases.
    for input in [
        "> [[ifcategory test]]\n> OMEGA_CATEGORY_D1\n> [[/ifcategory]]\nOMEGA_AFTER",
        "> [[ifcategory test]]\n> OMEGA_CATEGORY_DEEP\n>> [[/ifcategory]]\nOMEGA_AFTER",
    ] {
        let (text, html) =
            render_text_and_html_with_layout_and_errors(input, Layout::Wikidot, true);

        assert!(text.contains("[[ifcategory test]]"), "{input:?}: {text}");
        assert!(text.contains("[[/ifcategory]]"), "{input:?}: {text}");
        assert!(text.contains("OMEGA_CATEGORY_"), "{input:?}: {text}");
        assert!(text.contains("OMEGA_AFTER"), "{input:?}: {text}");
        assert!(html.contains("<blockquote>"), "{input:?}: {html}");
    }
}

#[test]
fn wikidot_content_section_marker_lines_do_not_render() {
    let (text, html) = render_text_and_html("before\n====\nmiddle\n=====\nafter");

    assert!(text.contains("before"));
    assert!(text.contains("middle"));
    assert!(text.contains("after"));
    assert!(!text.contains("===="), "{text}");
    assert!(!html.contains("===="), "{html}");
}

#[test]
fn wikidot_section_marker_rule_preserves_existing_equal_and_literal_contexts() {
    let (text, html) = render_text_and_html(
        "= centered\n++ Heading\n[[code]]\n====\n[[/code]]\n@@====@@",
    );

    assert!(text.contains("centered"));
    assert!(text.contains("Heading"));
    assert!(text.contains("===="), "{text}");
    assert!(html.contains("text-align: center"), "{html}");
    assert!(html.contains("<h2"), "{html}");
    assert!(html.contains("===="), "{html}");
}

#[test]
fn wikidot_canonical_unclosed_block_markers_do_not_render_as_text() {
    let (text, html) = render_text_and_html_with_layout_and_errors(
        r#"[[iftags +test]]
[[div_ class="authorlink-wrapper"]]
Calibold"#,
        Layout::Wikidot,
        true,
    );

    assert!(text.contains("[[iftags +test]]"), "{text}");
    assert!(text.contains("Calibold"), "{text}");
    assert!(html.contains("[[iftags +test]]"), "{html}");
    assert!(!html.contains("[[div_"), "{html}");
    assert!(
        html.contains(r#"<div class="authorlink-wrapper">Calibold</div>"#),
        "{html}"
    );
}

#[test]
fn wikidot_unclosed_false_iftags_marker_is_literal() {
    let (text, html) = render_text_and_html_with_layout_and_errors(
        r#"[[iftags +theme]]
Article body survives."#,
        Layout::Wikidot,
        true,
    );

    assert!(text.contains("[[iftags +theme]]"), "{text}");
    assert!(text.contains("Article body survives."), "{text}");
    assert!(html.contains("Article body survives."), "{html}");
    assert!(html.contains("[[iftags +theme]]"), "{html}");
}

#[test]
fn wikidot_unclosed_true_iftags_marker_is_literal() {
    let (text, html) = render_text_and_html_with_layout_and_errors(
        r#"[[iftags +test]]
Article body survives."#,
        Layout::Wikidot,
        true,
    );

    assert!(text.contains("[[iftags +test]]"), "{text}");
    assert!(text.contains("Article body survives."), "{text}");
    assert!(html.contains("Article body survives."), "{html}");
    assert!(html.contains("[[iftags +test]]"), "{html}");
}

#[test]
fn wikidot_unclosed_false_ifcategory_marker_is_literal() {
    let (text, html) = render_text_and_html_with_layout_and_errors(
        r#"[[ifcategory +other]]
Article body survives."#,
        Layout::Wikidot,
        true,
    );

    assert!(text.contains("[[ifcategory +other]]"), "{text}");
    assert!(text.contains("Article body survives."), "{text}");
    assert!(html.contains("Article body survives."), "{html}");
    assert!(html.contains("[[ifcategory +other]]"), "{html}");
}

#[test]
fn wikidot_unclosed_true_ifcategory_marker_is_literal() {
    let (text, html) = render_text_and_html_with_layout_and_errors(
        r#"[[ifcategory +test]]
Article body survives."#,
        Layout::Wikidot,
        true,
    );

    assert!(text.contains("[[ifcategory +test]]"), "{text}");
    assert!(text.contains("Article body survives."), "{text}");
    assert!(html.contains("Article body survives."), "{html}");
    assert!(html.contains("[[ifcategory +test]]"), "{html}");
}

#[test]
fn wikidot_licensebox_collapsible_expanded_source_renders() {
    let (text, html) = render_text_and_html(
        r#"[[div class="licensebox"]]
[[collapsible show="‡ Licensing / Citation" hide="‡ Hide Licensing / Citation"]]
Cite this page as:
[[div class="list-pages-box"]]
[[div class="list-pages-item"]]
> "SCP-2117" by Administrator.
[[/div]]
[[/div]]
For information on licensing, see the guide.
=====
> **Filename:** 2117.png
> **Author:** Cyantreuse
> **License:** CC BY-SA 3.0
> **Source Link:** [[[http://scp-wiki.wikidot.com/scp-2117/|SCP Wiki]]]
> **Derivative of:**
> ------
> **Author:** Dr Reach
> **License:** CC BY-SA 3.0
> **Source Link:** [[[http://scp-wiki.wikidot.com/scp-2117/|SCP Wiki]]]
=====
[[/collapsible]]
[[/div]]"#,
    );

    assert!(text.contains("Filename:"), "{text}");
    assert!(html.contains(r#"<div class="collapsible-block""#), "{html}");
    assert!(!html.contains("[[collapsible"), "{html}");
    assert!(!html.contains("[[/collapsible]]"), "{html}");
}
