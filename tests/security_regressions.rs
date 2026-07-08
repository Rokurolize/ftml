use ftml::data::{PageInfo, PageRef, ScoreValue};
use ftml::includes::DebugIncluder;
use ftml::layout::Layout;
use ftml::parsing::ParseErrorKind;
use ftml::render::Render;
use ftml::render::html::HtmlOutput;
use ftml::render::html::HtmlRender;
use ftml::render::text::TextRender;
use ftml::settings::{WikitextMode, WikitextSettings};
use ftml::tree::{
    AttributeMap, ContainerType, Element, LinkLabel, LinkLocation, LinkType, ListItem,
    PartialElement, RubyText, SyntaxTree, Tab, TableCell, TableRow,
};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::num::NonZeroU32;
use std::time::{Duration, Instant};

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("security-regression"),
        category: None,
        site: Cow::Borrowed("security"),
        title: Cow::Borrowed("Security Regression"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![],
        language: Cow::Borrowed("default"),
    }
}

fn parse(input: &str, layout: Layout) -> SyntaxTree<'static> {
    let (tree, errors) = parse_with_errors(input, layout);

    assert!(errors.is_empty(), "{errors:?}");
    tree
}

fn parse_with_errors(
    input: &str,
    layout: Layout,
) -> (SyntaxTree<'static>, Vec<ftml::parsing::ParseError>) {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
    let mut text = input.to_owned();
    ftml::preprocess(&mut text);
    let tokens = ftml::tokenize(&text);
    let result = ftml::parse(&tokens, &page_info, &settings);
    let (tree, errors) = result.into();

    (tree.to_owned(), errors)
}

fn render_html(tree: &SyntaxTree, layout: Layout) -> String {
    render_html_output(tree, layout).body
}

fn render_html_output(tree: &SyntaxTree, layout: Layout) -> HtmlOutput {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
    HtmlRender.render(tree, &page_info, &settings)
}

fn render_text(tree: &SyntaxTree, layout: Layout) -> String {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
    TextRender.render(tree, &page_info, &settings)
}

fn render_iframe_with_url_and_attributes(
    url: &'static str,
    attributes: AttributeMap,
) -> String {
    let tree = SyntaxTree {
        elements: vec![Element::Iframe {
            url: Cow::Borrowed(url),
            attributes,
        }],
        ..SyntaxTree::default()
    };

    render_html(&tree, Layout::Wikijump)
}

fn render_math_inline_source(latex_source: &'static str) -> String {
    let tree = SyntaxTree {
        elements: vec![Element::MathInline {
            latex_source: Cow::Borrowed(latex_source),
        }],
        ..SyntaxTree::default()
    };

    render_html(&tree, Layout::Wikijump)
}

fn render_iframe_with_attributes(attributes: AttributeMap) -> String {
    render_iframe_with_url_and_attributes("https://example.com/embed", attributes)
}

