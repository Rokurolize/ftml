/*
 * parsing/rule/impls/block/blocks/list.rs
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

use super::prelude::*;
use crate::parsing::{ParserWrap, strip_newlines};
use crate::tree::{AcceptsPartial, ListItem, ListType, PartialElement};

// Definitions

pub const BLOCK_UL: BlockRule = BlockRule {
    name: "block-list-unordered",
    accepts_names: &["ul"],
    accepts_star: false,
    accepts_score: true,
    accepts_newlines: true,
    parse_fn: parse_unordered_block,
};

pub const BLOCK_OL: BlockRule = BlockRule {
    name: "block-list-ordered",
    accepts_names: &["ol"],
    accepts_star: false,
    accepts_score: true,
    accepts_newlines: true,
    parse_fn: parse_ordered_block,
};

pub const BLOCK_LI: BlockRule = BlockRule {
    name: "block-list-item",
    accepts_names: &["li"],
    accepts_star: false,
    accepts_score: true,
    accepts_newlines: true,
    parse_fn: parse_list_item,
};

fn parse_unordered_block<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    let block = (&BLOCK_UL, ListType::Bullet);

    parse_list_block(block, parser, name, flag_star, flag_score, in_head)
}

fn parse_ordered_block<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    let block = (&BLOCK_OL, ListType::Numbered);

    parse_list_block(block, parser, name, flag_star, flag_score, in_head)
}

// List block

fn parse_list_block<'r, 't>(
    (block_rule, list_type): (&BlockRule, ListType),
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    let rule_name = block_rule.name;
    let list_type_name = list_type.name();
    debug!(
        "List block: name={name}, rule={rule_name}, type={list_type_name}, in_head={in_head}, score={flag_score}"
    );

    let parser = &mut ParserWrap::new(parser, AcceptsPartial::ListItem);

    assert!(!flag_star, "List block doesn't allow star flag");
    assert_block_name(block_rule, name);

    // Get attributes
    let arguments = parser.get_head_map(block_rule, in_head)?;
    let attributes = arguments.to_attribute_map(parser.settings());

    // Get body and convert into list form.
    let body = parser.get_body_elements(block_rule, false)?;
    let (mut elements, errors, _) = body.into();

    let items = {
        let mut items = Vec::new();

        // "ul_" strips outer newlines and paragraph breaks.
        if flag_score {
            strip_newlines(&mut elements);
        }

        // Empty lists aren't allowed
        if elements.is_empty() {
            return Err(parser.make_err(ParseErrorKind::ListEmpty));
        }

        // Convert and extract list elements
        for element in elements {
            match element {
                // Ensure all elements of a list are only items, i.e. [[li]].
                Element::Partial(PartialElement::ListItem(list_item)) => {
                    items.push(list_item);
                }

                // Or sub-lists.
                element @ Element::List { .. } => {
                    let element = Box::new(element);
                    items.push(ListItem::SubList { element });
                }

                // Ignore "whitespace" elements
                element if element.is_whitespace() => continue,

                // Other kinds of elements result in an exception.
                _ => return Err(parser.make_err(ParseErrorKind::ListContainsNonItem)),
            }
        }

        items
    };

    let element = Element::List {
        ltype: list_type,
        items,
        attributes,
    };

    ok!(false; element, errors)
}

// List item

fn parse_list_item<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("List item block: name={name}, in_head={in_head}, score={flag_score}");
    assert!(!flag_star, "List item block doesn't allow star flag");
    assert_block_name(&BLOCK_LI, name);

    // Get attributes
    let arguments = parser.get_head_map(&BLOCK_LI, in_head)?;
    let attributes = arguments.to_attribute_map(parser.settings());

    // Get body elements
    let body = parser.get_body_elements(&BLOCK_LI, false)?;
    let (mut elements, errors, _) = body.into();

    // "li_" strips outer newlines and paragraph breaks.
    if flag_score {
        strip_newlines(&mut elements);
    }

    let element = Element::Partial(PartialElement::ListItem(ListItem::Elements {
        elements,
        attributes,
    }));

    ok!(false; element, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::parsing::ParseError;
    use crate::settings::{WikitextMode, WikitextSettings};

    fn with_parse<R>(
        source: &str,
        check: impl for<'t> FnOnce(Vec<Element<'t>>, Vec<ParseError>) -> R,
    ) -> R {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        check(tree.elements, errors)
    }

    fn element_text(elements: &[Element]) -> String {
        elements
            .iter()
            .filter_map(|element| match element {
                Element::Text(text) => Some(text.as_ref()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn unordered_block_list_preserves_attributes_and_items() {
        with_parse(
            r#"[[ul class="menu"]]
[[li class="first"]]Alpha[[/li]]
[[li]]Beta[[/li]]
[[/ul]]"#,
            |tree, errors| {
                assert!(errors.is_empty(), "{errors:?}");
                let [
                    Element::List {
                        ltype,
                        attributes,
                        items,
                    },
                ] = tree.as_slice()
                else {
                    panic!("expected one unordered list, got {tree:?}");
                };

                assert_eq!(*ltype, ListType::Bullet);
                assert_eq!(
                    attributes.get().get("class").map(|value| value.as_ref()),
                    Some("menu")
                );
                assert_eq!(items.len(), 2);

                let ListItem::Elements {
                    attributes,
                    elements,
                } = &items[0]
                else {
                    panic!("expected first list item, got {:?}", items[0]);
                };
                assert_eq!(
                    attributes.get().get("class").map(|value| value.as_ref()),
                    Some("first")
                );
                assert_eq!(element_text(elements), "Alpha");

                let ListItem::Elements { elements, .. } = &items[1] else {
                    panic!("expected second list item, got {:?}", items[1]);
                };
                assert_eq!(element_text(elements), "Beta");
            },
        );
    }

    #[test]
    fn ordered_block_list_accepts_nested_sublist() {
        with_parse(
            r#"[[ol]]
[[li]]Parent[[/li]]
[[ul]]
[[li]]Child[[/li]]
[[/ul]]
[[/ol]]"#,
            |tree, errors| {
                assert!(errors.is_empty(), "{errors:?}");
                let [Element::List { ltype, items, .. }] = tree.as_slice() else {
                    panic!("expected one ordered list, got {tree:?}");
                };

                assert_eq!(*ltype, ListType::Numbered);
                assert_eq!(items.len(), 2);
                let ListItem::Elements { elements, .. } = &items[0] else {
                    panic!("expected parent item, got {:?}", items[0]);
                };
                assert_eq!(element_text(elements), "Parent");

                let ListItem::SubList { element } = &items[1] else {
                    panic!("expected nested sublist item, got {:?}", items[1]);
                };
                let Element::List {
                    ltype,
                    attributes,
                    items,
                } = element.as_ref()
                else {
                    panic!("expected nested list element, got {element:?}");
                };

                assert_eq!(*ltype, ListType::Bullet);
                assert!(attributes.get().is_empty());
                let [ListItem::Elements { elements, .. }] = items.as_slice() else {
                    panic!("expected one nested list item, got {items:?}");
                };
                assert_eq!(element_text(elements), "Child");
            },
        );
    }

    #[test]
    fn scored_block_and_item_strip_outer_line_breaks() {
        with_parse(
            r#"[[ul_]]
[[li_]]
Alpha
[[/li]]
[[/ul]]"#,
            |tree, errors| {
                assert!(errors.is_empty(), "{errors:?}");
                let [Element::List { items, .. }] = tree.as_slice() else {
                    panic!("expected one list, got {tree:?}");
                };
                let [ListItem::Elements { elements, .. }] = items.as_slice() else {
                    panic!("expected one list item, got {items:?}");
                };

                assert!(
                    !elements
                        .iter()
                        .any(|element| matches!(element, Element::LineBreak))
                );
                assert_eq!(element_text(elements), "Alpha");
            },
        );
    }

    #[test]
    fn block_list_rejects_empty_body() {
        with_parse("[[ul]][[/ul]]", |_tree, errors| {
            assert!(
                errors
                    .iter()
                    .any(|error| error.kind() == ParseErrorKind::ListEmpty)
            );
        });
    }

    #[test]
    fn block_list_rejects_non_item_body() {
        with_parse("[[ul]]plain text[[/ul]]", |_tree, errors| {
            assert!(
                errors
                    .iter()
                    .any(|error| error.kind() == ParseErrorKind::ListContainsNonItem)
            );
        });
    }
}
