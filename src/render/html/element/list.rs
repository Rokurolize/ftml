/*
 * render/html/element/list.rs
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

use super::container::render_container;
use super::math::{render_math_block, wikidot_math_number_will_render};
use super::prelude::*;
use crate::tree::{
    AttributeMap, Container, ContainerType, ListItem, ListType, PartialElement,
};

/// A `[[math]]` block directly following an inline scope segment in an item
/// renders its equation number inside that same segment on live, preceded by
/// exactly two line breaks. Foreign scopes (a formed bold, a crossed pair
/// that literalises) and unnumbered failures keep the split layout.
fn render_scope_math_pair(
    ctx: &mut HtmlContext,
    elements: &[Element],
    index: usize,
) -> bool {
    let Element::Container(container) = &elements[index] else {
        return false;
    };
    if !matches!(container.ctype(), ContainerType::Span | ContainerType::Size) {
        return false;
    }
    let Some(math) = elements.get(index + 1) else {
        return false;
    };
    let Element::Math {
        name,
        latex_source,
        environment,
        wikidot_block_open_elements,
        wikidot_label_spans,
        ..
    } = math
    else {
        return false;
    };
    if wikidot_label_spans.is_empty() {
        return false;
    }
    let Some(assigned) = ctx.wikidot_equation_number(std::ptr::from_ref(math) as usize)
    else {
        return false;
    };
    let number_will_render =
        wikidot_math_number_will_render(environment.as_deref(), latex_source);
    let mut children = container.elements().to_vec();
    let mut trailing_breaks = 0;
    for element in children.iter().rev() {
        match element {
            Element::LineBreak => trailing_breaks += 1,
            Element::LineBreaks(amount) => trailing_breaks += amount.get(),
            _ => break,
        }
    }
    for _ in trailing_breaks..2 {
        children.push(Element::LineBreak);
    }
    if number_will_render {
        children.push(Element::Partial(PartialElement::WikidotEquationNumber(
            assigned,
            wikidot_block_open_elements.to_vec(),
        )));
    }
    let merged =
        Container::new(container.ctype(), children, container.attributes().clone());
    render_container(ctx, &merged);
    render_math_block(
        ctx,
        Some(assigned),
        name.as_deref(),
        latex_source,
        environment.as_deref(),
        wikidot_block_open_elements,
        wikidot_label_spans,
        number_will_render,
    );
    true
}

pub fn render_list(
    ctx: &mut HtmlContext,
    ltype: ListType,
    list_items: &[ListItem],
    attributes: &AttributeMap,
    unclosed: bool,
) {
    debug!(
        "Rendering list '{}' (items {})",
        ltype.name(),
        list_items.len(),
    );
    let list_tag = ltype.html_tag();
    let mut tag = ctx.html().tag(list_tag);
    if unclosed {
        tag.omit_close();
    }

    tag.attr(attr!(;; attributes)).inner(|ctx| {
        if ctx.layout().legacy() {
            ctx.push_raw('\n');
        }
        for list_item in list_items {
            match list_item {
                ListItem::Elements {
                    elements,
                    attributes,
                } => {
                  {
                    let mut li = ctx.html().li();
                    li.attr(attr!(;; attributes));
                    if unclosed
                        && list_items
                            .iter()
                            .filter(|item| matches!(item, ListItem::Elements { .. }))
                            .last()
                            .is_some_and(|last| std::ptr::eq(last, list_item))
                    {
                        li.omit_close();
                    }
                    li.inner(|ctx| {
                        let mut index = 0;
                        while index < elements.len() {
                            if ctx.layout().legacy()
                                && render_scope_math_pair(ctx, elements, index)
                            {
                                index += 2;
                                continue;
                            }
                            let element = &elements[index];
                            let nested_list = matches!(element, Element::List { .. });
                            let paragraph = matches!(
                                element,
                                Element::Container(container)
                                    if container.ctype() == ContainerType::Paragraph
                            );
                            if ctx.layout().legacy() && (nested_list || paragraph) {
                                ctx.push_raw('\n');
                            }
                            render_element(ctx, element);
                            if ctx.layout().legacy() && (nested_list || paragraph) {
                                ctx.push_raw('\n');
                            }
                            index += 1;
                        }
                    });
                  }
                    if ctx.layout().legacy() {
                        ctx.push_raw('\n');
                    }
                }
                ListItem::SubList { element } => {
                    let attributes = match ctx.layout() {
                        Layout::Wikidot => {
                            attr!("style" => "list-style: none; display: inline")
                        }
                        Layout::Wikijump => attr!(),
                    };
                    ctx.html().li().attr(attributes).inner(|ctx| {
                        render_element(ctx, element);
                    });
                }
            }
        }
    });
}
