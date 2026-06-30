/*
 * render/html/element/container.rs
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
use crate::tree::{Container, ContainerType, HtmlTag};

pub fn render_container(ctx: &mut HtmlContext, container: &Container) {
    debug!("Rendering container '{}'", container.ctype().name());

    match container.ctype() {
        // We wrap with <rp> around the <rt> contents
        ContainerType::RubyText => {
            ctx.html().rp().contents("(");
            render_container_internal(ctx, container);
            ctx.html().rp().contents(")");
        }

        // Render normally
        _ => render_container_internal(ctx, container),
    }
}

pub fn render_container_internal(ctx: &mut HtmlContext, container: &Container) {
    // Get HTML tag type for this type of container
    let layout = ctx.layout();
    let tag_spec = container.ctype().html_tag(layout, ctx);

    // Get correct ID, based on the render setting
    let random_id = choose_id(ctx, &tag_spec);

    // Build the tag
    let mut tag = ctx.html().tag(tag_spec.tag());

    // Merge the class attribute with the container's class, if it conflicts
    match tag_spec {
        HtmlTag::Tag(_) => tag.attr(attr!(;; container.attributes())),
        HtmlTag::TagAndClass { class, .. } => tag.attr(attr!(
            "class" => class;;
            container.attributes(),
        )),
        HtmlTag::TagAndStyle { style, .. } => tag.attr(attr!(
            "style" => style;;
            container.attributes(),
        )),
        HtmlTag::TagAndId { id, .. } => {
            let id = random_id.as_deref().unwrap_or(&id);
            tag.attr(attr!("id" => id;; container.attributes()))
        }
    };

    // Add container internals
    tag.contents(container.elements());
}

pub fn render_color(ctx: &mut HtmlContext, color: &str, elements: &[Element]) {
    debug!("Rendering color container (color '{color}')");

    ctx.html()
        .span()
        .attr(attr!("style" => "color: " color ";"))
        .inner(|ctx| render_elements(ctx, elements));
}

fn choose_id(ctx: &mut HtmlContext, tag_spec: &HtmlTag) -> Option<String> {
    // If we're in a situation where we want a randomly generated ID
    if matches!(tag_spec, HtmlTag::TagAndId { .. }) && !ctx.settings().use_true_ids {
        Some(ctx.random().generate_html_id())
    } else {
        None
    }
}

#[test]
fn container_rendering_covers_special_tag_variants() {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::render::html::HtmlRender;
    use crate::settings::{WikitextMode, WikitextSettings};
    use crate::tree::{
        Alignment, AttributeMap, Container, Heading, HeadingLevel, SyntaxTree,
    };

    let page_info = PageInfo::dummy();
    let wikijump = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);

    let wikijump_tree = SyntaxTree {
        elements: vec![
            Element::Container(Container::new(
                ContainerType::RubyText,
                vec![Element::Text(cow!("go"))],
                AttributeMap::new(),
            )),
            Element::Container(Container::new(
                ContainerType::Monospace,
                vec![Element::Text(cow!("mono"))],
                AttributeMap::new(),
            )),
            Element::Container(Container::new(
                ContainerType::Header(Heading {
                    level: HeadingLevel::Two,
                    has_toc: true,
                }),
                vec![Element::Text(cow!("Heading"))],
                AttributeMap::new(),
            )),
        ],
        ..SyntaxTree::default()
    };
    let wikijump_output = HtmlRender.render(&wikijump_tree, &page_info, &wikijump);

    assert!(
        wikijump_output
            .body
            .contains("<rp>(</rp><rt>go</rt><rp>)</rp>")
    );
    assert!(
        wikijump_output
            .body
            .contains(r#"<code class="wj-monospace">mono</code>"#)
    );
    assert!(
        wikijump_output
            .body
            .contains(r#"<h2 id="toc0">Heading</h2>"#)
    );

    let wikidot = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let wikidot_tree = SyntaxTree {
        elements: vec![Element::Container(Container::new(
            ContainerType::Align(Alignment::Right),
            vec![Element::Text(cow!("right"))],
            AttributeMap::new(),
        ))],
        ..SyntaxTree::default()
    };
    let wikidot_output = HtmlRender.render(&wikidot_tree, &page_info, &wikidot);

    assert_eq!(
        wikidot_output.body,
        r#"<div style="text-align: right;">right</div>"#
    );
}

#[test]
fn choose_id_generates_random_ids_only_when_true_ids_are_disabled() {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Handle;
    use crate::settings::{WikitextMode, WikitextSettings};
    use crate::tree::BibliographyList;

    let page_info = PageInfo::dummy();
    let handle = Handle;
    let bibliographies = BibliographyList::new();
    let tag_with_id = HtmlTag::with_id("h2", str!("toc0"));

    let mut random_id_settings =
        WikitextSettings::from_mode(WikitextMode::Draft, Layout::Wikijump);
    random_id_settings.use_true_ids = false;
    let mut random_id_ctx = HtmlContext::new(
        &page_info,
        &handle,
        &random_id_settings,
        &[],
        &[],
        &bibliographies,
        0,
    );
    assert_eq!(
        choose_id(&mut random_id_ctx, &tag_with_id).as_deref(),
        Some("wj-id-zvGvLlhGI6VEZFKj"),
    );

    let true_id_settings =
        WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
    let mut true_id_ctx = HtmlContext::new(
        &page_info,
        &handle,
        &true_id_settings,
        &[],
        &[],
        &bibliographies,
        0,
    );
    assert_eq!(choose_id(&mut true_id_ctx, &tag_with_id), None);
    assert_eq!(choose_id(&mut true_id_ctx, &HtmlTag::new("h2")), None);
}
