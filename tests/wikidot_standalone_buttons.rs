use ftml::data::{PageInfo, PageRef, ScoreValue};
use ftml::delayed::{
    DelayedInput, GeneratedInput, GeneratedKind, GeneratedValue, InputSegment,
    SlotBindings, SlotId, TextOrigin, parse_delayed_list,
};
use ftml::layout::Layout;
use ftml::parsing::ParseErrorKind;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use ftml::tree::{
    Element, StandaloneButton, StandaloneButtonAction, SyntaxTree, TagAlteration,
};
use serde_json::Value;
use std::borrow::Cow;
use std::collections::HashSet;
use std::time::{Duration, Instant};

const LIVE_EVIDENCE: &str =
    include_str!("fixtures/wikidot-standalone-buttons-live-20260730.json");

fn render(source: &str, layout: Layout) -> ftml::render::html::HtmlOutput {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
    let tokenization = ftml::tokenize(source);
    let (tree, _errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    HtmlRender.render(&tree, &page_info, &settings)
}

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("standalone-button-evidence"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("scp-wiki"),
        title: Cow::Borrowed("Standalone button evidence"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    }
}

#[test]
fn all_42_focused_cases_preserve_the_action_boundary() {
    let fixture: Value =
        serde_json::from_str(LIVE_EVIDENCE).expect("valid evidence fixture");
    assert_eq!(
        fixture["schema"],
        "ftml.wikidot-standalone-button-evidence.v1"
    );
    assert_eq!(
        fixture["provenance"]["requirements_sha256"],
        "45717e5351f7eb1c46431dd44bf15db9777dbbc4fa40026931bd2e6458b2fcc9"
    );
    assert_eq!(
        fixture["provenance"]["wikidot_py_commit"],
        "2434bf77744488cb2095327c9e0e4450add78df3"
    );

    let live_cases = fixture["cases"].as_array().expect("case array");
    assert_eq!(live_cases.len(), 36);
    let inactive_live: HashSet<&str> = [
        "focused-button-basic",
        "focused-button-code",
        "focused-button-corpus-1",
        "focused-button-corpus-4",
        "focused-button-unclosed",
        "focused-button-unknown-action",
    ]
    .into_iter()
    .collect();

    for case in live_cases {
        let id = case["id"].as_str().expect("case id");
        let source = case["source"].as_str().expect("case source");
        let raw_html_sha256 = case["raw_html_sha256"].as_str().expect("reference hash");
        assert_eq!(raw_html_sha256.len(), 64, "{id}: provenance hash");

        let output = render(source, Layout::Wikidot);
        let requirement_count = output
            .resource_requirements
            .iter()
            .filter(|item| item.standalone_button_requirement().is_some())
            .count();
        assert_eq!(
            requirement_count,
            usize::from(!inactive_live.contains(id)),
            "{id}: {source:?} rendered as {}",
            output.body
        );
    }

    let synthetic_controls = [
        ("raw-owner", "@@[[button edit]]@@", false),
        ("comment-owner", "[!-- [[button edit]] --]", false),
        (
            "html-owner",
            "[[html]]<span>[[button edit]]</span>[[/html]]",
            false,
        ),
        ("crlf", "before\r\n[[button edit]]", true),
        ("unicode", "[[button edit text=\"編集する🛠\"]]", true),
        ("multiline-malformed", "[[button edit\ntext=\"X\"]]", false),
    ];
    assert_eq!(live_cases.len() + synthetic_controls.len(), 42);
    for (id, source, active) in synthetic_controls {
        let output = render(source, Layout::Wikidot);
        assert_eq!(
            output
                .resource_requirements
                .iter()
                .filter(|item| item.standalone_button_requirement().is_some())
                .count(),
            usize::from(active),
            "{id}: {}",
            output.body
        );
    }
}

#[test]
fn typed_descriptors_keep_ordered_tag_data_and_no_authored_script() {
    let output = render(
        r#"[[button set-tags -* +favorite +_book -_movie text="Change & <tags>" onclick="alert(1)"]]"#,
        Layout::Wikidot,
    );
    let [requirement] = output.resource_requirements.as_slice() else {
        panic!(
            "expected one requirement: {:#?}",
            output.resource_requirements
        );
    };
    let requirement = requirement
        .standalone_button_requirement()
        .expect("standalone button requirement");
    assert!(requirement.id().starts_with("wj-button-"));
    assert_eq!(
        requirement.action(),
        &StandaloneButtonAction::SetTags(vec![
            TagAlteration::ClearVisible,
            TagAlteration::Add(Cow::Borrowed("favorite")),
            TagAlteration::Add(Cow::Borrowed("_book")),
            TagAlteration::Remove(Cow::Borrowed("_movie")),
        ])
    );
    assert!(output.body.contains("Change &amp; &lt;tags&gt;"));
    assert!(output.body.contains(
        "onclick=\"WIKIDOT.page.listeners.updateTagsByButton(event, &#39;-* +favorite +_book -_movie&#39;)\""
    ));
    assert!(!output.body.contains("alert(1)"));

    let serialized = serde_json::to_value(requirement).expect("serializable requirement");
    assert_eq!(serialized["action"]["type"], "set-tags");
    assert_eq!(
        serialized["action"]["data"][0]["operation"],
        "clear-visible"
    );
}

#[test]
fn legacy_and_wikijump_layouts_emit_static_controls_with_typed_hooks() {
    let legacy = render(
        r#"[[button print class="print-control" style="color:red" text="Print"]]"#,
        Layout::Wikidot,
    );
    let legacy_id = legacy.resource_requirements[0]
        .standalone_button_requirement()
        .unwrap()
        .id();
    assert!(!legacy.body.contains(legacy_id));
    assert_eq!(
        legacy.body,
        r#"<p><a class="print-control" href="javascript:;" onclick="WIKIDOT.page.listeners.printClick(event)" style="color:red">Print</a></p>"#
    );

    let wikijump = render(
        r#"[[button edit class="primary" text="Edit"]]"#,
        Layout::Wikijump,
    );
    let wikijump_id = wikijump.resource_requirements[0]
        .standalone_button_requirement()
        .unwrap()
        .id();
    assert_eq!(
        wikijump.body.replace(wikijump_id, "BUTTON_ID"),
        r#"<p><button type="button" class="wj-standalone-button primary" id="BUTTON_ID">Edit</button></p>"#
    );
}

#[test]
fn wikidot_layout_emits_live_standalone_button_handlers() {
    for (source, handler) in [
        ("[[button edit]]", "WIKIDOT.page.listeners.editClick(event)"),
        (
            "[[button history]]",
            "WIKIDOT.page.listeners.historyClick(event)",
        ),
        (
            "[[button source]]",
            "WIKIDOT.page.listeners.viewSourceClick(event)",
        ),
        (
            "[[button print]]",
            "WIKIDOT.page.listeners.printClick(event)",
        ),
        (
            r#"[[button set-tags +probe text="Set"]]"#,
            "WIKIDOT.page.listeners.updateTagsByButton(event, &#39;+probe&#39;)",
        ),
    ] {
        let output = render(source, Layout::Wikidot);
        assert!(
            output.body.contains(&format!(r#" onclick="{handler}""#)),
            "{source:?}: {}",
            output.body
        );
        assert!(!output.body.contains(" id="), "{source:?}: {}", output.body);
    }
}

#[test]
fn wikidot_set_tags_handler_preserves_live_backslash_but_escapes_other_string_data() {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let tree = SyntaxTree {
        elements: vec![Element::StandaloneButton(StandaloneButton {
            action: StandaloneButtonAction::SetTags(vec![
                TagAlteration::Add(Cow::Borrowed("quote'")),
                TagAlteration::Add(Cow::Borrowed("slash\\")),
                TagAlteration::Add(Cow::Borrowed("line\u{2028}break")),
            ]),
            label: Cow::Borrowed("Set"),
            class: None,
            style: None,
        })],
        ..SyntaxTree::default()
    };
    let output = HtmlRender.render(&tree, &page_info, &settings);
    assert!(output.body.contains(
        r#"onclick="WIKIDOT.page.listeners.updateTagsByButton(event, &#39;+quote\x27 +slash\ +line\u2028break&#39;)""#
    ));
}

#[test]
fn wikidot_set_tags_rejects_apostrophe_tag_without_changing_wikijump() {
    let source = r#"[[button set-tags +quote' text="Set"]]"#;
    assert_eq!(
        render(source, Layout::Wikidot).body,
        r#"<div class="error-block">You need to set text for set-tags button.</div>"#
    );

    let wikijump = render(source, Layout::Wikijump);
    assert!(wikijump.body.contains(">Set</button>"), "{}", wikijump.body);
    assert_eq!(
        wikijump.resource_requirements[0]
            .standalone_button_requirement()
            .unwrap()
            .action(),
        &StandaloneButtonAction::SetTags(vec![TagAlteration::Add(Cow::Borrowed(
            "quote'"
        ))])
    );
}

#[test]
fn wikidot_set_tags_matches_live_delimiter_and_separator_boundaries() {
    let error =
        r#"<div class="error-block">You need to set text for set-tags button.</div>"#;
    for source in [
        r#"[[button set-tags +quote" text="Set"]]"#,
        r#"[[button set-tags +a=b text="Set"]]"#,
        r#"[[button set-tags +a&b text="Set"]]"#,
        r#"[[button set-tags +a<b text="Set"]]"#,
        r#"[[button set-tags +a>b text="Set"]]"#,
    ] {
        assert_eq!(render(source, Layout::Wikidot).body, error, "{source:?}");
    }

    for (source, alterations) in [
        ("[[button set-tags +a\u{000b}+b text=\"Set\"]]", "+a +b"),
        ("[[button set-tags +a\u{000c}+b text=\"Set\"]]", "+a +b"),
        ("[[button set-tags +a\ntext=\"Set\"]]", "+a"),
        ("[[button set-tags +a\rtext=\"Set\"]]", "+a"),
        ("[[button set-tags +a\0b text=\"Set\"]]", "+ab"),
        (
            "[[button set-tags +a.b:c/d@e#f+g*h +\u{65e5}\u{672c}\u{8a9e} text=\"Set\"]]",
            "+a.b:c/d@e#f+g*h +\u{65e5}\u{672c}\u{8a9e}",
        ),
    ] {
        let output = render(source, Layout::Wikidot);
        assert!(
            output.body.contains(&format!(
                "updateTagsByButton(event, &#39;{alterations}&#39;)"
            )),
            "{source:?}: {}",
            output.body
        );
    }

    assert!(
        render(r#"[[button set-tags +a=b text="Set"]]"#, Layout::Wikijump)
            .body
            .contains(">Set</button>")
    );
}

#[test]
fn malformed_and_unsupported_forms_preserve_exact_residual_text() {
    assert_eq!(
        render("[[button]]", Layout::Wikidot).body,
        "<p>[[button]]</p>"
    );
    assert_eq!(
        render("[[button edit", Layout::Wikidot).body,
        "<p>[[button edit</p>"
    );
    let trailing = render("[[button edit]]]", Layout::Wikidot);
    let trailing_id = trailing.resource_requirements[0]
        .standalone_button_requirement()
        .unwrap()
        .id();
    assert!(!trailing.body.contains(trailing_id));
    assert_eq!(
        trailing.body,
        r#"<p><a class="wiki-standalone-button" href="javascript:;" onclick="WIKIDOT.page.listeners.editClick(event)">edit</a>]</p>"#
    );
    assert_eq!(
        render(r#"[[button unknown text="X"]]"#, Layout::Wikidot).body,
        r#"<div class="error-block"><em>unknown</em> is not a valid button type</div>"#
    );
    assert_eq!(
        render(
            r#"[[button set-tags <tag_alterations> text="<button_text>"]]"#,
            Layout::Wikidot
        )
        .body,
        r#"<div class="error-block">You need to set text for set-tags button.</div>"#
    );
}

#[test]
fn malformed_forms_report_stable_recovery_kinds() {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    for (source, expected) in [
        ("[[button]]", ParseErrorKind::BlockMissingArguments),
        ("[[button edit", ParseErrorKind::EndOfInput),
        (
            "[[button edit\ntext=\"X\"]]",
            ParseErrorKind::BlockMalformedArguments,
        ),
    ] {
        let tokenization = ftml::tokenize(source);
        let outcome = ftml::parse(&tokenization, &page_info, &settings);
        assert!(
            outcome
                .errors()
                .iter()
                .any(|error| error.kind() == expected),
            "{source:?}: {:#?}",
            outcome.errors()
        );
    }
}

#[test]
fn unsafe_style_is_dropped_while_safe_evidenced_style_is_preserved() {
    let unsafe_output = render(
        r#"[[button print style="background:url(javascript:alert(1))"]]"#,
        Layout::Wikidot,
    );
    assert!(!unsafe_output.body.contains("style="));
    assert!(!unsafe_output.body.contains("alert"));

    let safe_output = render(
        r#"[[button source style="background-image: url(http://www.wikidot.com/local--files/files/view-source.png); background-repeat: no-repeat; background-position: bottom right; padding-right: 20px; color: #444"]]"#,
        Layout::Wikidot,
    );
    assert!(safe_output.body.contains("background-image: url(http://"));

    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let tree = SyntaxTree {
        elements: vec![Element::StandaloneButton(StandaloneButton {
            action: StandaloneButtonAction::Print,
            label: Cow::Borrowed("print"),
            class: None,
            style: Some(Cow::Borrowed("background:url(javascript:alert(1))")),
        })],
        ..SyntaxTree::default()
    };
    let direct_output = HtmlRender.render(&tree, &page_info, &settings);
    assert!(!direct_output.body.contains("style="));
    assert!(!direct_output.body.contains("alert"));
}

#[test]
fn generated_list_values_cannot_forge_a_button_action() {
    let source = "[[button %%title_linked%%]]";
    let marker = "%%title_linked%%";
    let start = source.find(marker).unwrap();
    let end = start + marker.len();
    let input = DelayedInput::new(
        source,
        vec![
            InputSegment::text(0..start, TextOrigin::Authored),
            InputSegment::generated(GeneratedInput {
                source_range: start..end,
                id: SlotId::new(331),
                kind: GeneratedKind::PageLink,
                occurrence: 0,
            }),
            InputSegment::text(end..source.len(), TextOrigin::Authored),
        ],
    )
    .expect("valid delayed input");
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::List, Layout::Wikidot);
    let delayed =
        parse_delayed_list(&input, &page_info, &settings).expect("parse delayed");
    let bindings = SlotBindings::new(vec![(
        SlotId::new(331),
        GeneratedValue::PageLink {
            page: PageRef::page_only("edit"),
            label: Cow::Borrowed("edit"),
        },
    )])
    .expect("valid binding");
    let bound = delayed.bind(&bindings).expect("bind generated value");
    let fragment = bound.render_html(&page_info, &settings);
    assert!(!fragment.body().contains("wiki-standalone-button"));
    assert!(fragment.body().contains("[[button"));
}

#[test]
fn runtime_include_and_parser_values_cannot_forge_button_syntax_or_data() {
    for (source, runtime) in [
        ("[[button edit]]", "edit"),
        (r#"[[button edit text="Injected"]]"#, "Injected"),
        ("prefix [[button print]] suffix", "[[button print]]"),
    ] {
        let start = source.find(runtime).expect("runtime fragment");
        let end = start + runtime.len();
        let input = DelayedInput::new(
            source,
            vec![
                InputSegment::text(0..start, TextOrigin::Authored),
                InputSegment::text(start..end, TextOrigin::RuntimeScalar),
                InputSegment::text(end..source.len(), TextOrigin::Authored),
            ],
        )
        .expect("valid runtime-scalar input");
        let page_info = page_info();
        let settings = WikitextSettings::from_mode(WikitextMode::List, Layout::Wikidot);
        let delayed = parse_delayed_list(&input, &page_info, &settings)
            .expect("parse delayed input");
        let bound = delayed
            .bind(&SlotBindings::empty())
            .expect("runtime scalars require no binding");
        let fragment = bound.render_html(&page_info, &settings);
        assert!(!fragment.body().contains("wiki-standalone-button"));
        assert!(fragment.body().contains("button"), "{}", fragment.body());
    }
}

#[test]
fn very_long_button_heads_fail_closed_in_bounded_time() {
    let source = format!("[[button edit text=\"{}\"]]", "x".repeat(256 * 1024));
    let started = Instant::now();
    let output = render(&source, Layout::Wikidot);
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(output.resource_requirements.is_empty());
    assert!(output.body.contains("[[button edit"));
}
