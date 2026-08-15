use ftml::data::{PageInfo, PageRef, ScoreValue};
use ftml::delayed::{
    DelayedInput, GeneratedInput, GeneratedKind, GeneratedValue, InputSegment,
    SlotBindings, SlotId, TextOrigin, parse_delayed_list,
};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender, text::TextRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use ftml::tree::{Element, GalleryEntrySource, GallerySelection, SyntaxTree};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::HashSet;
use std::time::{Duration, Instant};

const LIVE_EVIDENCE: &str = include_str!("fixtures/wikidot-gallery-live-20260807.json");

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("gallery-evidence"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("scp-wiki"),
        title: Cow::Borrowed("Gallery evidence"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    }
}

fn settings(mode: WikitextMode) -> WikitextSettings {
    let mut settings = WikitextSettings::from_mode(mode, Layout::Wikidot);
    settings.enable_html_blocks = false;
    settings
}

fn parse(source: &str) -> (SyntaxTree<'static>, Vec<ftml::parsing::ParseError>) {
    let page_info = page_info();
    let settings = settings(WikitextMode::Page);
    let tokenization = ftml::tokenize(source);
    let (tree, errors): (SyntaxTree<'_>, _) =
        ftml::parse(&tokenization, &page_info, &settings).into();
    (tree.to_owned(), errors)
}

fn render(source: &str) -> ftml::render::html::HtmlOutput {
    let page_info = page_info();
    let settings = settings(WikitextMode::Page);
    let tokenization = ftml::tokenize(source);
    let (tree, _errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    HtmlRender.render(&tree, &page_info, &settings)
}

fn render_text(source: &str) -> String {
    let page_info = page_info();
    let settings = settings(WikitextMode::Page);
    let tokenization = ftml::tokenize(source);
    let (tree, _errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    TextRender.render(&tree, &page_info, &settings)
}

fn gallery_requirements(
    output: &ftml::render::html::HtmlOutput,
) -> Vec<&ftml::render::html::GalleryRequirement> {
    output
        .resource_requirements
        .iter()
        .filter_map(|requirement| requirement.gallery_requirement())
        .collect()
}

fn sha256_hex(source: &str) -> String {
    use std::fmt::Write;

    Sha256::digest(source.as_bytes()).iter().fold(
        String::with_capacity(64),
        |mut output, byte| {
            write!(&mut output, "{byte:02x}").expect("write to String");
            output
        },
    )
}

#[test]
fn all_35_live_backed_cases_preserve_gallery_ownership() {
    let fixture: Value = serde_json::from_str(LIVE_EVIDENCE).expect("valid evidence");
    assert_eq!(fixture["schema"], "ftml.wikidot-gallery-evidence.v1");
    for capture in ["archived_capture", "supplemental_capture"] {
        assert_eq!(fixture["provenance"][capture]["authenticated"], false);
        assert_eq!(fixture["provenance"][capture]["mutated"], false);
        assert_eq!(
            fixture["provenance"][capture]["module"],
            "edit/PagePreviewModule",
        );
    }

    let cases = fixture["cases"].as_array().expect("case array");
    assert_eq!(cases.len(), 35);
    let inactive: HashSet<&str> = [
        "focused-gallery-code",
        "focused-gallery-comment",
        "focused-gallery-escaped",
        "focused-gallery-heading",
        "focused-gallery-prose",
        "focused-gallery-table",
        "focused-gallery-unclosed",
    ]
    .into_iter()
    .collect();

    for case in cases {
        let id = case["id"].as_str().expect("case id");
        let source = case["source"].as_str().expect("source");
        assert_eq!(
            sha256_hex(source),
            case["source_sha256"].as_str().expect("source hash"),
            "{id}: source provenance",
        );
        assert_eq!(case["raw_html_sha256"].as_str().unwrap().len(), 64, "{id}");

        let output = render(source);
        let requirements = gallery_requirements(&output);
        let expected_count = if inactive.contains(id) {
            0
        } else if id == "focused-gallery-corpus-2-doc-wiki-syntax:images" {
            2
        } else {
            1
        };
        assert_eq!(requirements.len(), expected_count, "{id}: {}", output.body);
        if let Some(requirement) = requirements.first() {
            assert!(requirement.id().starts_with("wj-gallery-"), "{id}");
            assert_eq!(
                requirement.gallery().source_sha256().to_hex(),
                sha256_hex(requirement.gallery().source()),
                "{id}: owner identity",
            );
            assert!(!output.body.contains("<script"), "{id}: {}", output.body);
            assert!(!output.body.contains("lightbox"), "{id}: {}", output.body);
            let explicit = id.contains("breaddddd-art-1");
            assert_eq!(
                matches!(
                    requirement.gallery().selection(),
                    GallerySelection::Explicit(_)
                ),
                explicit,
                "{id}: selection type",
            );
            if let GallerySelection::Explicit(entries) = requirement.gallery().selection()
            {
                assert_eq!(
                    entries.len(),
                    source
                        .lines()
                        .filter(|line| line.trim_start().starts_with(':'))
                        .count(),
                    "{id}"
                );
                assert!(
                    entries.iter().all(|entry| matches!(
                        entry.image(),
                        GalleryEntrySource::HttpUrl(_)
                    )),
                    "{id}"
                );
            }
        }
    }
}

#[test]
fn gallery_head_and_entry_data_remain_ordered_and_inert() {
    let source = concat!(
        "[[gallery size=\"small\" size=\"large\" viewer=\"no\" unknown=\"雪\"]]\r\n",
        ": https://example.com/雪.webp title=\"First\" title=\"Second\"\r\n",
        ": local-image.png caption=\"Local\"\r\n",
        ": javascript:alert(1) caption=\"Unsafe\"\r\n",
        "[[/GALLERY]]",
    );
    let output = render(source);
    let gallery = gallery_requirements(&output)[0].gallery();
    assert_eq!(
        gallery
            .arguments()
            .iter()
            .map(|argument| (argument.name(), argument.value()))
            .collect::<Vec<_>>(),
        [
            ("size", "small"),
            ("size", "large"),
            ("viewer", "no"),
            ("unknown", "雪"),
        ],
    );
    let GallerySelection::Explicit(entries) = gallery.selection() else {
        panic!("expected explicit entries: {gallery:#?}");
    };
    assert_eq!(entries.len(), 3);
    assert!(matches!(entries[0].image(), GalleryEntrySource::HttpUrl(_)));
    assert!(matches!(entries[1].image(), GalleryEntrySource::File(_)));
    assert!(matches!(entries[2].image(), GalleryEntrySource::Inert(_)));
    assert_eq!(
        entries[0]
            .arguments()
            .iter()
            .map(|argument| argument.value())
            .collect::<Vec<_>>(),
        ["First", "Second"],
    );
    assert!(!output.body.contains("example.com"), "{}", output.body);
    assert!(!output.body.contains("javascript:"), "{}", output.body);
    assert_eq!(render_text(source), "");
}

#[test]
fn wikidot_gallery_non_assignment_tail_recovers_as_empty_arguments() {
    let source = "[[gallery stray]]\n[[/gallery]]";
    let output = render(source);
    let requirements = gallery_requirements(&output);
    let [requirement] = requirements.as_slice() else {
        panic!("expected one gallery requirement: {}", output.body);
    };
    assert_eq!(requirement.gallery().source(), "[[gallery stray]]");
    assert!(requirement.gallery().arguments().is_empty());
    assert!(matches!(
        requirement.gallery().selection(),
        GallerySelection::CurrentPageFiles
    ));
    assert!(output.body.contains("[[/gallery]]"), "{}", output.body);

    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
    let tokenization = ftml::tokenize(source);
    let (tree, _errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    let output = HtmlRender.render(&tree, &page_info, &settings);
    assert!(gallery_requirements(&output).is_empty(), "{}", output.body);
    assert!(output.body.contains("[[gallery stray]]"), "{}", output.body);
}

#[test]
fn opener_only_and_first_closer_recovery_preserve_residual_source() {
    let ordinary = render("[[gallery]]\nBODY\n[[/gallery]]\nAFTER");
    assert_eq!(gallery_requirements(&ordinary).len(), 1);
    assert!(ordinary.body.contains("BODY"), "{}", ordinary.body);
    assert!(ordinary.body.contains("[[/gallery]]"), "{}", ordinary.body);
    assert!(ordinary.body.contains("AFTER"), "{}", ordinary.body);

    let source = concat!(
        "[[gallery size=\"thumbnail\"]]\n",
        ": https://example.com/first.png\n",
        "[[/gallery]]]\n",
        ": https://example.com/residual.png\n",
        "[[/gallery]]",
    );
    let output = render(source);
    let gallery = gallery_requirements(&output)[0].gallery();
    let GallerySelection::Explicit(entries) = gallery.selection() else {
        panic!("expected explicit gallery");
    };
    assert_eq!(entries.len(), 1);
    assert!(output.body.contains("]"), "{}", output.body);
    assert!(
        !output.body.contains("residual.png"),
        "the first closer leaves the later empty-term line invisible like Wikidot: {}",
        output.body
    );
    assert!(output.body.contains("[[/gallery]]"), "{}", output.body);

    let extra_opener = render("[[gallery]]]\nAFTER");
    assert_eq!(gallery_requirements(&extra_opener).len(), 1);
    assert!(extra_opener.body.contains("]"), "{}", extra_opener.body);
    assert!(extra_opener.body.contains("AFTER"), "{}", extra_opener.body);
}

#[test]
fn gallery_is_line_owned_and_respects_literal_boundaries() {
    for source in [
        "A[[gallery]]B",
        "+ [[gallery]]",
        "|| [[gallery]] ||",
        "\\[[gallery]]",
        "[!-- [[gallery]] --]",
        "[[code]]\n[[gallery]]\n[[/code]]",
    ] {
        assert!(
            gallery_requirements(&render(source)).is_empty(),
            "{source:?}"
        );
    }

    for source in [
        "[[raw]]\n[[gallery]]\n[[/raw]]",
        "[[html]]\n[[gallery]]\n[[/html]]",
    ] {
        let output = render(source);
        assert_eq!(
            gallery_requirements(&output).len(),
            1,
            "{source:?}: {}",
            output.body
        );
        assert!(output.body.contains("[["), "{source:?}: {}", output.body);
    }
}

#[test]
fn owned_gallery_round_trips_and_rejects_forged_authority() {
    let source = concat!(
        "[[gallery size=\"thumbnail\"]]\r\n",
        ": https://example.com/雪.png title=\"雪\"\r\n",
        "[[/gallery]]",
    );
    let (tree, errors) = parse(source);
    assert!(errors.is_empty(), "{errors:#?}");
    let [Element::Gallery(gallery)] = tree.elements.as_slice() else {
        panic!("expected one Gallery: {:#?}", tree.elements);
    };
    assert_eq!(gallery.source(), source);

    let serialized = serde_json::to_string(&tree).expect("serialize tree");
    let restored: SyntaxTree<'_> =
        serde_json::from_str(&serialized).expect("restore tree");
    assert_eq!(restored.elements, tree.elements);

    let mut forged_hash = serde_json::to_value(&tree).expect("tree value");
    forged_hash["elements"][0]["data"]["source-sha256"] = Value::String("0".repeat(64));
    assert!(serde_json::from_value::<SyntaxTree<'_>>(forged_hash).is_err());

    let mut forged_entry = serde_json::to_value(&tree).expect("tree value");
    forged_entry["elements"][0]["data"]["selection"]["entries"][0]["image"]["source"] =
        Value::String("javascript:alert(1)".to_owned());
    assert!(serde_json::from_value::<SyntaxTree<'_>>(forged_entry).is_err());

    let mut forged_argument = serde_json::to_value(&tree).expect("tree value");
    forged_argument["elements"][0]["data"]["arguments"][0]["value"] =
        Value::String("large".to_owned());
    assert!(serde_json::from_value::<SyntaxTree<'_>>(forged_argument).is_err());
}

fn render_delayed(
    input: &DelayedInput<'_>,
    bindings: &SlotBindings<'_>,
) -> ftml::delayed::SealedFragment {
    let page_info = page_info();
    let settings = settings(WikitextMode::List);
    let delayed = parse_delayed_list(input, &page_info, &settings)
        .expect("supported delayed gallery input");
    let bound = delayed.bind(bindings).expect("matching bindings");
    bound.render_html(&page_info, &settings)
}

#[test]
fn generated_and_runtime_values_cannot_forge_gallery_authority() {
    let authored = concat!(
        "[[gallery size=\"thumbnail\"]]\n",
        ": https://example.com/authored.png title=\"A\"\n",
        "[[/gallery]]",
    );
    let input = DelayedInput::new(
        authored,
        vec![InputSegment::text(0..authored.len(), TextOrigin::Authored)],
    )
    .expect("authored input");
    let fragment = render_delayed(&input, &SlotBindings::empty());
    let gallery = fragment
        .resource_requirements()
        .iter()
        .find_map(|requirement| requirement.gallery_requirement())
        .expect("authored gallery");
    assert!(matches!(
        gallery.gallery().selection(),
        GallerySelection::Explicit(_)
    ));

    let source = concat!(
        "[[gallery size=\"thumbnail\"]]\n",
        ": %%title_linked%% title=\"A\"\n",
        "[[/gallery]]",
    );
    let marker = "%%title_linked%%";
    let start = source.find(marker).unwrap();
    let end = start + marker.len();
    let input = DelayedInput::new(
        source,
        vec![
            InputSegment::text(0..start, TextOrigin::Authored),
            InputSegment::generated(GeneratedInput {
                source_range: start..end,
                id: SlotId::new(333),
                kind: GeneratedKind::PageLink,
                occurrence: 0,
            }),
            InputSegment::text(end..source.len(), TextOrigin::Authored),
        ],
    )
    .expect("generated input");
    let bindings = SlotBindings::new(vec![(
        SlotId::new(333),
        GeneratedValue::PageLink {
            page: PageRef::page_only("generated-gallery"),
            label: Cow::Borrowed("https://example.com/forged.png"),
        },
    )])
    .expect("binding");
    let fragment = render_delayed(&input, &bindings);
    assert!(
        fragment
            .resource_requirements()
            .iter()
            .all(|requirement| requirement.gallery_requirement().is_none()),
        "generated entry data must make the whole explicit owner literal"
    );
    assert!(fragment.body().contains("generated-gallery"));
    assert!(fragment.body().contains("forged.png"));
    assert!(!fragment.body().contains("wj-gallery"));

    for runtime in [
        "[[gallery",
        "https://example.com/runtime.png",
        "title=\"A\"",
        "[[/gallery]]",
    ] {
        let source = concat!(
            "[[gallery size=\"thumbnail\"]]\n",
            ": https://example.com/runtime.png title=\"A\"\n",
            "[[/gallery]]",
        );
        let start = source.find(runtime).unwrap();
        let end = start + runtime.len();
        let input = DelayedInput::new(
            source,
            vec![
                InputSegment::text(0..start, TextOrigin::Authored),
                InputSegment::text(start..end, TextOrigin::RuntimeScalar),
                InputSegment::text(end..source.len(), TextOrigin::Authored),
            ],
        )
        .expect("runtime input");
        let fragment = render_delayed(&input, &SlotBindings::empty());
        let requirements = fragment
            .resource_requirements()
            .iter()
            .filter_map(|requirement| requirement.gallery_requirement())
            .collect::<Vec<_>>();
        assert!(
            requirements.is_empty(),
            "{runtime:?} gained gallery authority"
        );
        assert!(fragment.body().contains("gallery"), "{}", fragment.body());
        assert!(
            !fragment.body().contains("wj-gallery"),
            "{}",
            fragment.body()
        );
    }
}

#[test]
fn malformed_and_dense_gallery_inputs_remain_bounded() {
    for source in [
        "[[gallery",
        "[[gallery malformed]]",
        "[[gallery]]\n: \n[[/gallery]]",
        "[[gallery]]\n: https://example.com/a.png\nNOPE\n[[/gallery]]",
        "[[gallery]]\n: https://example.com/a.png",
    ] {
        let output = render(source);
        assert!(gallery_requirements(&output).len() <= 1, "{source:?}");
    }

    let source = "A[[gallery]]B\n".repeat(4_096);
    let started = Instant::now();
    let output = render(&source);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed(),
    );
    assert!(gallery_requirements(&output).is_empty());

    let mut explicit = String::from("[[gallery size=\"thumbnail\"]]\n");
    for index in 0..2_048 {
        explicit.push_str(&format!(
            ": https://example.com/{index}.png title=\"{index}\"\n"
        ));
    }
    explicit.push_str("[[/gallery]]");
    let started = Instant::now();
    let output = render(&explicit);
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed(),
    );
    let gallery = gallery_requirements(&output)[0].gallery();
    assert!(
        matches!(gallery.selection(), GallerySelection::Explicit(entries) if entries.len() == 2_048)
    );
}
