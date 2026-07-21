use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender, text::TextRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("parser-functions"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("coverage"),
        title: Cow::Borrowed("Parser Functions"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![Cow::Borrowed("test")],
        language: Cow::Borrowed("en"),
    }
}

fn render(input: &str) -> (String, String) {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut source = input.to_owned();
    ftml::preprocess(&mut source);
    let tokens = ftml::tokenize(&source);
    let result = ftml::parse(&tokens, &page_info, &settings);
    let (tree, errors) = result.into();
    assert!(errors.is_empty(), "{input:?}: {errors:#?}");
    (
        TextRender.render(&tree, &page_info, &settings),
        HtmlRender.render(&tree, &page_info, &settings).body,
    )
}

#[test]
fn expression_prefix_matrix_matches_saved_wikidot() {
    // Live provenance:
    // ftml-oracle-20260712T211704Z/run-parser-function-prefix.
    let (text, html) = render("OMEGA_ROOT=[[#expr 7*6 ]]\nOMEGA_AFTER");
    assert!(text.contains("OMEGA_ROOT=42"), "{text}");
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    assert!(!html.contains("[[#expr"), "{html}");

    for input in [
        ">[[#expr 7*6 ]]OMEGA_TIGHT_D1\nOMEGA_AFTER",
        ">>[[#expr 7*6 ]]OMEGA_TIGHT_D2\nOMEGA_AFTER",
    ] {
        let (text, html) = render(input);
        assert!(!text.contains("OMEGA_TIGHT"), "{input:?}: {text}");
        assert_eq!(text, "OMEGA_AFTER", "{input:?}: {text}");
        assert!(!html.contains("<blockquote>"), "{input:?}: {html}");
    }

    let (text, html) = render("> [[#expr 7*6 ]] OMEGA_SPACED_D1\nOMEGA_AFTER");
    assert!(text.contains("42 OMEGA_SPACED_D1"), "{text}");
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");

    let (text, html) = render(">> [[#expr 7*6 ]] OMEGA_SPACED_D2\nOMEGA_AFTER");
    assert!(text.contains("42 OMEGA_SPACED_D2"), "{text}");
    assert_eq!(html.matches("<blockquote>").count(), 2, "{html}");

    let (text, html) = render("> > [[#expr 7*6 ]] OMEGA_SPACED_INNER\nOMEGA_AFTER");
    assert!(text.contains("> 42 OMEGA_SPACED_INNER"), "{text}");
    assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");
    assert!(html.contains("&gt; 42 OMEGA_SPACED_INNER"), "{html}");
}

#[test]
fn document_leading_whitespace_uses_saved_page_semantics() {
    // Live provenance:
    // ftml-oracle-20260712T214547Z/run-quote-indentation and
    // ftml-oracle-20260712T215005Z/run-quote-document-leading-whitespace.
    let (text, html) =
        render("\n\t  > [[#expr 7*6 ]] OMEGA_FIRST\n  > OMEGA_SECOND\nOMEGA_AFTER");
    assert!(text.contains("42 OMEGA_FIRST"), "{text}");
    assert!(text.contains("> OMEGA_SECOND"), "{text}");
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");
    assert!(html.contains("&gt; OMEGA_SECOND"), "{html}");

    let (text, html) = render("[!-- comment --]\n  > [[#expr 7*6 ]] OMEGA_AFTER_COMMENT");
    assert!(text.contains("> 42 OMEGA_AFTER_COMMENT"), "{text}");
    assert!(!html.contains("<blockquote>"), "{html}");
}

#[test]
fn ifexpr_prefix_matrix_selects_only_the_live_branch() {
    let (text, html) = render(concat!(
        "> [[#ifexpr 3>2 | OMEGA_TRUE | OMEGA_FALSE ]]\n",
        "> [[#ifexpr 2>3 | OMEGA_HIDDEN | OMEGA_SELECTED ]]\n",
        "OMEGA_AFTER",
    ));
    assert!(text.contains("OMEGA_TRUE"), "{text}");
    assert!(text.contains("OMEGA_SELECTED"), "{text}");
    assert!(!text.contains("OMEGA_FALSE"), "{text}");
    assert!(!text.contains("OMEGA_HIDDEN"), "{text}");
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");

    let (text, html) =
        render(">[[#ifexpr 3>2 | OMEGA_TIGHT_TRUE | OMEGA_TIGHT_FALSE ]]\nOMEGA_AFTER");
    assert_eq!(text, "OMEGA_AFTER", "{text}");
    assert!(!html.contains("OMEGA_TIGHT"), "{html}");
}

