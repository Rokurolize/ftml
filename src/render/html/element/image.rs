/*
 * render/html/element/image.rs
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
use crate::tree::{AttributeMap, FileSource, FloatAlignment, LinkLocation};
use crate::url::normalize_link;

pub fn render_image(
    ctx: &mut HtmlContext,
    source: &FileSource,
    link: &Option<LinkLocation>,
    alignment: Option<FloatAlignment>,
    attributes: &AttributeMap,
) {
    debug!(
        "Rendering image element (source '{}', link {:?}, alignment {}, float {})",
        source.name(),
        link,
        match alignment {
            Some(image) => image.align.name(),
            None => "<default>",
        },
        match alignment {
            Some(image) => image.float,
            None => false,
        },
    );

    let source_url = ctx
        .handle()
        .get_file_link(source, ctx.info(), ctx.settings());

    match source_url {
        // Found URL
        Some(url) => render_image_element(ctx, &url, link, alignment, attributes),

        // Missing or error
        None => render_image_missing(ctx),
    }
}

fn render_image_element(
    ctx: &mut HtmlContext,
    image_url: &str,
    link: &Option<LinkLocation>,
    alignment: Option<FloatAlignment>,
    attributes: &AttributeMap,
) {
    trace!("Found URL, rendering image (value '{image_url}')");

    match ctx.layout() {
        Layout::Wikidot => {
            render_image_element_wikidot(ctx, image_url, link, alignment, attributes);
        }
        Layout::Wikijump => {
            render_image_element_wikijump(ctx, image_url, link, alignment, attributes);
        }
    }
}

/// Render an image block with a Wikidot-compatible DOM.
///
/// The structure is thus:
/// 1. If alignment, wrap in `<div>`. Otherwise nothing.
/// 2. If link, wrap in `<a>`. Otherwise nothing.
/// 3. The image itself, `<img>`.
///
/// We define the closures in reverse order so
/// we can properly (conditionally) nest them.
fn render_image_element_wikidot(
    ctx: &mut HtmlContext,
    image_url: &str,
    link: &Option<LinkLocation>,
    alignment: Option<FloatAlignment>,
    attributes: &AttributeMap,
) {
    let get = |name| attributes.get().get(name).map(|value| value.as_ref());
    let default_alt = image_url
        .split(['?', '#'])
        .next()
        .unwrap_or(image_url)
        .rsplit('/')
        .next()
        .unwrap_or("");
    let alt = get("alt").unwrap_or(default_alt);
    let class = get("class").unwrap_or("image");
    let width = get("width");
    let height = get("height");
    let title = get("title");
    let style = get("style");

    let build_image = |ctx: &mut HtmlContext| {
        ctx.html().img().attr(attr!(
            "src" => image_url,
            "width" => width.unwrap_or(""); if width.is_some(),
            "height" => height.unwrap_or(""); if height.is_some(),
            "title" => title.unwrap_or(""); if title.is_some(),
            "style" => style.unwrap_or(""); if style.is_some(),
            "class" => class,
            "alt" => alt,
        ));
    };

    let build_link = |ctx: &mut HtmlContext| match link {
        None => build_image(ctx),
        Some(link) => {
            let url = normalize_link(link, ctx.handle());
            ctx.html()
                .a()
                .attr(attr!("href" => &url))
                .inner(build_image);
        }
    };

    match alignment {
        None => build_link(ctx),
        Some(align) => {
            let class = if align.float && align.align == crate::tree::Alignment::Center {
                str!("image-container")
            } else {
                format!("image-container {}", align.wd_html_class())
            };
            ctx.html()
                .div()
                .attr(attr!("class" => &class))
                .inner(build_link);
        }
    }
}

fn render_image_element_wikijump(
    ctx: &mut HtmlContext,
    image_url: &str,
    link: &Option<LinkLocation>,
    alignment: Option<FloatAlignment>,
    attributes: &AttributeMap,
) {
    let (space, align_class) = match alignment {
        Some(align) => (" ", align.wj_html_class()),
        None => ("", ""),
    };

    ctx.html()
        .div()
        .attr(attr!(
            "class" => "wj-image-container" space align_class,
        ))
        .inner(|ctx| {
            let build_image = |ctx: &mut HtmlContext| {
                ctx.html().img().attr(attr!(
                    "class" => "wj-image",
                    "src" => image_url;;
                    attributes
                ));
            };

            match link {
                Some(link) => {
                    let url = normalize_link(link, ctx.handle());
                    ctx.html()
                        .a()
                        .attr(attr!("href" => &url))
                        .inner(build_image);
                }
                None => build_image(ctx),
            };
        });
}

fn render_image_missing(ctx: &mut HtmlContext) {
    trace!("Image URL unresolved, missing or error");

    let message = ctx
        .handle()
        .get_message(ctx.language(), "image-context-bad");

    ctx.html()
        .div()
        .attr(attr!("class" => "wj-error-block"))
        .inner(|ctx| ctx.push_escaped(message));
}
