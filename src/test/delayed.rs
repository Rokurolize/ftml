use crate::data::{PageInfo, PageRef};
use crate::delayed::{
    DelayedInput, GeneratedInput, GeneratedKind, GeneratedValue, InputSegment,
    ResolvedTagRef, SlotBindings, SlotId, TextOrigin, parse_delayed_list,
};
use crate::layout::Layout;
use crate::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;

fn page_link_input(source: &str) -> DelayedInput<'_> {
    let marker = "%%title_linked%%";
    let start = source.find(marker).expect("fixture marker");
    let end = start + marker.len();
    DelayedInput::new(
        source,
        vec![
            InputSegment::text(0..start, TextOrigin::Authored),
            InputSegment::generated(GeneratedInput {
                source_range: start..end,
                id: SlotId::new(1),
                kind: GeneratedKind::PageLink,
                occurrence: 0,
            }),
            InputSegment::text(end..source.len(), TextOrigin::Authored),
        ],
    )
    .expect("valid delayed fixture")
}

fn page_bindings() -> SlotBindings<'static> {
    page_bindings_for(
        PageRef::page_only("component:image-block"),
        "Standard Image Block",
    )
}

fn page_bindings_for(page: PageRef, label: &'static str) -> SlotBindings<'static> {
    SlotBindings::new(vec![(
        SlotId::new(1),
        GeneratedValue::PageLink {
            page,
            label: Cow::Borrowed(label),
        },
    )])
    .expect("unique bindings")
}

fn render(source: &str) -> String {
    let input = page_link_input(source);
    render_input(&input)
}

fn render_input(input: &DelayedInput<'_>) -> String {
    render_input_with_bindings(input, &page_bindings())
}

fn render_input_with_bindings(
    input: &DelayedInput<'_>,
    bindings: &SlotBindings<'_>,
) -> String {
    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::List, Layout::Wikidot);
    let delayed = parse_delayed_list(input, &page_info, &settings)
        .expect("supported delayed input");
    let bound = delayed.bind(bindings).expect("matching bindings");
    bound.render_html(&page_info, &settings).body().to_owned()
}

fn render_tag(source: &str) -> String {
    render_tag_values(source, &["component"], " ")
}

fn render_tag_values(source: &str, tags: &[&str], separator: &str) -> String {
    let marker = "%%tags_linked%%";
    let start = source.find(marker).expect("fixture marker");
    let end = start + marker.len();
    let input = DelayedInput::new(
        source,
        vec![
            InputSegment::text(0..start, TextOrigin::Authored),
            InputSegment::generated(GeneratedInput {
                source_range: start..end,
                id: SlotId::new(2),
                kind: GeneratedKind::TagLinks,
                occurrence: 0,
            }),
            InputSegment::text(end..source.len(), TextOrigin::Authored),
        ],
    )
    .expect("valid delayed fixture");
    let bindings = SlotBindings::new(vec![(
        SlotId::new(2),
        GeneratedValue::TagLinks {
            tags: Cow::Owned(
                tags.iter()
                    .map(|tag| ResolvedTagRef {
                        tag: Cow::Owned((*tag).to_owned()),
                    })
                    .collect(),
            ),
            separator: Cow::Owned(separator.to_owned()),
        },
    )])
    .expect("unique bindings");
    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::List, Layout::Wikidot);
    let delayed = parse_delayed_list(&input, &page_info, &settings)
        .expect("supported delayed input");
    let bound = delayed.bind(&bindings).expect("matching bindings");
    bound.render_html(&page_info, &settings).body().to_owned()
}

#[test]
fn delayed_page_link_is_an_active_inline_leaf_without_textual_substitution() {
    assert_eq!(
        render("BEGIN|**%%title_linked%%**|END"),
        concat!(
            "<p>BEGIN|<strong>",
            "<a href=\"/component:image-block\">Standard Image Block</a>",
            "</strong>|END</p>",
        ),
    );
}

#[test]
fn delayed_page_link_uses_legacy_source_projection_inside_inline_raw() {
    assert_eq!(
        render("BEGIN|@@%%title_linked%%@@|END"),
        concat!(
            "<p>BEGIN|<span style=\"white-space: pre-wrap;\">",
            "[[[component:image-block | Standard Image Block]]]",
            "</span>|END</p>",
        ),
    );
}