#[test]
fn simple_if_prefix_matrix_selects_only_the_live_branch() {
    let (text, html) = render(concat!(
        "[[#if 1 | OMEGA_ROOT_TRUE | OMEGA_ROOT_FALSE ]]\n",
        "[[#if 0 | OMEGA_ZERO_TRUE | OMEGA_ZERO_FALSE ]]\n",
        "> [[#if 1 | OMEGA_QUOTED_TRUE | OMEGA_QUOTED_FALSE ]]\n",
        "OMEGA_AFTER",
    ));
    for selected in ["OMEGA_ROOT_TRUE", "OMEGA_ZERO_FALSE", "OMEGA_QUOTED_TRUE"] {
        assert!(text.contains(selected), "{selected}: {text}");
    }
    for hidden in ["OMEGA_ROOT_FALSE", "OMEGA_ZERO_TRUE", "OMEGA_QUOTED_FALSE"] {
        assert!(!text.contains(hidden), "{hidden}: {text}");
    }
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");

    let (text, html) =
        render(">[[#if 1 | OMEGA_TIGHT_TRUE | OMEGA_TIGHT_FALSE ]]\nOMEGA_AFTER");
    assert_eq!(text, "OMEGA_AFTER", "{text}");
    assert!(!html.contains("OMEGA_TIGHT"), "{html}");
}

#[test]
fn simple_if_string_truthiness_matches_saved_wikidot() {
    // Live provenance:
    // ftml-oracle-20260712T225511Z/run-parser-if-string and
    // ftml-oracle-20260712T225812Z/run-parser-if-include.
    let (text, html) = render(concat!(
        "[[#if aroace | OMEGA_STRING_TRUE | OMEGA_STRING_FALSE ]]\n",
        "[[#if {$code} | OMEGA_PLACEHOLDER_TRUE | OMEGA_PLACEHOLDER_FALSE ]]\n",
        "[[#if  | OMEGA_EMPTY_TRUE | OMEGA_EMPTY_FALSE ]]\n",
        "OMEGA_AFTER",
    ));

    for selected in [
        "OMEGA_STRING_TRUE",
        "OMEGA_PLACEHOLDER_TRUE",
        "OMEGA_EMPTY_FALSE",
        "OMEGA_AFTER",
    ] {
        assert!(text.contains(selected), "{selected}: {text}");
    }
    for hidden in [
        "OMEGA_STRING_FALSE",
        "OMEGA_PLACEHOLDER_FALSE",
        "OMEGA_EMPTY_TRUE",
    ] {
        assert!(!text.contains(hidden), "{hidden}: {text}");
    }
    assert!(!html.contains("[[#if"), "{html}");
}

#[test]
fn parser_functions_generate_comment_delimiters_before_comment_parsing() {
    // Live provenance:
    // ftml-oracle-20260712T230555Z/run-parser-comment-delimiter.
    let (text, html) = render(concat!(
        "[!-- [[#if aroace | --] |  ]]OMEGA_TRUE[!-- --]\n",
        "[!-- [[#if 0 | --] |  ]]OMEGA_FALSE[!-- --]\n",
        "[!-- [[#expr 1+1]] OMEGA_COMMENT --]\n",
        "OMEGA_AFTER",
    ));

    assert!(text.contains("OMEGA_TRUE"), "{text}");
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    for hidden in ["OMEGA_FALSE", "OMEGA_COMMENT", "[[#expr", "2 OMEGA_COMMENT"] {
        assert!(!text.contains(hidden), "{hidden}: {text}");
        assert!(!html.contains(hidden), "{hidden}: {html}");
    }
}

#[test]
fn literal_and_runtime_error_outputs_survive_the_full_pipeline() {
    let (text, html) = render(concat!(
        "> @@OMEGA_RAW [[#ifexpr 3>2 | OMEGA_RAW_TRUE | OMEGA_RAW_FALSE ]]@@\n",
        "> OMEGA_BAD=[[#expr unknown(1) ]]\n",
        "> OMEGA_DIV=[[#expr 1/0 ]]\n",
        "> OMEGA_MOD=[[#expr 1%0 ]]\n",
        "OMEGA_AFTER",
    ));
    assert!(
        text.contains("[[#ifexpr 3>2 | OMEGA_RAW_TRUE | OMEGA_RAW_FALSE ]]"),
        "{text}",
    );
    assert!(
        text.contains(r#"OMEGA_BAD=run-time error: undefined function "unknown""#),
        "{text}",
    );
    assert!(
        text.contains("OMEGA_DIV=run-time error: division by zero"),
        "{text}",
    );
    assert!(
        text.contains("OMEGA_MOD=run-time error: rest-division by zero"),
        "{text}",
    );
    assert!(text.contains("OMEGA_AFTER"), "{text}");
    assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");
}
