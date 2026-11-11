/*
 * render/html/element/footnotes.rs
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
use crate::render::text::TextRender;
use crate::tree::{BibliographyScopeWrapper, ContainerType, HtmlTag, ListItem};
use std::num::NonZeroUsize;

pub fn render_footnote(ctx: &mut HtmlContext, index: usize, broke_anchor: bool) {
    debug!("Rendering footnote reference");

    let Some(index) = NonZeroUsize::new(index) else {
        render_missing_footnote(ctx, index);
        return;
    };
    let id = str!(index);

    let footnote_string = ctx.handle().get_message(ctx.language(), "footnote");
    let label = format!("{footnote_string} {index}.");

    let Some(contents) = ctx.get_footnote(index) else {
        render_missing_footnote(ctx, index.get());
        return;
    };

    if ctx.layout() == Layout::Wikidot {
        let reference_id = format!("footnoteref-{index}");
        let footnote_id = format!("footnote-{index}");
        let onclick = format!("WIKIDOT.page.utils.scrollToReference('{footnote_id}')");
        let emit_anchor = |ctx: &mut HtmlContext| {
            ctx.html()
                .a()
                .attr(attr!(
                    "id" => &reference_id,
                    "href" => "javascript:;",
                    "class" => "footnoteref",
                    "onclick" => &onclick,
                ))
                .contents(&id);
        };
        if broke_anchor {
            emit_anchor(ctx);
        } else {
            ctx.html()
                .sup()
                .attr(attr!("class" => "footnoteref"))
                .inner(emit_anchor);
        }
        return;
    }

    let tooltip_text = (!contents.iter().all(|element| {
        !matches!(element, Element::Partial(_)) && element.paragraph_safe()
    }))
    .then(|| TextRender.render_partial(contents, ctx.info(), ctx.settings(), 0));

    if tooltip_text.is_none() && !ctx.enter_footnote_ref(index.get()) {
        render_missing_footnote(ctx, index.get());
        return;
    }

    ctx.html()
        .span()
        .attr(attr!("class" => "wj-footnote-ref"))
        .inner(|ctx| {
            ctx.html()
                .element("wj-footnote-ref-marker")
                .attr(attr!(
                    "class" => "wj-footnote-ref-marker",
                    "role" => "link",
                    "aria-label" => &label,
                    "data-id" => &id,
                ))
                .contents(&id);

            ctx.html()
                .span()
                .attr(attr!(
                    "class" => "wj-footnote-ref-tooltip",
                    "aria-hidden" => "true",
                ))
                .inner(|ctx| {
                    ctx.html()
                        .span()
                        .attr(attr!("class" => "wj-footnote-ref-tooltip-label"))
                        .contents(&label);

                    ctx.html()
                        .span()
                        .attr(attr!("class" => "wj-footnote-ref-contents"))
                        .inner(|ctx| match tooltip_text.as_deref() {
                            Some(text) => render_tooltip_text(ctx, text),
                            None => render_elements(ctx, contents),
                        });
                });
        });
    if tooltip_text.is_none() {
        ctx.exit_footnote_ref(index.get());
    }
}

fn render_missing_footnote(ctx: &mut HtmlContext, index: usize) {
    warn!("Footnote index out of bounds from gathered footnote list: {index}");
    let message = ctx
        .handle()
        .get_message(ctx.language(), "footnote-cite-not-found");

    ctx.html()
        .span()
        .attr(attr!("class" => "wj-error-inline"))
        .inner(|ctx| ctx.push_escaped(message));
}

fn render_tooltip_text(ctx: &mut HtmlContext, text: &str) {
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            ctx.html().br();
        }
        ctx.push_escaped(line);
    }
}

pub fn render_footnote_block(
    ctx: &mut HtmlContext,
    title: Option<&str>,
    wrappers: &[BibliographyScopeWrapper<'_>],
) {
    debug!(
        "Rendering footnote block (title {})",
        title.unwrap_or("<default>"),
    );

    let title_default;
    let explicit_empty_title =
        title == Some(crate::tree::WIKIDOT_EXPLICIT_EMPTY_LABEL_MARKER);
    let title: &str = match title {
        Some(title) if !explicit_empty_title => title,
        Some(_) => "",
        None => {
            title_default = ctx
                .handle()
                .get_message(ctx.language(), "footnote-block-title");

            title_default
        }
    };

    if ctx.layout() == Layout::Wikidot {
        let title = (explicit_empty_title || !title.is_empty()).then_some(title);
        let tiers: Vec<Vec<u8>> = ctx
            .footnotes()
            .iter()
            .map(|contents| {
                if wrappers.is_empty() {
                    wikidot_footnote_tiers(contents)
                } else {
                    vec![0; contents.len()]
                }
            })
            .collect();
        ctx.html()
            .div()
            .attr(attr!("class" => "footnotes-footer"))
            .inner(|ctx| {
                if let Some(title) = title {
                    if wrappers.is_empty() {
                        ctx.html()
                            .div()
                            .attr(attr!("class" => "title"))
                            .contents(title);
                    } else {
                        ctx.html()
                            .div()
                            .attr(attr!("class" => "title"))
                            .inner(|ctx| {
                                render_wikidot_wrapped(ctx, wrappers, &mut |ctx| {
                                    ctx.push_escaped(title);
                                });
                            });
                    }
                }
                for (index, contents) in ctx.footnotes().iter().enumerate() {
                    let tiers = &tiers[index];
                    let index = index + 1;
                    let id = index.to_string();
                    let footnote_id = format!("footnote-{index}");
                    let onclick = format!(
                        "WIKIDOT.page.utils.scrollToReference('footnoteref-{index}')"
                    );
                    ctx.html()
                        .div()
                        .attr(attr!(
                            "class" => "footnote-footer",
                            "id" => &footnote_id,
                        ))
                        .inner(|ctx| {
                            let mut body = |ctx: &mut HtmlContext| {
                                ctx.html()
                                    .a()
                                    .attr(attr!(
                                        "href" => "javascript:;",
                                        "onclick" => &onclick,
                                    ))
                                    .contents(&id);
                                ctx.push_raw('.');
                                if tiers.contains(&0) {
                                    ctx.push_raw(' ');
                                }
                                render_wikidot_footnote_tier(ctx, contents, tiers, 0);
                            };
                            if wrappers.is_empty() {
                                body(ctx);
                            } else {
                                render_wikidot_wrapped(ctx, wrappers, &mut body);
                            }
                        });
                    render_wikidot_footnote_tier(ctx, contents, tiers, 1);
                }
            });
        for (index, contents) in ctx.footnotes().iter().enumerate() {
            render_wikidot_footnote_tier(ctx, contents, &tiers[index], 2);
        }
        return;
    }

    ctx.html()
        .div()
        .attr(attr!("class" => "wj-footnote-list"))
        .inner(|ctx| {
            ctx.html()
                .div()
                .attr(attr!("class" => "wj-title"))
                .contents(title);

            ctx.html().ol().inner(|ctx| {
                for (index, contents) in ctx.footnotes().iter().enumerate() {
                    let index = index + 1;
                    let id = &format!("{index}");

                    ctx.html()
                        .li()
                        .attr(attr!(
                            "class" => "wj-footnote-list-item",
                            "data-id" => id,
                        ))
                        .inner(|ctx| {
                            ctx.html()
                                .element("wj-footnote-list-item-marker")
                                .attr(attr!(
                                    "class" => "wj-footnote-list-item-marker",
                                    "type" => "button",
                                    "role" => "link",
                                ))
                                .inner(|ctx| {
                                    str_write!(ctx, "{index}");

                                    ctx.html()
                                        .span()
                                        .attr(attr!("class" => "wj-footnote-sep"))
                                        .inner(|ctx| ctx.push_raw('.'));
                                });

                            ctx.html()
                                .div()
                                .attr(attr!("class" => "wj-footnote-list-item-contents"))
                                .contents(contents);
                        });
                }
            });
        });
}

fn render_wikidot_footnote_tier(
    ctx: &mut HtmlContext,
    contents: &[Element],
    tiers: &[u8],
    tier: u8,
) {
    for (element, element_tier) in contents.iter().zip(tiers) {
        if *element_tier != tier {
            continue;
        }
        if matches!(
            element,
            Element::Container(container) if container.ctype() == ContainerType::Paragraph
        ) {
            ctx.push_raw('\n');
        }
        render_element(ctx, element);
    }
}

fn footnote_element_has_row_suffix(element: &Element<'_>) -> bool {
    match element {
        Element::Text(text) if text.as_ref() == "||" => true,
        Element::Container(container) => container.elements().iter().any(|child| {
            matches!(child, Element::Text(text) if text.as_ref() == "||")
        }),
        _ => false,
    }
}

fn wikidot_footnote_tiers(contents: &[Element]) -> Vec<u8> {
    let mut tiers = vec![0u8; contents.len()];
    if let Some(index) = contents
        .iter()
        .position(footnote_element_has_row_suffix)
    {
        for tier in &mut tiers[index..] {
            *tier = 1;
        }
    }
    let list_positions: Vec<usize> = contents
        .iter()
        .enumerate()
        .filter(|(_, element)| matches!(element, Element::List { .. }))
        .map(|(index, _)| index)
        .collect();
    let Some(frame_order) = list_positions
        .iter()
        .position(|&index| footnote_list_contains_frame(&contents[index]))
    else {
        return tiers;
    };
    let frame_index = list_positions[frame_order];
    let Element::List { items, .. } = &contents[frame_index] else {
        return tiers;
    };
    let close_in_frame =
        items.len() == 1 || items.last().is_some_and(footnote_item_is_bare_marker);
    if close_in_frame {
        let mut order = 0usize;
        let mut preceding = 0u8;
        for (index, element) in contents.iter().enumerate().skip(frame_index + 1) {
            if matches!(element, Element::List { .. }) {
                order += 1;
                let tier = order.min(2) as u8;
                tiers[index] = tier;
                preceding = tier;
            } else {
                tiers[index] = preceding.clamp(1, 2);
            }
        }
    } else {
        let Some(&following_index) = list_positions.get(frame_order + 1) else {
            return tiers;
        };
        let mut tier = 0u8;
        let mut preceding = 0u8;
        for (index, element) in contents.iter().enumerate().skip(following_index + 1) {
            if let Element::List { items, .. } = element {
                if !footnote_list_is_all_bare(items) {
                    tier = (tier + 1).min(2);
                }
                tiers[index] = tier;
                preceding = tier;
            } else {
                tiers[index] = preceding.clamp(1, 2);
            }
        }
    }
    tiers
}

fn footnote_list_contains_frame(element: &Element) -> bool {
    let Element::List { items, .. } = element else {
        return false;
    };
    items.iter().any(|item| match item {
        ListItem::Elements { elements, .. } => footnote_elements_contain_frame(elements),
        ListItem::SubList { element } => footnote_element_contains_frame(element),
    })
}

fn footnote_element_contains_frame(element: &Element) -> bool {
    match element {
        Element::WikidotBlockFrame { .. } => true,
        Element::Container(container) => {
            footnote_elements_contain_frame(container.elements())
        }
        Element::List { items, .. } => items.iter().any(|item| match item {
            ListItem::Elements { elements, .. } => footnote_elements_contain_frame(elements),
            ListItem::SubList { element } => footnote_element_contains_frame(element),
        }),
        Element::Color { elements, .. }
        | Element::Anchor { elements, .. }
        | Element::Collapsible { elements, .. }
        | Element::Include { elements, .. } => footnote_elements_contain_frame(elements),
        _ => false,
    }
}

fn footnote_elements_contain_frame(elements: &[Element]) -> bool {
    elements.iter().any(footnote_element_contains_frame)
}

fn footnote_item_is_bare_marker(item: &ListItem) -> bool {
    let ListItem::Elements { elements, .. } = item else {
        return false;
    };
    elements.len() == 1
        && matches!(&elements[0], Element::Text(text) if text.as_ref() == "*" || text.as_ref() == "#")
}

fn footnote_list_is_all_bare(items: &[ListItem]) -> bool {
    !items.is_empty() && items.iter().all(footnote_item_is_bare_marker)
}

fn render_wikidot_wrapped(
    ctx: &mut HtmlContext,
    wrappers: &[BibliographyScopeWrapper<'_>],
    inner: &mut dyn FnMut(&mut HtmlContext),
) {
    let Some((wrapper, rest)) = wrappers.split_first() else {
        inner(ctx);
        return;
    };

    match wrapper {
        BibliographyScopeWrapper::Color { color, background } => {
            let property = if *background {
                "background-color"
            } else {
                "color"
            };
            let style = format!("{property}: {color}");
            ctx.html()
                .span()
                .attr(attr!("style" => &style))
                .inner(|ctx| render_wikidot_wrapped(ctx, rest, inner));
        }
        BibliographyScopeWrapper::Container { ctype, attributes } => {
            let tag_spec = ctype.html_tag(ctx.layout(), ctx);
            let mut tag = ctx.html().tag(tag_spec.tag());
            match &tag_spec {
                HtmlTag::Tag(_) => {
                    tag.attr(attr!(;; attributes));
                }
                HtmlTag::TagAndClass { class, .. } => {
                    tag.attr(attr!("class" => class;; attributes));
                }
                HtmlTag::TagAndStyle { style, .. } => {
                    tag.attr(attr!("style" => style;; attributes));
                }
                HtmlTag::TagAndId { id, .. } => {
                    tag.attr(attr!("id" => id;; attributes));
                }
            }
            tag.inner(|ctx| render_wikidot_wrapped(ctx, rest, inner));
        }
    }
}
