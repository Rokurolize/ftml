/*
 * tree/mod.rs
 *
 * ftml - Library to parse Wikidot text
 * Copyright (C) 2019-2026 Wikijump Team
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <http://www.gnu.org/licenses/>.
 */

pub mod attribute;

mod align;
mod anchor;
mod bibliography;
mod button;
mod clear_float;
mod clone;
mod code;
mod container;
mod content;
mod date;
mod definition_list;
mod element;
mod embed;
mod embed_video;
mod file_source;
mod heading;
mod image_source;
mod link;
mod list;
mod module;
mod partial;
mod ruby;
mod tab;
mod table;
mod tag;
mod variables;

pub use self::align::*;
pub use self::anchor::*;
pub use self::attribute::AttributeMap;
pub use self::bibliography::*;
pub use self::button::*;
pub use self::clear_float::*;
pub use self::code::CodeBlock;
pub use self::container::*;
pub use self::content::*;
pub use self::date::DateItem;
pub(crate) use self::date::date_format_within_limits;
pub use self::definition_list::*;
pub use self::element::*;
pub use self::embed::*;
pub use self::embed_video::*;
pub use self::file_source::*;
pub use self::heading::*;
pub use self::image_source::*;
pub use self::link::*;
pub use self::list::*;
pub use self::module::*;
pub use self::partial::*;
pub use self::ruby::*;
pub use self::tab::*;
pub use self::table::*;
pub use self::tag::*;
pub use self::variables::*;

use self::clone::{elements_lists_to_owned, elements_to_owned, string_to_owned};
use crate::data::PageRef;
use crate::parsing::{ParseError, ParseOutcome};
use std::borrow::Cow;
use std::collections::HashSet;
use std::ops::Not;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct SyntaxTree<'t> {
    /// The list of elements that compose this tree.
    ///
    /// Note that each `Element<'t>` can contain other elements within it,
    /// and these as well, etc. This structure composes the depth of the
    /// syntax tree.
    pub elements: Vec<Element<'t>>,

    /// The full table of contents for this page.
    ///
    /// Depth list conversion happens here, so that depths on the table
    /// match the heading level.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub table_of_contents: Vec<Element<'t>>,

    /// The full list of HTML blocks for this page.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub html_blocks: Vec<Cow<'t, str>>,

    /// The full list of code blocks for this page.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub code_blocks: Vec<CodeBlock<'t>>,

    /// The full footnote list for this page.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub footnotes: Vec<Vec<Element<'t>>>,

    /// Whether the renderer should add its own footnote block.
    ///
    /// This is true if there is no footnote block in the element
    /// list above, *and* there are footnotes to render.
    // NOTE: Not::not() here is effectively saying "don't serialize if !value"
    //       which is just "is false".
    #[serde(default, skip_serializing_if = "Not::not")]
    pub needs_footnote_block: bool,

    /// The full list of bibliographies for this page.
    #[serde(default, skip_serializing_if = "BibliographyList::is_empty")]
    pub bibliographies: BibliographyList<'t>,

    /// Hint for the size of the wikitext input.
    ///
    /// This is an optimization to make rendering large parges slightly faster.
    #[serde(skip)]
    pub wikitext_len: usize,
}

impl<'t> SyntaxTree<'t> {
    pub(crate) fn from_element_result(
        elements: Vec<Element<'t>>,
        errors: Vec<ParseError>,
        (html_blocks, code_blocks): (Vec<Cow<'t, str>>, Vec<CodeBlock<'t>>),
        table_of_contents: Vec<Element<'t>>,
        (footnotes, needs_footnote_block): (Vec<Vec<Element<'t>>>, bool),
        bibliographies: BibliographyList<'t>,
        wikitext_len: usize,
    ) -> ParseOutcome<Self> {
        let tree = SyntaxTree {
            elements,
            table_of_contents,
            html_blocks,
            code_blocks,
            footnotes,
            needs_footnote_block,
            bibliographies,
            wikitext_len,
        };
        ParseOutcome::new(tree, errors)
    }

    pub fn to_owned(&self) -> SyntaxTree<'static> {
        SyntaxTree {
            elements: elements_to_owned(&self.elements),
            table_of_contents: elements_to_owned(&self.table_of_contents),
            html_blocks: self
                .html_blocks
                .iter()
                .map(|html| string_to_owned(html))
                .collect(),
            code_blocks: self
                .code_blocks
                .iter()
                .map(|code| code.to_owned())
                .collect(),
            footnotes: elements_lists_to_owned(&self.footnotes),
            needs_footnote_block: self.needs_footnote_block,
            bibliographies: self.bibliographies.to_owned(),
            wikitext_len: self.wikitext_len,
        }
    }

