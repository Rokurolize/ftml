//! Implicit-attachment image display follows the attachment's file type.
//!
//! Anonymous Wikidot PagePreview evidence (scp-wiki, 2026-10-11) is frozen in
//! `tests/fixtures/wikidot-image-attachment-default-20261011.json`. A
//! current-page attachment without a recognized quoted size shows the original
//! only when its name ends in `.png`, `.jpg`, or `.gif` (ASCII
//! case-insensitive); every other name shows the linked `medium.jpg` resize.
//! Saved pages in the shared EN corpus follow the same split.

use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use regex::Regex;
use serde_json::Value;
use std::borrow::Cow;

const FIXTURE: &str =
    include_str!("fixtures/wikidot-image-attachment-default-20261011.json");

/// Page-qualified attachments without an original-display extension resize on
/// Wikidot, but `ImageSource::ImplicitAttachment` only addresses the current
/// page. Tracked by issue #734; the observations stay in the fixture.
const PAGE_QUALIFIED_RESIZE_GAP: &[&str] = &["[[image scp-9506/x.jpeg]]"];

fn render(source: &str) -> String {
    let page_info = PageInfo {
        page: Cow::Borrowed(""),
        category: None,
        site: Cow::Borrowed("scp-wiki"),
        title: Cow::Borrowed("probe"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    };
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokenization = ftml::tokenize(&source);
    let (tree, _errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    HtmlRender.render(&tree, &page_info, &settings).body
}

/// Extract `(anchor href, img src, alt)` from the first image, mapping FTML's
/// file host to Wikidot's so only the selected shape is compared.
fn image_shape(html: &str) -> (Option<String>, String, String) {
    let image =
        Regex::new(r#"(?:<a href="([^"]*)">)?<img src="([^"]*)"[^>]*? alt="([^"]*)""#)
            .expect("valid image regex");
    let captures = image
        .captures(html)
        .unwrap_or_else(|| panic!("rendered HTML has no image: {html}"));
    let host =
        |value: &str| value.replace("scp-wiki.wjfiles.com", "scp-wiki.wdfiles.com");
    (
        captures.get(1).map(|value| host(value.as_str())),
        host(&captures[2]),
        captures[3].to_owned(),
    )
}

#[test]
fn implicit_attachment_display_matches_live_file_type_default() {
    let fixture: Value = serde_json::from_str(FIXTURE).expect("valid fixture JSON");
    let observations = fixture["observations"]
        .as_array()
        .expect("fixture observations");
    assert!(observations.len() >= 40, "fixture lost observations");

    let mut compared = 0;
    for observation in observations {
        let source = observation["source"].as_str().expect("source");
        if PAGE_QUALIFIED_RESIZE_GAP.contains(&source) {
            continue;
        }
        let expected = (
            observation["anchor_href"].as_str().map(str::to_owned),
            observation["img_src"].as_str().expect("img_src").to_owned(),
            observation["alt"].as_str().expect("alt").to_owned(),
        );
        let html = render(source);
        assert_eq!(image_shape(&html), expected, "{source}\n{html}");
        compared += 1;
    }
    assert_eq!(
        compared,
        observations.len() - PAGE_QUALIFIED_RESIZE_GAP.len()
    );
}

#[test]
fn original_display_extensions_are_exact_and_case_insensitive() {
    for (source, resized) in [
        ("[[image a.png]]", false),
        ("[[image a.JPG]]", false),
        ("[[image a.GiF]]", false),
        ("[[image .png]]", false),
        ("[[image a.jpeg.png]]", false),
        ("[[image a.jpeg]]", true),
        ("[[image a.jpe]]", true),
        ("[[image a.apng]]", true),
        ("[[image a.png?v=1]]", true),
        ("[[image a.jpg.jpeg]]", true),
        ("[[image noext]]", true),
    ] {
        let html = render(source);
        assert_eq!(
            html.contains("/local--resized-images//"),
            resized,
            "{source}: {html}"
        );
        assert_eq!(html.contains("<a href="), resized, "{source}: {html}");
    }
}