#[test]
fn delayed_page_link_is_omitted_with_its_comment_owner() {
    assert_eq!(
        render("BEGIN|[!-- %%title_linked%% --]|END"),
        "<p>BEGIN||END</p>",
    );
}

#[test]
fn slot_occupancy_recovers_outer_links_as_authored_source() {
    assert_eq!(
        render("BEGIN|[[[scp-003|%%title_linked%%]]]|END"),
        concat!(
            "<p>BEGIN|[[[scp-003|",
            "<a href=\"/component:image-block\">Standard Image Block</a>",
            "]]]|END</p>",
        ),
    );
    assert_eq!(
        render("BEGIN|[https://example.com %%title_linked%%]|END"),
        concat!(
            "<p>BEGIN|[<a href=\"https://example.com\">https://example.com</a> ",
            "<a href=\"/component:image-block\">Standard Image Block</a>",
            "]|END</p>",
        ),
    );
}

#[test]
fn page_and_tag_slots_have_distinct_span_attribute_recovery() {
    assert_eq!(
        render("BEGIN|[[span title=\"%%title_linked%%\"]]X[[/span]]|END"),
        "<p>BEGIN|<span>X</span>|END</p>",
    );
    assert_eq!(
        render_tag("BEGIN|[[span title=\"%%tags_linked%%\"]]X[[/span]]|END"),
        concat!(
            "<p>BEGIN|[[span title=&quot;",
            "<a href=\"/system:page-tags/tag/component\">component</a>",
            "&quot;]]X[[/span]]|END</p>",
        ),
    );
}

#[test]
fn page_slot_recovers_image_owner_while_tag_slot_stays_attribute_text() {
    assert_eq!(
        render("BEGIN|[[image https://example.com/x.png alt=\"%%title_linked%%\"]]|END",),
        concat!(
            "<p>BEGIN|[[image ",
            "<a href=\"https://example.com/x.png\">https://example.com/x.png</a>",
            " alt=&quot;",
            "<a href=\"/component:image-block\">Standard Image Block</a>",
            "&quot;]]|END</p>",
        ),
    );
    assert_eq!(
        render_tag(
            "BEGIN|[[image https://example.com/x.png alt=\"%%tags_linked%%\"]]|END",
        ),
        concat!(
            "\n\nBEGIN|<img src=\"https://example.com/x.png\" class=\"image\" ",
            "alt=\"[/system:page-tags/tag/component component]\">|END",
        ),
    );
}

#[test]
fn slot_schema_rejects_missing_and_wrong_kind_bindings_atomically() {
    let source = "%%title_linked%%";
    let input = page_link_input(source);
    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::List, Layout::Wikidot);
    let delayed = parse_delayed_list(&input, &page_info, &settings)
        .expect("supported delayed input");

    assert!(delayed.bind(&SlotBindings::empty()).is_err());
    let wrong = SlotBindings::new(vec![(
        SlotId::new(1),
        GeneratedValue::TagLinks {
            tags: Cow::Borrowed(&[]),
            separator: Cow::Borrowed(" "),
        },
    )])
    .expect("unique bindings");
    assert!(delayed.bind(&wrong).is_err());
}

#[test]
fn authored_marker_shaped_text_cannot_steal_a_generated_slot() {
    let source = "%%title_linked%%|%%title_linked%%|%%title_linked%%";
    let marker = "%%title_linked%%";
    let generated_start = marker.len() + 1;
    let generated_end = generated_start + marker.len();
    let input = DelayedInput::new(
        source,
        vec![
            InputSegment::text(0..generated_start, TextOrigin::RuntimeScalar),
            InputSegment::generated(GeneratedInput {
                source_range: generated_start..generated_end,
                id: SlotId::new(1),
                kind: GeneratedKind::PageLink,
                occurrence: 0,
            }),
            InputSegment::text(generated_end..source.len(), TextOrigin::RuntimeScalar),
        ],
    )
    .expect("valid collision fixture");
    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::List, Layout::Wikidot);
    let delayed = parse_delayed_list(&input, &page_info, &settings)
        .expect("supported delayed input");
    let bound = delayed
        .bind(&page_bindings())
        .expect("out-of-band slot binds atomically");

    assert_eq!(
        bound.render_html(&page_info, &settings).body(),
        concat!(
            "<p>%%title_linked%%|",
            "<a href=\"/component:image-block\">Standard Image Block</a>",
            "|%%title_linked%%</p>",
        ),
    );
}