    /// Collects the distinct internal page references rendered by this tree.
    ///
    /// The returned references retain their site and extra URL portions. Their
    /// order is the order of first appearance in the tree.
    pub fn page_references(&self) -> Vec<PageRef> {
        let mut references = Vec::new();
        let mut seen = HashSet::new();

        collect_page_references(&self.elements, &mut references, &mut seen);
        collect_page_references(&self.table_of_contents, &mut references, &mut seen);
        for footnote in &self.footnotes {
            collect_page_references(footnote, &mut references, &mut seen);
        }
        for bibliography in self.bibliographies.slice() {
            for (_, elements) in bibliography.slice() {
                collect_page_references(elements, &mut references, &mut seen);
            }
        }

        references
    }

    /// Collects the distinct user names rendered by this tree.
    ///
    /// Their order is the order of first appearance in the tree.
    pub fn user_references(&self) -> Vec<String> {
        let mut references = Vec::new();
        let mut seen = HashSet::new();

        collect_user_references(&self.elements, &mut references, &mut seen);
        collect_user_references(&self.table_of_contents, &mut references, &mut seen);
        for footnote in &self.footnotes {
            collect_user_references(footnote, &mut references, &mut seen);
        }
        for bibliography in self.bibliographies.slice() {
            for (_, elements) in bibliography.slice() {
                collect_user_references(elements, &mut references, &mut seen);
            }
        }

        references
    }
}

fn collect_page_references(
    elements: &[Element<'_>],
    references: &mut Vec<PageRef>,
    seen: &mut HashSet<PageRef>,
) {
    for element in elements {
        if let Element::Link {
            link: LinkLocation::Page(page),
            ..
        } = element
            && seen.insert(page.clone())
        {
            references.push(page.clone());
        }

        match element {
            Element::Container(container) => {
                collect_page_references(container.elements(), references, seen);
            }
            Element::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_page_references(&cell.elements, references, seen);
                    }
                }
            }
            Element::TabView(tabs) => {
                for tab in tabs {
                    collect_page_references(&tab.elements, references, seen);
                }
            }
            Element::Anchor { elements, .. }
            | Element::Collapsible { elements, .. }
            | Element::Color { elements, .. }
            | Element::Include { elements, .. } => {
                collect_page_references(elements, references, seen);
            }
            Element::List { items, .. } => {
                for item in items {
                    match item {
                        ListItem::Elements { elements, .. } => {
                            collect_page_references(elements, references, seen);
                        }
                        ListItem::SubList { element } => {
                            collect_page_references(
                                std::slice::from_ref(element),
                                references,
                                seen,
                            );
                        }
                    }
                }
            }
            Element::DefinitionList(items) => {
                for item in items {
                    collect_page_references(&item.key_elements, references, seen);
                    collect_page_references(&item.value_elements, references, seen);
                }
            }
            Element::Partial(partial) => match partial {
                PartialElement::ListItem(ListItem::Elements { elements, .. }) => {
                    collect_page_references(elements, references, seen);
                }
                PartialElement::ListItem(ListItem::SubList { element }) => {
                    collect_page_references(
                        std::slice::from_ref(element),
                        references,
                        seen,
                    );
                }
                PartialElement::TableRow(row) => {
                    for cell in &row.cells {
                        collect_page_references(&cell.elements, references, seen);
                    }
                }
                PartialElement::TableCell(cell) => {
                    collect_page_references(&cell.elements, references, seen);
                }
                PartialElement::Tab(tab) => {
                    collect_page_references(&tab.elements, references, seen);
                }
                PartialElement::RubyText(ruby_text) => {
                    collect_page_references(&ruby_text.elements, references, seen);
                }
                PartialElement::WikidotEmptyInlineOwner
                | PartialElement::InlineSizeOpen(_)
                | PartialElement::InlineSizeClose(_)
                | PartialElement::InlineSpanOpen(_)
                | PartialElement::InlineSpanClose(_) => {}
            },
            _ => {}
        }
    }
}

