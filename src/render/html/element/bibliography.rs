/*
 * render/html/element/bibliography.rs
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
use crate::tree::Bibliography;

pub fn render_bibcite(ctx: &mut HtmlContext, label: &str, brackets: bool) {
    debug!("Rendering bibliography citation (label {label}, brackets {brackets})");

    if !ctx.enter_bibliography_ref(label) {
        warn!("Recursive bibliography citation detected for label {label}");
        render_missing_bibcite(ctx);
        return;
    }

    match ctx.get_bibliography_ref(label) {
        // Valid bibliography reference, render it
        Some((index, contents)) => {
            // TODO make this into a locale template string
            let reference_string = ctx
                .handle()
                .get_message(ctx.language(), "bibliography-reference");
            let label = format!("{reference_string} {index}.");

            // TODO: For now, copied from footnotes
            ctx.html()
                .span()
                .attr(attr!("class" => "wj-bibliography-ref"))
                .inner(|ctx| {
                    let id = str!(index);

                    // Bibliography marker that is hoverable
                    if brackets {
                        ctx.push_raw('[');
                    }

                    ctx.html()
                        .element("wj-bibliography-ref-marker")
                        .attr(attr!(
                            "class" => "wj-bibliography-ref-marker",
                            "role" => "link",
                            "aria-label" => &label,
                            "data-id" => &id,
                        ))
                        .contents(&id);

                    if brackets {
                        ctx.push_raw(']');
                    }

                    // Tooltip shown on hover.
                    // Is aria-hidden due to difficulty in getting a simultaneous
                    // tooltip and link to work. A screen reader can still navigate
                    // through to the link and read the bibliography directly.
                    ctx.html()
                        .span()
                        .attr(attr!(
                            "class" => "wj-bibliography-ref-tooltip",
                            "aria-hidden" => "true",
                        ))
                        .inner(|ctx| {
                            // Tooltip label
                            ctx.html()
                                .span()
                                .attr(
                                    attr!("class" => "wj-bibliography-ref-tooltip-label"),
                                )
                                .contents(&label);

                            // Actual tooltip contents
                            ctx.html()
                                .span()
                                .attr(attr!("class" => "wj-bibliography-ref-contents"))
                                .contents(contents);
                        });
                });
        }
        None => {
            render_missing_bibcite(ctx);
        }
    }

    ctx.exit_bibliography_ref(label);
}

fn render_missing_bibcite(ctx: &mut HtmlContext) {
    let message = ctx
        .handle()
        .get_message(ctx.language(), "bibliography-cite-not-found");

    ctx.html()
        .span()
        .attr(attr!("class" => "wj-error-inline"))
        .inner(|ctx| ctx.push_escaped(message));
}

pub fn render_bibliography(
    ctx: &mut HtmlContext,
    title: Option<&str>,
    bibliography_index: usize,
    bibliography: &Bibliography,
) {
    debug!(
        "Rendering bibliography block (title {}, items {})",
        title.unwrap_or("<default>"),
        bibliography.slice().len(),
    );

    let title_default;
    let title: &str = match title {
        Some(title) => title,
        None => {
            title_default = ctx
                .handle()
                .get_message(ctx.language(), "bibliography-block-title");

            title_default
        }
    };

    ctx.html()
        .div()
        .attr(attr!("class" => "wj-bibliography bibitems"))
        .inner(|ctx| {
            ctx.html()
                .div()
                .attr(attr!("class" => "wj-bibliography-title title"))
                .contents(title);

            let mut id = String::new();
            let mut class = String::new();
            for (entry_index, (_, elements)) in bibliography.slice().iter().enumerate() {
                // Convert to 1-indexing
                let bibliography_index = bibliography_index + 1;
                let entry_index = entry_index + 1;

                // Produce HTML ID
                id.clear();
                str_write!(
                    id,
                    "wj-bibliography-item-{bibliography_index}-{entry_index}",
                );
                class.clear();
                str_write!(
                    class,
                    "wj-bibliography-item bibitem bibitem-{bibliography_index}-{entry_index}",
                );

                // Make bibliography row
                ctx.html()
                    .div()
                    .attr(attr!("class" => &class, "id" => &id))
                    .inner(|ctx| {
                        // Number and clickable anchor
                        ctx.html()
                            .element("wj-bibliography-item-marker")
                            .attr(attr!(
                                "class" => "wj-bibliography-item-marker",
                                "type" => "button",
                                "role" => "link",
                            ))
                            .inner(|ctx| {
                                str_write!(ctx, "{entry_index}");

                                // Period after entry number. Has special class to permit styling.
                                ctx.html()
                                    .span()
                                    .attr(attr!("class" => "wj-bibliography-sep"))
                                    .inner(|ctx| ctx.push_raw('.'));
                            });

                        render_elements(ctx, elements);
                    });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::super::super::context::HtmlContext;
    use super::super::super::output::HtmlOutput;
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Handle;
    use crate::settings::{WikitextMode, WikitextSettings};
    use crate::tree::{BibliographyList, Element};

    #[test]
    fn bibliography_rendering_covers_missing_and_block_variants() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        let footnotes: [Vec<Element<'static>>; 0] = [];

        let mut bibliography = Bibliography::new();
        bibliography.add(cow!("alpha"), vec![text!("Alpha & source")]);

        let mut bibliographies = BibliographyList::new();
        bibliographies.push(bibliography);
        let bibliography_ref = bibliographies.get_bibliography(0);

        let mut ctx = HtmlContext::new(
            &page_info,
            &Handle,
            &settings,
            &[],
            &footnotes,
            &bibliographies,
            0,
        );

        render_bibcite(&mut ctx, "missing", false);
        render_bibcite(&mut ctx, "alpha", false);
        render_bibliography(&mut ctx, Some("Works Cited"), 0, bibliography_ref);

        let output = HtmlOutput::from(ctx);
        assert!(output.body.contains(r#"<span class="wj-error-inline">"#));
        assert!(output.body.contains("Bibliography item not found"));
        assert!(
            output.body.contains(
                r#"<div class="wj-bibliography-title title">Works Cited</div>"#
            )
        );
        assert!(
            output
                .body
                .contains(r#"<span class="wj-bibliography-sep">.</span>"#)
        );
        assert!(output.body.contains("Alpha &amp; source"));
    }
}