#[test]
fn nested_line_start_owners_bind_without_retaining_delayed_leaves() {
    for source in [
        "|| %%title_linked%% ||\n",
        "* %%title_linked%%\n",
        "term:: %%title_linked%%\n",
        concat!(
            "[[tabview]]\n",
            "[[tab One]]\n",
            "%%title_linked%%\n",
            "[[/tab]]\n",
            "[[/tabview]]",
        ),
    ] {
        let input = page_link_input(source);
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::List, Layout::Wikidot);
        let delayed = parse_delayed_list(&input, &page_info, &settings)
            .expect("supported delayed input");
        let bound = delayed
            .bind(&page_bindings())
            .expect("every nested owner is traversed during atomic binding");
        let html = bound.render_html(&page_info, &settings);
        assert!(
            html.body()
                .contains("<a href=\"/component:image-block\">Standard Image Block</a>",)
                && !html.body().contains("%%title_linked%%"),
            "a renderable nested owner must be completely bound: {source}",
        );
    }
}

#[test]
fn bibliography_definition_values_bind_before_rendering() {
    let html = render(concat!(
        "[[bibliography title=\"Works\"]]\n",
        ": alpha : %%title_linked%%\n",
        "[[/bibliography]]",
    ));

    assert!(
        html.contains(concat!(
            r#"<a href="/component:image-block">"#,
            "Standard Image Block</a>",
        )),
        "bibliography values are out-of-band syntax-tree owners: {html}",
    );
    assert!(
        !html.contains("%%title_linked%%"),
        "a bound bibliography must not retain a delayed leaf: {html}",
    );
}

#[test]
fn adjacent_text_segments_do_not_create_synthetic_token_boundaries() {
    let source = "**bold** %%title_linked%%";
    let marker_start = source.find("%%title_linked%%").expect("fixture marker");
    let marker_end = source.len();

    for second_origin in [TextOrigin::Authored, TextOrigin::RuntimeScalar] {
        let input = DelayedInput::new(
            source,
            vec![
                InputSegment::text(0..1, TextOrigin::Authored),
                InputSegment::text(1..marker_start, second_origin),
                InputSegment::generated(GeneratedInput {
                    source_range: marker_start..marker_end,
                    id: SlotId::new(1),
                    kind: GeneratedKind::PageLink,
                    occurrence: 0,
                }),
            ],
        )
        .expect("adjacent text remains a valid provenance split");

        assert_eq!(
            render_input(&input),
            concat!(
                "<p><strong>bold</strong> ",
                "<a href=\"/component:image-block\">Standard Image Block</a>",
                "</p>",
            ),
            "text provenance must not alter syntax: {second_origin:?}",
        );
    }
}

#[test]
fn delayed_heading_binds_before_building_its_table_of_contents_label() {
    let html = render("[[toc]]\n+ %%title_linked%%");
    assert!(
        html.contains(concat!(
            r#"<div style="margin-left: 1em;">"#,
            r#"<a href="javascript:;">Standard Image Block</a></div>"#,
        ),),
        "the TOC label should use the bound heading text: {html}",
    );
    assert!(
        html.contains(concat!(
            r#"<h1><span><a href="/component:image-block">"#,
            "Standard Image Block</a></span></h1>",
        ),),
        "the heading should retain the bound page link: {html}",
    );
}

#[test]
fn tag_external_label_recovery_matches_zero_one_and_many_textual_tags() {
    let source = "BEGIN|[https://example.com %%tags_linked%%]|END";
    for (tags, expected) in [
        (
            &[][..],
            r#"<p>BEGIN|<a href="https://example.com"></a>|END</p>"#,
        ),
        (
            &["component"][..],
            concat!(
                r#"<p>BEGIN|<a href="https://example.com">"#,
                "[/system:page-tags/tag/component component</a>]|END</p>",
            ),
        ),
        (
            &["component", "featured"][..],
            concat!(
                r#"<p>BEGIN|<a href="https://example.com">"#,
                "[/system:page-tags/tag/component component</a> ",
                "[/system:page-tags/tag/featured featured]]|END</p>",
            ),
        ),
    ] {
        assert_eq!(
            render_tag_values(source, tags, " "),
            expected,
            "tag cardinality: {}",
            tags.len(),
        );
    }
}