fn collect_user_references(
    elements: &[Element<'_>],
    references: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    for element in elements {
        if let Element::User { name, .. } = element {
            let name = name.to_string();
            if seen.insert(name.clone()) {
                references.push(name);
            }
        }

        match element {
            Element::Container(container) => {
                collect_user_references(container.elements(), references, seen);
            }
            Element::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_user_references(&cell.elements, references, seen);
                    }
                }
            }
            Element::TabView(tabs) => {
                for tab in tabs {
                    collect_user_references(&tab.elements, references, seen);
                }
            }
            Element::Anchor { elements, .. }
            | Element::Collapsible { elements, .. }
            | Element::Color { elements, .. }
            | Element::Include { elements, .. } => {
                collect_user_references(elements, references, seen);
            }
            Element::List { items, .. } => {
                for item in items {
                    match item {
                        ListItem::Elements { elements, .. } => {
                            collect_user_references(elements, references, seen);
                        }
                        ListItem::SubList { element } => {
                            collect_user_references(
                                std::slice::from_ref(element),
                                references,
                                seen,
                            );
                        }
                    }
                }
            }
            Element::DefinitionList(items) => {
                for item in items {
                    collect_user_references(&item.key_elements, references, seen);
                    collect_user_references(&item.value_elements, references, seen);
                }
            }
            Element::Partial(partial) => match partial {
                PartialElement::ListItem(ListItem::Elements { elements, .. }) => {
                    collect_user_references(elements, references, seen);
                }
                PartialElement::ListItem(ListItem::SubList { element }) => {
                    collect_user_references(
                        std::slice::from_ref(element),
                        references,
                        seen,
                    );
                }
                PartialElement::TableRow(row) => {
                    for cell in &row.cells {
                        collect_user_references(&cell.elements, references, seen);
                    }
                }
                PartialElement::TableCell(cell) => {
                    collect_user_references(&cell.elements, references, seen);
                }
                PartialElement::Tab(tab) => {
                    collect_user_references(&tab.elements, references, seen);
                }
                PartialElement::RubyText(ruby_text) => {
                    collect_user_references(&ruby_text.elements, references, seen);
                }
                PartialElement::WikidotEmptyInlineOwner
                | PartialElement::InlineSizeOpen(_)
                | PartialElement::InlineSizeClose(_)
                | PartialElement::InlineSpanOpen(_)
                | PartialElement::InlineSpanClose(_) => {}
            },
            _ => {}
        }
    }
}

#[test]
fn borrowed_to_owned() {
    use std::mem;

    let tree_1: SyntaxTree<'_> = SyntaxTree::default();
    let tree_2: SyntaxTree<'static> = tree_1.to_owned();

    mem::drop(tree_1);

    let tree_3: SyntaxTree<'static> = tree_2.clone();

    mem::drop(tree_3);
}

#[test]
fn page_references_collect_nested_rendered_locations_once() {
    fn page_link(page: PageRef) -> Element<'static> {
        Element::Link {
            ltype: LinkType::Page,
            link: LinkLocation::Page(page),
            label: LinkLabel::Slug(cow!("label")),
            target: None,
        }
    }

    let nested = PageRef::page_only("nested#section");
    let footnote = PageRef::page_and_site("other-site", "footnote");
    let bibliography = PageRef::page_only("bibliography");
    let mut bibliography_list = BibliographyList::new();
    let mut bibliography_items = Bibliography::new();
    bibliography_items.add(cow!("source"), vec![page_link(bibliography.clone())]);
    bibliography_list.push(bibliography_items);

    let tree = SyntaxTree {
        elements: vec![
            Element::Container(Container::new(
                ContainerType::Div,
                vec![page_link(nested.clone())],
                AttributeMap::new(),
            )),
            page_link(nested.clone()),
            Element::Link {
                ltype: LinkType::Direct,
                link: LinkLocation::Url(cow!("https://example.com")),
                label: LinkLabel::Text(cow!("external")),
                target: None,
            },
        ],
        footnotes: vec![vec![page_link(footnote.clone())]],
        bibliographies: bibliography_list,
        ..SyntaxTree::default()
    };

    assert_eq!(tree.page_references(), vec![nested, footnote, bibliography],);
}

#[test]
fn user_references_collect_nested_names_once() {
    fn user(name: &'static str) -> Element<'static> {
        Element::User {
            name: cow!(name),
            show_avatar: false,
        }
    }

    let tree = SyntaxTree {
        elements: vec![
            Element::Container(Container::new(
                ContainerType::Div,
                vec![user("SYSTEM")],
                AttributeMap::new(),
            )),
            user("SYSTEM"),
        ],
        footnotes: vec![vec![user("account-name")]],
        ..SyntaxTree::default()
    };

    assert_eq!(tree.user_references(), ["SYSTEM", "account-name"]);
}
