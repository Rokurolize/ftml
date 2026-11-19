use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::io::{self, Read};

fn main() {
    let mut source = String::new();
    io::stdin().read_to_string(&mut source).unwrap();
    let layout = Layout::Wikidot;
    let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
    ftml::preprocess_for_layout(&mut source, layout);
    let page_info = PageInfo {
        page: Cow::Borrowed("probe"),
        category: None,
        site: Cow::Borrowed("sandbox-for-codex"),
        title: Cow::Borrowed("probe"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: Vec::new(),
        language: Cow::Borrowed("en"),
    };
    let tokenization = ftml::tokenize(&source);
    let (tree, _errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    println!("{:#?}", tree.elements);
}
