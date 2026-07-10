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

pub fn render_footnote(ctx: &mut HtmlContext) {
    debug!("Rendering footnote reference");

    let index = ctx.next_footnote_index();
    let id = str!(index);

    // TODO make this into a locale template string
    let footnote_string = ctx.handle().get_message(ctx.language(), "footnote");
    let label = format!("{footnote_string} {index}.");

    let Some(contents) = ctx.get_footnote(index) else {
        warn!("Footnote index out of bounds from gathered footnote list: {index}");
        let message = ctx
            .handle()
            .get_message(ctx.language(), "footnote-cite-not-found");

        ctx.html()
            .span()
            .attr(attr!("class" => "wj-error-inline"))
            .inner(|ctx| ctx.push_escaped(message));
        return;
    };

    // A footnote reference is phrasing content and is commonly rendered inside a
    // paragraph. Preserve rich inline markup when possible, but flatten block
    // footnotes to escaped text for the hover preview. The full footnote block
    // still renders the original structure below the article.
    let tooltip_text = (!contents.iter().all(|element| {
        !matches!(element, Element::Partial(_)) && element.paragraph_safe()
    }))
    .then(|| TextRender.render_partial(contents, ctx.info(), ctx.settings(), 0));

    ctx.html()
        .span()
        .attr(attr!("class" => "wj-footnote-ref"))
        .inner(|ctx| {
            // Footnote marker that is hoverable
            ctx.html()
                .element("wj-footnote-ref-marker")
                .attr(attr!(
                    "class" => "wj-footnote-ref-marker",
                    "role" => "link",
                    "aria-label" => &label,
                    "data-id" => &id,
                ))
                .contents(&id);

            // Tooltip shown on hover.
            // Is aria-hidden due to difficulty in getting a simultaneous
            // tooltip and link to work. A screen reader can still navigate
            // through to the link and read the footnote directly.
            ctx.html()
                .span()
                .attr(attr!(
                    "class" => "wj-footnote-ref-tooltip",
                    "aria-hidden" => "true",
                ))
                .inner(|ctx| {
                    // Tooltip label
                    ctx.html()
                        .span()
                        .attr(attr!("class" => "wj-footnote-ref-tooltip-label"))
                        .contents(&label);

                    // Actual tooltip contents
                    ctx.html()
                        .span()
                        .attr(attr!("class" => "wj-footnote-ref-contents"))
                        .inner(|ctx| match tooltip_text.as_deref() {
                            Some(text) => render_tooltip_text(ctx, text),
                            None => render_elements(ctx, contents),
                        });
                });
        });
}

fn render_tooltip_text(ctx: &mut HtmlContext, text: &str) {
    for (index, line) in text.split('\n').enumerate() {
        if index > 0 {
            ctx.html().br();
        }
        ctx.push_escaped(line);
    }
}

pub fn render_footnote_block(ctx: &mut HtmlContext, title: Option<&str>) {
    debug!(
        "Rendering footnote block (title {})",
        title.unwrap_or("<default>"),
    );

    let title_default;
    let title: &str = match title {
        Some(title) => title,
        None => {
            title_default = ctx
                .handle()
                .get_message(ctx.language(), "footnote-block-title");

            title_default
        }
    };

    ctx.html()
        .div()
        .attr(attr!("class" => "wj-footnote-list"))
        .inner(|ctx| {
            ctx.html()
                .div()
                .attr(attr!("class" => "wj-title"))
                .contents(title);

            ctx.html().ol().inner(|ctx| {
                // TODO make this into a footnote helper method
                for (index, contents) in ctx.footnotes().iter().enumerate() {
                    let index = index + 1;
                    let id = &format!("{index}");

                    // Build actual footnote item
                    ctx.html()
                        .li()
                        .attr(attr!(
                            "class" => "wj-footnote-list-item",
                            "data-id" => id,
                        ))
                        .inner(|ctx| {
                            // Number and clickable anchor
                            ctx.html()
                                .element("wj-footnote-list-item-marker")
                                .attr(attr!(
                                    "class" => "wj-footnote-list-item-marker",
                                    "type" => "button",
                                    "role" => "link",
                                ))
                                .inner(|ctx| {
                                    str_write!(ctx, "{index}");

                                    // Period after entry number. Has special class to permit styling.
                                    ctx.html()
                                        .span()
                                        .attr(attr!("class" => "wj-footnote-sep"))
                                        .inner(|ctx| ctx.push_raw('.'));
                                });

                            // Footnote contents
                            ctx.html()
                                .div()
                                .attr(attr!("class" => "wj-footnote-list-item-contents"))
                                .contents(contents);
                        });
                }
            });
        });
}
