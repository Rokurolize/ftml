use super::context::HtmlContext;
use crate::data::PageInfo;
use crate::layout::Layout;
use crate::render::Handle;
use crate::settings::{WikitextMode, WikitextSettings};
use crate::tree::{BibliographyList, Element};
use std::sync::LazyLock;

pub(super) fn context<'a>(
    info: &'a PageInfo<'a>,
) -> HtmlContext<'a, 'static, 'static, 'static> {
    static HANDLE: Handle = Handle;
    static SETTINGS: LazyLock<WikitextSettings> = LazyLock::new(|| {
        WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump)
    });
    static ELEMENTS: [Element<'static>; 0] = [];
    static FOOTNOTES: [Vec<Element<'static>>; 0] = [];
    static BIBLIOGRAPHIES: LazyLock<BibliographyList<'static>> =
        LazyLock::new(BibliographyList::new);

    HtmlContext::new(
        info,
        &HANDLE,
        &SETTINGS,
        &ELEMENTS,
        &FOOTNOTES,
        &BIBLIOGRAPHIES,
        0,
    )
}
