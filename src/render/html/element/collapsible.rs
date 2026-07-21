/*
 * render/html/element/collapsible.rs
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
use crate::tree::{AttributeMap, Element};

#[derive(Debug, Copy, Clone)]
pub struct Collapsible<'a> {
    elements: &'a [Element<'a>],
    attributes: &'a AttributeMap<'a>,
    start_open: bool,
    show_text: Option<&'a str>,
    hide_text: Option<&'a str>,
    show_top: bool,
    show_bottom: bool,
}

impl<'a> Collapsible<'a> {
    #[inline]
    pub fn new(
        elements: &'a [Element<'a>],
        attributes: &'a AttributeMap<'a>,
        start_open: bool,
        show_text: Option<&'a str>,
        hide_text: Option<&'a str>,
        show_top: bool,
        show_bottom: bool,
    ) -> Self {
        Collapsible {
            elements,
            attributes,
            start_open,
            show_text,
            hide_text,
            show_top,
            show_bottom,
        }
    }
}

pub fn render_collapsible(ctx: &mut HtmlContext, collapsible: Collapsible) {
    let Collapsible {
        elements,
        attributes,
        start_open,
        show_text,
        hide_text,
        show_top,
        show_bottom,
    } = collapsible;

    debug!(
        "Rendering collapsible (elements length {}, start-open {}, show-text {}, hide-text {}, show-top {}, show-bottom {})",
        elements.len(),
        start_open,
        show_text.unwrap_or("<default>"),
        hide_text.unwrap_or("<default>"),
        show_top,
        show_bottom,
    );

    let show_text = show_text
        .unwrap_or_else(|| ctx.handle().get_message(ctx.language(), "collapsible-open"));

    let hide_text = hide_text
        .unwrap_or_else(|| ctx.handle().get_message(ctx.language(), "collapsible-hide"));

    match ctx.layout() {
        Layout::Wikidot => render_collapsible_wikidot(
            ctx,
            elements,
            attributes,
            start_open,
            show_text,
            hide_text,
            show_top,
            show_bottom,
        ),
        Layout::Wikijump => render_collapsible_wikijump(
            ctx,
            elements,
            attributes,
            start_open,
            show_text,
            hide_text,
            show_top,
            show_bottom,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_collapsible_wikidot(
    ctx: &mut HtmlContext,
    elements: &[Element],
    attributes: &AttributeMap,
    start_open: bool,
    show_text: &str,
    hide_text: &str,
    show_top: bool,
    show_bottom: bool,
) {
    ctx.html()
        .div()
        .attr(attr!(
            "class" => "collapsible-block";;
            attributes,
        ))
        .inner(|ctx| {
            ctx.html()
                .div()
                .attr(attr!(
                    "class" => "collapsible-block-folded",
                    "style" => "display:none"; if start_open,
                ))
                .inner(|ctx| render_wikidot_collapsible_link(ctx, show_text));

            ctx.html()
                .div()
                .attr(attr!(
                    "class" => "collapsible-block-unfolded",
                    "style" => "display:none"; if !start_open,
                ))
                .inner(|ctx| {
                    if show_top {
                        render_wikidot_hide_link(ctx, hide_text);
                    }

                    ctx.html()
                        .div()
                        .attr(attr!("class" => "collapsible-block-content"))
                        .contents(elements);

                    if show_bottom {
                        render_wikidot_hide_link(ctx, hide_text);
                    }
                });
        });
}

fn render_wikidot_hide_link(ctx: &mut HtmlContext, hide_text: &str) {
    ctx.html()
        .div()
        .attr(attr!("class" => "collapsible-block-unfolded-link"))
        .inner(|ctx| render_wikidot_collapsible_link(ctx, hide_text));
}

fn render_wikidot_collapsible_link(ctx: &mut HtmlContext, label: &str) {
    ctx.html()
        .a()
        .attr(attr!(
            "class" => "collapsible-block-link",
            "href" => "javascript:;",
        ))
        .inner(|ctx| {
            for (index, part) in label.split(' ').enumerate() {
                if index > 0 {
                    ctx.push_raw_str("&nbsp;");
                }
                ctx.push_escaped(part);
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn render_collapsible_wikijump(
    ctx: &mut HtmlContext,
    elements: &[Element],
    attributes: &AttributeMap,
    start_open: bool,
    show_text: &str,
    hide_text: &str,
    show_top: bool,
    show_bottom: bool,
) {
    ctx.html()
        .details()
        .attr(attr!(
            "class" => "wj-collapsible",
            "open"; if start_open,
            "data-show-top"; if show_top,
            "data-show-bottom"; if show_bottom;;
            attributes,
        ))
        .inner(|ctx| {
            // Open/close button
            ctx.html()
                .summary()
                .attr(attr!(
                    "class" => "wj-collapsible-button wj-collapsible-button-top",
                ))
                .inner(|ctx| {
                    // Block is folded text
                    ctx.html()
                        .span()
                        .attr(attr!("class" => "wj-collapsible-show-text"))
                        .contents(show_text);

                    // Block is unfolded text
                    ctx.html()
                        .span()
                        .attr(attr!("class" => "wj-collapsible-hide-text"))
                        .contents(hide_text);
                });

            // Content block
            ctx.html()
                .div()
                .attr(attr!("class" => "wj-collapsible-content"))
                .contents(elements);

            // Bottom open/close button
            if show_bottom {
                ctx.html()
                    .element("wj-collapsible-button-bottom")
                    .attr(attr!(
                        "class" => "wj-collapsible-button wj-collapsible-button-bottom",
                    ))
                    .inner(|ctx| {
                        // Block is unfolded text
                        ctx.html()
                            .span()
                            .attr(attr!("class" => "wj-collapsible-hide-text"))
                            .contents(hide_text);
                    });
            }
        });
}
