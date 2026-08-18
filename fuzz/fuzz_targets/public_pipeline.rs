#![no_main]

use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::Render;
use ftml::render::html::HtmlRender;
use ftml::render::text::TextRender;
use ftml::settings::{WikitextMode, WikitextSettings};
use libfuzzer_sys::fuzz_target;
use std::borrow::Cow;

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("fuzz"),
        category: None,
        site: Cow::Borrowed("fuzz"),
        title: Cow::Borrowed("Fuzz"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    }
}

fn exercise(source: &str, layout: Layout) {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
    let mut preprocessed = source.to_owned();

    ftml::preprocess_for_layout(&mut preprocessed, layout);
    let tokenization = ftml::tokenize(&preprocessed);
    let (tree, _errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    let _html = HtmlRender.render(&tree, &page_info, &settings);
    let _text = TextRender.render(&tree, &page_info, &settings);
}

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };

    exercise(source, Layout::Wikidot);
    exercise(source, Layout::Wikijump);
});
