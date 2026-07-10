use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use ftml::tree::Element;
use std::borrow::Cow;
use std::time::{Duration, Instant};

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("inline-delimiter-performance"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("coverage"),
        title: Cow::Borrowed("Inline delimiter performance"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![Cow::Borrowed("test")],
        language: Cow::Borrowed("en"),
    }
}

#[test]
fn padded_inline_openers_inside_list_items_stay_literal_in_bounded_time() {
    const ROW_COUNT: usize = 128;

    for marker in ["**", "//", "__", "^^", ",,"] {
        let row = format!("# [[size 0%]]{marker} [[/size]]\n");
        let input = row.repeat(ROW_COUNT);
        let page_info = page_info();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        let started = Instant::now();
        let tokenization = ftml::tokenize(&input);
        let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        let elapsed = started.elapsed();

        assert!(elapsed < Duration::from_secs(5), "{marker:?}: {elapsed:?}");
        assert_eq!(tree.elements.len(), 1, "{marker:?}: {:#?}", tree.elements);
        let Element::List { items, .. } = &tree.elements[0] else {
            panic!("{marker:?}: expected one list, got {:#?}", tree.elements);
        };
        assert_eq!(items.len(), ROW_COUNT, "{marker:?}");
        assert_eq!(html.matches(marker).count(), ROW_COUNT, "{marker:?}");
        assert!(errors.is_empty(), "{marker:?}: {errors:#?}");
    }
}
