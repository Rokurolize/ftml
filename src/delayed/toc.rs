//! Deferred table-of-contents labels for generated heading leaves.

use super::elements_contain_delayed;
use crate::tree::{ContainerType, Element, ListItem, PartialElement, SyntaxTree};

#[cfg(feature = "html")]
use crate::data::PageInfo;
#[cfg(feature = "html")]
use crate::render::text::TextRender;
#[cfg(feature = "html")]
use crate::settings::WikitextSettings;
#[cfg(feature = "html")]
use crate::tree::{LinkLabel, LinkType};
#[cfg(feature = "html")]
use std::borrow::Cow;
#[cfg(feature = "html")]
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn entry_indices(tree: &SyntaxTree<'_>) -> Vec<usize> {
    let mut delayed = Vec::new();
    let mut entry_index = 0usize;
    let mut footnote_index = 0usize;
    visit_headings(
        &tree.elements,
        &tree.footnotes,
        &mut footnote_index,
        &mut |elements| {
            if elements_contain_delayed(elements) {
                delayed.push(entry_index);
            }
            entry_index += 1;
        },
    );
    delayed
}

fn visit_headings<'a, 't>(
    elements: &'a [Element<'t>],
    footnotes: &'a [Vec<Element<'t>>],
    footnote_index: &mut usize,
    visitor: &mut impl FnMut(&'a [Element<'t>]),
) {
    for element in elements {
        match element {
            Element::Container(container) => {
                if matches!(
                    container.ctype(),
                    ContainerType::Header(heading) if heading.has_toc
                ) {
                    visitor(container.elements());
                }
                visit_headings(container.elements(), footnotes, footnote_index, visitor);
            }
            Element::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        visit_headings(
                            &cell.elements,
                            footnotes,
                            footnote_index,
                            visitor,
                        );
                    }
                }
            }
            Element::TabView(tabs) => {
                for tab in tabs {
                    visit_headings(&tab.elements, footnotes, footnote_index, visitor);
                }
            }
            Element::List { items, .. } => {
                for item in items {
                    match item {
                        ListItem::Elements { elements, .. } => {
                            visit_headings(elements, footnotes, footnote_index, visitor);
                        }
                        ListItem::SubList { element } => {
                            visit_headings(
                                std::slice::from_ref(element.as_ref()),
                                footnotes,
                                footnote_index,
                                visitor,
                            );
                        }
                    }
                }
            }
            Element::DefinitionList(items) => {
                for item in items {
                    visit_headings(
                        &item.key_elements,
                        footnotes,
                        footnote_index,
                        visitor,
                    );
                    visit_headings(
                        &item.value_elements,
                        footnotes,
                        footnote_index,
                        visitor,
                    );
                }
            }
            Element::Anchor { elements, .. }
            | Element::Collapsible { elements, .. }
            | Element::Color { elements, .. }
            | Element::Include { elements, .. } => {
                visit_headings(elements, footnotes, footnote_index, visitor);
            }
            Element::Partial(partial) => match partial {
                PartialElement::ListItem(ListItem::Elements { elements, .. }) => {
                    visit_headings(elements, footnotes, footnote_index, visitor);
                }
                PartialElement::ListItem(ListItem::SubList { element }) => {
                    visit_headings(
                        std::slice::from_ref(element.as_ref()),
                        footnotes,
                        footnote_index,
                        visitor,
                    );
                }
                PartialElement::TableRow(row) => {
                    for cell in &row.cells {
                        visit_headings(
                            &cell.elements,
                            footnotes,
                            footnote_index,
                            visitor,
                        );
                    }
                }
                PartialElement::TableCell(cell) => {
                    visit_headings(&cell.elements, footnotes, footnote_index, visitor);
                }
                PartialElement::Tab(tab) => {
                    visit_headings(&tab.elements, footnotes, footnote_index, visitor);
                }
                PartialElement::RubyText(text) => {
                    visit_headings(&text.elements, footnotes, footnote_index, visitor);
                }
                PartialElement::InlineSizeOpen(_)
                | PartialElement::InlineSizeClose
                | PartialElement::InlineSpanOpen(_)
                | PartialElement::InlineSpanClose(_) => {}
            },
            Element::Footnote => {
                let index = *footnote_index;
                *footnote_index += 1;
                if let Some(contents) = footnotes.get(index) {
                    visit_headings(contents, footnotes, footnote_index, visitor);
                }
            }
            _ => {}
        }
    }
}

#[cfg(feature = "html")]
pub(super) fn bind_labels(
    tree: &mut SyntaxTree<'static>,
    delayed_entries: &[usize],
    page_info: &PageInfo,
    settings: &WikitextSettings,
) {
    if delayed_entries.is_empty() {
        return;
    }

    let targets = delayed_entries.iter().copied().collect::<BTreeSet<_>>();
    let mut labels = BTreeMap::new();
    let mut entry_index = 0usize;
    let mut footnote_index = 0usize;
    visit_headings(
        &tree.elements,
        &tree.footnotes,
        &mut footnote_index,
        &mut |elements| {
            if targets.contains(&entry_index) {
                labels.insert(
                    entry_index,
                    TextRender.render_partial(elements, page_info, settings, 0),
                );
            }
            entry_index += 1;
        },
    );
    debug_assert_eq!(labels.len(), delayed_entries.len());

    let mut current = 0usize;
    replace_labels(&mut tree.table_of_contents, &labels, &mut current);
}

#[cfg(feature = "html")]
fn replace_labels(
    elements: &mut [Element<'static>],
    labels: &BTreeMap<usize, String>,
    current: &mut usize,
) {
    for element in elements {
        let Element::List { items, .. } = element else {
            continue;
        };
        for item in items {
            match item {
                ListItem::Elements { elements, .. } => {
                    if let Some(label) = labels.get(current)
                        && let Some(LinkLabel::Text(text)) =
                            elements.iter_mut().find_map(|element| {
                                let Element::Link {
                                    ltype: LinkType::TableOfContents,
                                    label,
                                    ..
                                } = element
                                else {
                                    return None;
                                };
                                Some(label)
                            })
                    {
                        *text = Cow::Owned(label.clone());
                    }
                    *current += 1;
                }
                ListItem::SubList { element } => {
                    replace_labels(
                        std::slice::from_mut(element.as_mut()),
                        labels,
                        current,
                    );
                }
            }
        }
    }
}