#[test]
fn delayed_image_attributes_preserve_authored_link_and_suffix() {
    let linked =
        render_tag(r#"[[image x.png alt="%%tags_linked%%" link="https://example.com"]]"#);
    assert!(
        linked.contains(r#"<a href="https://example.com"><img "#),
        "authored image link must survive delayed alt binding: {linked}",
    );

    let suffixed = render_tag(r#"[[image x.png alt="%%tags_linked%% suffix"]]"#);
    assert!(
        suffixed.contains(r#"alt="[/system:page-tags/tag/component component] suffix""#,),
        "authored attribute suffix must survive delayed binding: {suffixed}",
    );
}

#[test]
fn delayed_raw_decodes_entities_and_div_shell_keeps_the_parsed_opener() {
    assert_eq!(
        render("@@&amp; %%title_linked%%@@"),
        concat!(
            r#"<p><span style="white-space: pre-wrap;">&amp; "#,
            "[[[component:image-block | Standard Image Block]]]",
            "</span></p>",
        ),
    );

    let div = render(r#"[[div title="[[probe"]]%%title_linked%%[[/div]]"#);
    assert!(
        div.contains(r#"[[div title=&quot;[[probe&quot;]]"#),
        "delayed div shell must start at its parsed opener: {div}",
    );
}

#[test]
fn delayed_code_shell_keeps_the_parsed_opener() {
    assert_eq!(
        render(concat!(
            "BEGIN|[[code type=\"[[probe\"]]\n",
            "%%title_linked%%\n",
            "[[/code]]|END",
        )),
        concat!(
            "<p>BEGIN|[[code type=&quot;[[probe&quot;]]<br>\n",
            "<a href=\"/component:image-block\">Standard Image Block</a>",
            "<br>\n[[/code]]|END</p>",
        ),
    );
}

#[test]
fn delayed_raw_recovery_owns_uninterpreted_heads() {
    let source = concat!(
        "BEGIN|[[raw probe=\"%%title_linked%%\"]]\n",
        "**AUTHORED**\n",
        "[[/raw]]|END",
    );
    assert_eq!(
        render(source),
        concat!(
            "<p>BEGIN|[[raw probe=&quot;",
            "<a href=\"/component:image-block\">Standard Image Block</a>",
            "&quot;]]<br>\n**AUTHORED**<br>\n[[/raw]]|END</p>",
        ),
    );

    let source = concat!(
        "BEGIN|[[raw malformed=\"unterminated]]\n",
        "**AUTHORED** %%title_linked%%\n",
        "[[/raw]]|END",
    );
    assert_eq!(
        render(source),
        concat!(
            "<p>BEGIN|[[raw malformed=&quot;unterminated]]<br>\n",
            "**AUTHORED** ",
            "<a href=\"/component:image-block\">Standard Image Block</a>",
            "<br>\n[[/raw]]|END</p>",
        ),
    );
}

#[test]
fn recovery_projections_preserve_complete_page_references() {
    let bindings = page_bindings_for(
        PageRef::page_and_site("Other Wiki", "Target Page#toc2"),
        "Remote Target",
    );

    for (source, expected) in [
        (
            "@@%%title_linked%%@@",
            concat!(
                r#"<p><span style="white-space: pre-wrap;">"#,
                "[[[:other-wiki:target-page#toc2 | Remote Target]]]",
                "</span></p>",
            ),
        ),
        (
            "[[#if true | %%title_linked%% | FALLBACK]]",
            "<p>[[[:other-wiki:target-page#toc2] | FALLBACK]]</p>",
        ),
    ] {
        let input = page_link_input(source);
        assert_eq!(
            render_input_with_bindings(&input, &bindings),
            expected,
            "source: {source}",
        );
    }
}

#[test]
fn recovery_source_decodes_semicolon_entities_once() {
    assert_eq!(
        render(concat!(
            "[[code]]\n",
            "&amp; %%title_linked%%\n",
            "[[/code]]",
        )),
        concat!(
            "<p>[[code]]<br>\n&amp; ",
            "<a href=\"/component:image-block\">Standard Image Block</a>",
            "<br>\n[[/code]]</p>",
        ),
    );
    assert_eq!(
        render("[[#if true | %%title_linked%% | A &amp; B]]"),
        "<p>[[[component:image-block] | A &amp; B]]</p>",
    );
}

#[test]
fn parser_recovery_diagnostics_do_not_bypass_atomic_slot_binding() {
    let mut observed_recovered_parse = false;
    for source in [
        "BEGIN|[[[scp-003|%%title_linked%%]]]|END",
        "BEGIN|[https://example.com %%title_linked%%]|END",
        "BEGIN|[[image https://example.com/x.png alt=\"%%title_linked%%\"]]|END",
        "BEGIN|[[#if true | %%title_linked%% | NO]]|END",
    ] {
        let input = page_link_input(source);
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::List, Layout::Wikidot);
        let delayed = parse_delayed_list(&input, &page_info, &settings)
            .expect("supported delayed input");
        if !delayed.errors().is_empty() {
            observed_recovered_parse = true;
            delayed
                .bind(&page_bindings())
                .expect("closed recovery owner still binds atomically");
        }
    }
    assert!(
        observed_recovered_parse,
        "the policy fixture should exercise an observable parser recovery",
    );
}

#[test]
fn delayed_slots_preserve_the_frozen_listpages_owner_boundaries() {
    for (source, expected) in [
        (
            "BEGIN|{{{%%title_linked%%}}}|END",
            concat!(
                "<p>BEGIN|<tt>{",
                "<a href=\"/component:image-block\">Standard Image Block</a>",
                "}</tt>|END</p>",
            ),
        ),
        (
            "BEGIN|[[code]]\n%%title_linked%%\n[[/code]]|END",
            concat!(
                "<p>BEGIN|[[code]]<br>\n",
                "<a href=\"/component:image-block\">Standard Image Block</a>",
                "<br>\n[[/code]]|END</p>",
            ),
        ),
        (
            "BEGIN|[[raw]]\n%%title_linked%%\n[[/raw]]|END",
            concat!(
                "<p>BEGIN|[[raw]]<br>\n",
                "<a href=\"/component:image-block\">Standard Image Block</a>",
                "<br>\n[[/raw]]|END</p>",
            ),
        ),
        (
            "BEGIN|[[div class=\"probe\"]]%%title_linked%%[[/div]]|END",
            concat!(
                "<p>BEGIN|[[div class=&quot;probe&quot;]]",
                "<a href=\"/component:image-block\">Standard Image Block</a>",
                "[[/div]]|END</p>",
            ),
        ),
        (
            "BEGIN|[[[%%title_linked%%|LABEL]]]|END",
            concat!(
                "<p>BEGIN|[[[",
                "<a href=\"/component:image-block\">Standard Image Block</a>",
                "|LABEL]]]|END</p>",
            ),
        ),
        (
            "BEGIN|[%%title_linked%% LABEL]|END",
            concat!(
                "<p>BEGIN|[",
                "<a href=\"/component:image-block\">Standard Image Block</a>",
                " LABEL]|END</p>",
            ),
        ),
        (
            "BEGIN|[[#if true | %%title_linked%% | NO]]|END",
            "<p>BEGIN|[[[component:image-block] | NO]]|END</p>",
        ),
    ] {
        assert_eq!(render(source), expected, "source: {source}");
    }

    for (source, expected) in [
        (
            "BEGIN|[[raw]]\n%%tags_linked%%\n[[/raw]]|END",
            concat!(
                "<p>BEGIN|[[raw]]<br>\n",
                "<a href=\"/system:page-tags/tag/component\">component</a>",
                "<br>\n[[/raw]]|END</p>",
            ),
        ),
        (
            "BEGIN|[https://example.com %%tags_linked%%]|END",
            concat!(
                "<p>BEGIN|<a href=\"https://example.com\">",
                "[/system:page-tags/tag/component component",
                "</a>]|END</p>",
            ),
        ),
        (
            "BEGIN|[[image https://example.com/x.png link=\"%%tags_linked%%\"]]|END",
            concat!(
                "\n\nBEGIN|<a href=\"/[/system:page-tags/tag/component%20component]\">",
                "<img src=\"https://example.com/x.png\" class=\"image\" alt=\"x.png\">",
                "</a>|END",
            ),
        ),
        (
            "BEGIN|[[#if true | %%tags_linked%% | NO]]|END",
            concat!(
                "<p>BEGIN|",
                "<a href=\"/system:page-tags/tag/component\">component</a>",
                "|END</p>",
            ),
        ),
    ] {
        assert_eq!(render_tag(source), expected, "source: {source}");
    }
}
