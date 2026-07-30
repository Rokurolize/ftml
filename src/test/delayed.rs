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
    SlotBindings::new(vec![(
        SlotId::new(1),
        GeneratedValue::PageLink {
            page: PageRef::page_only("component:image-block"),
            label: Cow::Borrowed("Standard Image Block"),
        },
    )])
    .expect("unique bindings")
}

fn render(source: &str) -> String {
    let input = page_link_input(source);
    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::List, Layout::Wikidot);
    let delayed = parse_delayed_list(&input, &page_info, &settings)
        .expect("supported delayed input");
    let bound = delayed.bind(&page_bindings()).expect("matching bindings");
    bound.render_html(&page_info, &settings).body().to_owned()
}

fn render_tag(source: &str) -> String {
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
            tags: Cow::Owned(vec![ResolvedTagRef {
                tag: Cow::Borrowed("component"),
            }]),
            separator: Cow::Borrowed(" "),
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
