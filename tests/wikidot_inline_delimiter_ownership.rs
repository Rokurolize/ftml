use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{PageExistenceResolver, Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::time::{Duration, Instant};

fn render(source: &str) -> String {
    let page_info = PageInfo {
        page: Cow::Borrowed("inline-delimiter-ownership"),
        category: None,
        site: Cow::Borrowed("syntax-differential"),
        title: Cow::Borrowed("Inline delimiter ownership"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    };
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokens = ftml::tokenize(&source);
    let (tree, _) = ftml::parse(&tokens, &page_info, &settings).into();
    HtmlRender.render(&tree, &page_info, &settings).body
}

struct MissingPages;

impl PageExistenceResolver for MissingPages {
    fn page_exists(&self, _site: &str, _page: &str) -> bool {
        false
    }
}

fn render_with_missing_pages(source: &str) -> String {
    let page_info = PageInfo {
        page: Cow::Borrowed("inline-delimiter-ownership"),
        category: None,
        site: Cow::Borrowed("syntax-differential"),
        title: Cow::Borrowed("Inline delimiter ownership"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    };
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokens = ftml::tokenize(&source);
    let (tree, _) = ftml::parse(&tokens, &page_info, &settings).into();
    HtmlRender
        .render_with_page_existence(&tree, &page_info, &settings, &MissingPages)
        .body
}

#[derive(Clone, Copy)]
struct FormatCase {
    name: &'static str,
    open: &'static str,
    close: &'static str,
    html_open: &'static str,
    html_close: &'static str,
    span_tag: bool,
}

const FORMATS: &[FormatCase] = &[
    FormatCase {
        name: "bold",
        open: "**",
        close: "**",
        html_open: "<strong>",
        html_close: "</strong>",
        span_tag: false,
    },
    FormatCase {
        name: "italics",
        open: "//",
        close: "//",
        html_open: "<em>",
        html_close: "</em>",
        span_tag: false,
    },
    FormatCase {
        name: "underline",
        open: "__",
        close: "__",
        html_open: r#"<span style="text-decoration: underline;">"#,
        html_close: "</span>",
        span_tag: true,
    },
    FormatCase {
        name: "strike-dash",
        open: "--",
        close: "--",
        html_open: r#"<span style="text-decoration: line-through;">"#,
        html_close: "</span>",
        span_tag: true,
    },
    FormatCase {
        name: "subscript",
        open: ",,",
        close: ",,",
        html_open: "<sub>",
        html_close: "</sub>",
        span_tag: false,
    },
    FormatCase {
        name: "superscript",
        open: "^^",
        close: "^^",
        html_open: "<sup>",
        html_close: "</sup>",
        span_tag: false,
    },
    FormatCase {
        name: "monospace",
        open: "{{",
        close: "}}",
        html_open: "<tt>",
        html_close: "</tt>",
        span_tag: false,
    },
    FormatCase {
        name: "color",
        open: "##red|",
        close: "##",
        html_open: r#"<span style="color: red">"#,
        html_close: "</span>",
        span_tag: true,
    },
];

#[test]
fn all_ordered_crossed_format_pairs_match_wikidot_split_and_reopen_dom() {
    for outer in FORMATS {
        for inner in FORMATS {
            if outer.name == inner.name {
                continue;
            }
            let source = format!(
                "{}OUTER {}INNER{} TAIL{}",
                outer.open, inner.open, outer.close, inner.close,
            );
            let expected = if outer.span_tag && inner.span_tag {
                format!(
                    "<p>{}OUTER {}INNER{} TAIL{}</p>",
                    outer.html_open, inner.html_open, inner.html_close, outer.html_close,
                )
            } else {
                format!(
                    "<p>{}OUTER {}INNER{}{} {}TAIL{}</p>",
                    outer.html_open,
                    inner.html_open,
                    inner.html_close,
                    outer.html_close,
                    inner.html_open,
                    inner.html_close,
                )
            };
            assert_eq!(
                render(&source),
                expected,
                "{} crossed by {}",
                outer.name,
                inner.name,
            );
        }
    }
}

#[test]
fn all_ordered_fifo_triples_match_wikidot_stack_transitions() {
    for outer in FORMATS {
        for middle in FORMATS {
            for inner in FORMATS {
                if outer.name == middle.name
                    || outer.name == inner.name
                    || middle.name == inner.name
                {
                    continue;
                }

                let source = format!(
                    "{}A {}B {}C{} X{} Y{}",
                    outer.open,
                    middle.open,
                    inner.open,
                    outer.close,
                    middle.close,
                    inner.close,
                );
                let outer_matches_middle = outer.span_tag && middle.span_tag;
                let outer_matches_inner = outer.span_tag && inner.span_tag;
                let middle_matches_inner = middle.span_tag && inner.span_tag;
                let expected = if outer_matches_inner {
                    format!(
                        "<p>{}A {}B {}C{} X{} Y{}</p>",
                        outer.html_open,
                        middle.html_open,
                        inner.html_open,
                        inner.html_close,
                        middle.html_close,
                        outer.html_close,
                    )
                } else if middle_matches_inner {
                    format!(
                        "<p>{}A {}B {}C{}{} {}X{} Y{}</p>",
                        outer.html_open,
                        middle.html_open,
                        inner.html_open,
                        inner.html_close,
                        middle.html_close,
                        middle.html_open,
                        middle.html_close,
                        outer.html_close,
                    )
                } else if outer_matches_middle {
                    format!(
                        "<p>{}A {}B {}C{}{} {}X{} Y{}</p>",
                        outer.html_open,
                        middle.html_open,
                        inner.html_open,
                        inner.html_close,
                        middle.html_close,
                        inner.html_open,
                        inner.html_close,
                        outer.html_close,
                    )
                } else {
                    format!(
                        "<p>{}A {}B {}C{}{} {}{}X{} Y{}{}</p>",
                        outer.html_open,
                        middle.html_open,
                        inner.html_open,
                        inner.html_close,
                        middle.html_close,
                        inner.html_open,
                        middle.html_open,
                        middle.html_close,
                        inner.html_close,
                        outer.html_close,
                    )
                };

                assert_eq!(
                    render(&source),
                    expected,
                    "{} crossed by {} crossed by {}",
                    outer.name,
                    middle.name,
                    inner.name,
                );
            }
        }
    }
}

#[test]
fn same_family_delimiters_commit_left_to_right() {
    for (source, expected) in [
        ("**A **B** C**", "<p><strong>A **B</strong> C**</p>"),
        ("//A //B// C//", "<p><em>A //B</em> C//</p>"),
        (
            "__A __B__ C__",
            r#"<p><span style="text-decoration: underline;">A __B</span> C__</p>"#,
        ),
        (",,A ,,B,, C,,", "<p><sub>A ,,B</sub> C,,</p>"),
        ("^^A ^^B^^ C^^", "<p><sup>A ^^B</sup> C^^</p>"),
        (
            "--A --B-- C--",
            r#"<p><span style="text-decoration: line-through;">A —B</span> C—</p>"#,
        ),
        ("////", ""),
    ] {
        assert_eq!(render(source), expected, "{source:?}");
    }
}

#[test]
fn dash_owner_precedes_typography_and_link_recovery() {
    for (source, expected) in [
        (
            "--A----B--",
            concat!(
                r#"<p><span style="text-decoration: line-through;">A</span>"#,
                r#"<span style="text-decoration: line-through;">B</span></p>"#,
            ),
        ),
        (
            "--A----",
            r#"<p><span style="text-decoration: line-through;">A</span>—</p>"#,
        ),
        ("-- A--", "<p>— A—</p>"),
        (
            "[https://example.com --A--]",
            r#"<p><a href="https://example.com">--A--</a></p>"#,
        ),
    ] {
        assert_eq!(render(source), expected, "{source:?}");
    }

    assert_eq!(
        render_with_missing_pages("[[[scp-002|--A--]]]"),
        r#"<p><a class="newpage" href="/scp-002">--A--</a></p>"#,
    );
}

#[test]
fn odd_monospace_brace_runs_keep_the_final_brace_inside_the_owner() {
    for (source, expected) in [
        ("{{{X}}}", "<p><tt>{X}</tt></p>"),
        ("{{{{{X}}}}}", "<p><tt>{{{X}}}</tt></p>"),
        ("{{X}}}", "<p><tt>X}</tt></p>"),
        ("A{{{雪}}}B", "<p>A<tt>{雪}</tt>B</p>"),
        ("{{{ X }}}", "<p><tt>{ X }</tt></p>"),
        ("{{}}", ""),
    ] {
        assert_eq!(render(source), expected, "{source:?}");
    }
}

#[test]
fn repeated_crossed_formats_scale_with_document_size() {
    let source = std::iter::repeat_n("**OUTER //INNER** TAIL//", 2_048)
        .collect::<Vec<_>>()
        .join(" ");
    let started = Instant::now();
    let html = render(&source);
    let elapsed = started.elapsed();

    assert_eq!(html.matches("<strong>").count(), 2_048, "{html}");
    assert_eq!(html.matches("<em>").count(), 4_096, "{html}");
    assert!(
        elapsed < Duration::from_secs(3),
        "crossed formatter normalization took {elapsed:?}",
    );
}