fn table_cell(text: &'static str) -> TableCell<'static> {
    TableCell {
        header: false,
        column_span: NonZeroU32::new(1).expect("one is non-zero"),
        align: None,
        attributes: AttributeMap::new(),
        elements: vec![Element::Text(Cow::Borrowed(text))],
    }
}

#[test]
fn color_markup_cannot_add_css_declarations() {
    for payload in [
        "red;background-image:url(https://attacker.invalid/pixel)",
        "red\";background-image:url(https://attacker.invalid/pixel)",
        "red';background-image:url(https://attacker.invalid/pixel)",
        "red&#59;background-image:url(https://attacker.invalid/pixel)",
    ] {
        let tree = parse(&format!("##{payload}|text##"), Layout::Wikijump);
        let output = render_html(&tree, Layout::Wikijump);

        assert!(output.contains(r#"style="color: inherit;""#));
        assert!(!output.contains("background-image"));
    }
}

#[test]
fn size_block_cannot_add_css_declarations() {
    for payload in [
        "80%;background-image:url(https://attacker.invalid/pixel)",
        "80%\";background-image:url(https://attacker.invalid/pixel)",
        "80%';background-image:url(https://attacker.invalid/pixel)",
        "80%&#59;background-image:url(https://attacker.invalid/pixel)",
    ] {
        let tree = parse(
            &format!("[[size {payload}]]text[[/size]]"),
            Layout::Wikijump,
        );
        let output = render_html(&tree, Layout::Wikijump);

        assert!(output.contains(r#"style="font-size: inherit;""#));
        assert!(!output.contains("background-image"));
    }

    let tree = parse("[[size 2vh]]text[[/size]]", Layout::Wikijump);
    let output = render_html(&tree, Layout::Wikijump);

    assert!(output.contains(r#"style="font-size: 2vh;""#));
}

#[test]
fn empty_label_triple_url_link_does_not_panic() {
    let tree = parse("[[[https://example.com|]]]", Layout::Wikidot);
    let output = render_html(&tree, Layout::Wikidot);

    assert!(output.contains(r#"href="https://example.com""#));
    assert!(output.contains(">https://example.com</a>"));
}

#[test]
fn protocol_relative_links_are_classified_as_external() {
    let tree = parse("[//attacker.invalid/path click]", Layout::Wikijump);
    let output = render_html_output(&tree, Layout::Wikijump);

    assert!(output.body.contains(r#"href="//attacker.invalid/path""#));
    assert!(output.body.contains("wj-link-external"));
    assert!(output.backlinks.internal_links.is_empty());
    assert_eq!(
        output
            .backlinks
            .external_links
            .iter()
            .map(Cow::as_ref)
            .collect::<Vec<_>>(),
        vec!["//attacker.invalid/path"],
    );

    let tree = parse("[/local-page local]", Layout::Wikijump);
    let output = render_html_output(&tree, Layout::Wikijump);

    assert!(output.body.contains("wj-link-internal"));
    assert!(output.backlinks.external_links.is_empty());
    assert_eq!(
        output.backlinks.internal_links,
        vec![PageRef::page_only("local-page")],
    );
}

#[test]
fn unsafe_triple_url_scheme_is_not_rendered_into_href() {
    for url in [
        "javascript:alert(1)",
        " JaVaScRiPt:alert(1)",
        "\tjavascript:alert(1)",
    ] {
        let tree = SyntaxTree {
            elements: vec![Element::Link {
                ltype: LinkType::Direct,
                link: LinkLocation::Url(Cow::Borrowed(url)),
                label: LinkLabel::Text(Cow::Borrowed("click")),
                target: None,
            }],
            ..SyntaxTree::default()
        };
        let output = render_html_output(&tree, Layout::Wikidot);

        assert!(!output.body.to_ascii_lowercase().contains("javascript:"));
        assert!(output.body.contains(r##"href="#invalid-url""##));
        assert!(output.backlinks.internal_links.is_empty());
        assert!(output.backlinks.external_links.is_empty());
    }

    {
        let url = "#frag";
        let tree = SyntaxTree {
            elements: vec![Element::Link {
                ltype: LinkType::Direct,
                link: LinkLocation::Url(Cow::Borrowed(url)),
                label: LinkLabel::Text(Cow::Borrowed("click")),
                target: None,
            }],
            ..SyntaxTree::default()
        };
        let output = render_html_output(&tree, Layout::Wikidot);

        assert!(output.body.contains(r##"href="#frag""##));
        assert!(output.backlinks.internal_links.is_empty());
        assert!(output.backlinks.external_links.is_empty());
    }
}

#[test]
fn mathml_text_payloads_are_escaped_without_disabling_mathml() {
    let output = render_math_inline_source(r"\text{<script>alert(1)</script> & <>}");
    let lower_output = output.to_ascii_lowercase();

    assert!(!lower_output.contains("<script"));
    assert!(output.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(output.contains("&amp;"));
    assert!(output.contains("&lt;&gt;"));
    assert!(output.contains("<wj-math-ml"));
    assert!(output.contains("<math "));
    assert!(output.contains("<mtext>"));
}

#[test]
fn mathml_operators_and_parse_error_text_are_escaped() {
    let output = render_math_inline_source(r"x < y & z");
    let lower_output = output.to_ascii_lowercase();

    assert!(!lower_output.contains("<script"));
    assert!(output.contains("<math "));
    assert!(output.contains("<mo>&lt;</mo>"));
    assert!(output.contains("&amp;"));

    let output = render_math_inline_source(r"2^{\pi - 1");

    assert!(!output.contains("<script"));
    assert!(output.contains("wj-error-inline") || output.contains("[PARSE ERROR:"));
}

#[test]
fn benign_mathml_rendering_still_outputs_math_elements() {
    let output = render_math_inline_source("x^2");

    assert!(output.contains("<wj-math-ml"));
    assert!(output.contains(
        r#"<math xmlns="http://www.w3.org/1998/Math/MathML" display="inline">"#
    ));
    assert!(output.contains("<msup><mi>x</mi><mn>2</mn></msup>"));
}

#[test]
fn unsafe_iframe_src_scheme_is_not_rendered() {
    for url in [
        "javascript:alert(1)",
        " JaVaScRiPt:alert(1)",
        "\tjavascript:alert(1)",
    ] {
        let output = render_iframe_with_url_and_attributes(url, AttributeMap::new());

        assert!(!output.to_ascii_lowercase().contains("javascript:"));
        assert!(output.contains(r##"src="#invalid-url""##));
    }
}

#[test]
fn raw_attribute_maps_are_sanitized_before_rendering() {
    let mut raw = BTreeMap::new();
    raw.insert(Cow::Borrowed("onclick"), Cow::Borrowed("alert(1)"));
    raw.insert(Cow::Borrowed("href"), Cow::Borrowed(" JaVaScRiPt:alert(1)"));
    let from_raw_attributes = AttributeMap::from(raw);

    assert!(!from_raw_attributes.get().contains_key("onclick"));
    assert_eq!(
        from_raw_attributes.get().get("href").map(Cow::as_ref),
        Some("#invalid-url"),
    );

    let from_raw_map = render_iframe_with_attributes(from_raw_attributes);

    assert!(!from_raw_map.contains("onclick"));
    assert!(!from_raw_map.contains("javascript:"));
    assert!(from_raw_map.contains(r##"href="#invalid-url""##));

    let mut inserted = AttributeMap::new();
    assert!(!inserted.insert("onclick", Cow::Borrowed("alert(1)")));
    assert!(inserted.insert("href", Cow::Borrowed(" JaVaScRiPt:alert(1)")));
    assert!(!inserted.get().contains_key("onclick"));
    assert_eq!(
        inserted.get().get("href").map(Cow::as_ref),
        Some("#invalid-url")
    );

    let from_insert = render_iframe_with_attributes(inserted);

    assert!(!from_insert.contains("onclick"));
    assert!(!from_insert.contains("javascript:"));
    assert!(from_insert.contains(r##"href="#invalid-url""##));

    let deserialized =
        serde_json::from_str(r#"{"onclick":"alert(1)","href":"\tjavascript:alert(1)"}"#)
            .expect("attribute map should deserialize");
    let deserialized: AttributeMap = deserialized;

    assert!(!deserialized.get().contains_key("onclick"));
    assert_eq!(
        deserialized.get().get("href").map(Cow::as_ref),
        Some("#invalid-url"),
    );

    let from_deserialize = render_iframe_with_attributes(deserialized);

    assert!(!from_deserialize.contains("onclick"));
    assert!(!from_deserialize.contains("javascript:"));
    assert!(from_deserialize.contains(r##"href="#invalid-url""##));

    let mut local_anchor = AttributeMap::new();
    assert!(local_anchor.insert("usemap", Cow::Borrowed("not-an-anchor")));
    assert_eq!(
        local_anchor.get().get("usemap").map(Cow::as_ref),
        Some("#invalid-url"),
    );
}

#[test]
fn mismatched_link_labels_render_fallback_text() {
    let tree = SyntaxTree {
        elements: vec![
            Element::Link {
                ltype: LinkType::Direct,
                link: LinkLocation::Page(PageRef::page_only("target-page")),
                label: LinkLabel::Url,
                target: None,
            },
            Element::Text(Cow::Borrowed(" ")),
            Element::Link {
                ltype: LinkType::Direct,
                link: LinkLocation::Url(Cow::Borrowed("https://example.com")),
                label: LinkLabel::Page,
                target: None,
            },
        ],
        ..SyntaxTree::default()
    };
    let html = render_html(&tree, Layout::Wikijump);
    let text = render_text(&tree, Layout::Wikijump);

    assert!(html.contains(">target-page</a>"));
    assert!(html.contains(">https://example.com</a>"));
    assert_eq!(text, "target-page https://example.com");
}

#[test]
fn malformed_ast_footnote_reference_renders_error() {
    let tree = SyntaxTree {
        elements: vec![Element::Footnote],
        footnotes: vec![],
        ..SyntaxTree::default()
    };
    let output = render_html(&tree, Layout::Wikijump);

    assert!(output.contains("wj-error-inline"));
    assert!(output.contains("Footnote item not found"));
}

#[test]
fn malformed_ast_bibliography_block_renders_error() {
    let tree = SyntaxTree {
        elements: vec![Element::BibliographyBlock {
            index: 0,
            title: None,
            hide: false,
        }],
        ..SyntaxTree::default()
    };
    let output = render_html(&tree, Layout::Wikijump);

    assert!(output.contains("wj-error-inline"));
    assert!(output.contains("Bibliography block not found"));
}

#[test]
fn recursive_bibcite_tooltips_render_error_instead_of_recursing() {
    for input in [
        "[[bibliography]]\n: a : ((bibcite a))\n[[/bibliography]]\n((bibcite a))",
        "[[bibliography]]\n: a : ((bibcite b))\n: b : ((bibcite a))\n[[/bibliography]]\n((bibcite a))",
    ] {
        let tree = parse(input, Layout::Wikijump);
        let output = render_html(&tree, Layout::Wikijump);

        assert!(output.contains("wj-error-inline"));
        assert!(output.contains("Bibliography item not found"));
        assert!(output.len() < 20_000);
    }
}

#[test]
fn non_bibcite_parentheses_render_as_text() {
    let tree = parse("before ((notbibcite label)) after", Layout::Wikijump);
    let output = render_html(&tree, Layout::Wikijump);

    assert!(output.contains("before"));
    assert!(output.contains("notbibcite"));
    assert!(output.contains("after"));
}

#[test]
fn malformed_partial_elements_render_fallback_html() {
    let tree = SyntaxTree {
        elements: vec![
            Element::Partial(PartialElement::ListItem(ListItem::Elements {
                attributes: AttributeMap::new(),
                elements: vec![Element::Text(Cow::Borrowed("partial"))],
            })),
            Element::Partial(PartialElement::ListItem(ListItem::SubList {
                element: Box::new(Element::Text(Cow::Borrowed("sublist"))),
            })),
            Element::Partial(PartialElement::TableRow(TableRow {
                attributes: AttributeMap::new(),
                cells: vec![table_cell("row-cell")],
            })),
            Element::Partial(PartialElement::TableCell(table_cell("cell"))),
            Element::Partial(PartialElement::Tab(Tab {
                label: Cow::Borrowed("tab-label"),
                elements: vec![Element::Text(Cow::Borrowed("tab-body"))],
            })),
            Element::Partial(PartialElement::RubyText(RubyText {
                attributes: AttributeMap::new(),
                elements: vec![Element::Text(Cow::Borrowed("ruby"))],
            })),
        ],
        ..SyntaxTree::default()
    };
    let output = render_html(&tree, Layout::Wikijump);

    for expected in [
        "partial",
        "sublist",
        "row-cell",
        "cell",
        "tab-label",
        "tab-body",
        "ruby",
    ] {
        assert!(
            output.contains(expected),
            "{expected} missing from {output}"
        );
    }
    assert!(output.contains("tab-label</span> tab-body"));
}

#[test]
fn malformed_partial_elements_render_fallback_text() {
    let cases = [
        (
            Element::Partial(PartialElement::ListItem(ListItem::Elements {
                attributes: AttributeMap::new(),
                elements: vec![],
            })),
            "",
        ),
        (
            Element::Partial(PartialElement::ListItem(ListItem::SubList {
                element: Box::new(Element::Text(Cow::Borrowed("sublist"))),
            })),
            "sublist",
        ),
        (
            Element::Partial(PartialElement::TableRow(TableRow {
                attributes: AttributeMap::new(),
                cells: vec![table_cell("row-cell")],
            })),
            "row-cell",
        ),
        (
            Element::Partial(PartialElement::TableCell(table_cell("cell"))),
            "cell",
        ),
        (
            Element::Partial(PartialElement::Tab(Tab {
                label: Cow::Borrowed("tab-label"),
                elements: vec![Element::Text(Cow::Borrowed("tab-body"))],
            })),
            "tab-label\ntab-body",
        ),
        (
            Element::Partial(PartialElement::RubyText(RubyText {
                attributes: AttributeMap::new(),
                elements: vec![Element::Text(Cow::Borrowed("ruby"))],
            })),
            "(ruby)",
        ),
    ];

    for (element, expected) in cases {
        let tree = SyntaxTree {
            elements: vec![element],
            ..SyntaxTree::default()
        };

        assert_eq!(render_text(&tree, Layout::Wikijump), expected);
    }

    for (element, expected) in [
        (
            Element::Partial(PartialElement::ListItem(ListItem::Elements {
                attributes: AttributeMap::new(),
                elements: vec![Element::Text(Cow::Borrowed("partial"))],
            })),
            "prefix\npartial",
        ),
        (
            Element::Partial(PartialElement::TableRow(TableRow {
                attributes: AttributeMap::new(),
                cells: vec![table_cell("row-cell")],
            })),
            "prefix\nrow-cell",
        ),
        (
            Element::Partial(PartialElement::Tab(Tab {
                label: Cow::Borrowed("tab-label"),
                elements: vec![Element::Text(Cow::Borrowed("tab-body"))],
            })),
            "prefix\ntab-label\ntab-body",
        ),
    ] {
        let tree = SyntaxTree {
            elements: vec![Element::Text(Cow::Borrowed("prefix")), element],
            ..SyntaxTree::default()
        };

        assert_eq!(render_text(&tree, Layout::Wikijump), expected);
    }
}

#[test]
fn empty_page_includes_are_rejected() {
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
    let (output, pages) = ftml::include(
        "[[include :scp-wiki:]]",
        &settings,
        ftml::includes::NullIncluder,
        || -> Infallible { unreachable!("no include request should be emitted") },
    )
    .expect("malformed include should be ignored, not treated as an include request");

    assert_eq!(output, "[[include :scp-wiki:]]");
    assert!(pages.is_empty());

    let (_tree, errors) =
        parse_with_errors("[[include-elements :scp-wiki:]]", Layout::Wikijump);
    assert!(
        errors
            .iter()
            .any(|error| error.kind() == ParseErrorKind::BlockMalformedArguments),
        "{errors:?}",
    );
}

#[test]
fn malformed_include_prefixes_are_skipped_once() {
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
    let input = "[[include :scp-wiki:]]\n[[include component:ok]]";
    let (output, pages) =
        ftml::include(input, &settings, DebugIncluder, || -> Infallible {
            unreachable!("debug includer should match include requests")
        })
        .expect("include preprocessing should not fail");

    assert!(output.contains("[[include :scp-wiki:]]"));
    assert!(output.contains("<INCLUDED-PAGE component:ok {}>"));
    assert_eq!(pages, vec![PageRef::page_only("component:ok")]);

    let repeated = "[[include page\n".repeat(6_400);
    let started = Instant::now();
    let (output, pages) = ftml::include(
        &repeated,
        &settings,
        ftml::includes::NullIncluder,
        || -> Infallible { unreachable!("null includer should not fail") },
    )
    .expect("malformed includes should be ignored");

    assert_eq!(output, repeated);
    assert!(pages.is_empty());
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn empty_list_lines_do_not_fallback_quadratically() {
    let input = "* \n".repeat(1_000);
    let (tree, errors) = parse_with_errors(&input, Layout::Wikijump);

    assert!(errors.is_empty(), "{errors:?}");
    assert!(tree.elements.is_empty(), "{:?}", tree.elements);
}

#[test]
fn over_limit_date_formats_fall_back_without_format_class() {
    let format = "%c".repeat(65);
    let input = format!(r#"[[date 2010-01-01 format="{format}"]]"#);
    let tree = parse(&input, Layout::Wikijump);
    let output = render_html_output(&tree, Layout::Wikijump);

    assert!(!output.body.contains("format_"));
    assert!(!output.body.contains("%25c"));
    assert!(output.body.contains("wj-date"));
}

#[test]
fn hidden_conditionals_do_not_publish_metadata_blocks() {
    for input in [
        "[[iftags +missing]]\n[[code]]\nsecret\n[[/code]]\n[[html]]\n<b>secret</b>\n[[/html]]\n[[/iftags]]",
        "[[ifcategory missing]]\n[[code]]\nsecret\n[[/code]]\n[[html]]\n<b>secret</b>\n[[/html]]\n[[/ifcategory]]",
    ] {
        let tree = parse(input, Layout::Wikijump);

        assert!(tree.elements.is_empty(), "{:?}", tree.elements);
        assert!(tree.code_blocks.is_empty(), "{:?}", tree.code_blocks);
        assert!(tree.html_blocks.is_empty(), "{:?}", tree.html_blocks);
    }

    let tree = parse(
        "[[ifcategory _default]]\n[[code]]\nvisible\n[[/code]]\n[[/ifcategory]]",
        Layout::Wikijump,
    );

    assert_eq!(tree.code_blocks.len(), 1);
    assert_eq!(tree.code_blocks[0].contents, "visible");
}

#[test]
fn deeply_repeated_headings_parse_without_stack_recursion() {
    let input = (0..10_000)
        .map(|index| format!("+ heading {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let tree = parse(&input, Layout::Wikijump);

    assert_eq!(tree.elements.len(), 10_000);
    assert!(tree.elements.iter().all(|element| {
        matches!(
            element,
            Element::Container(container)
                if matches!(container.ctype(), ContainerType::Header(_))
        )
    }));
}
