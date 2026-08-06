/*
 * render/html/element/definition_list.rs
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
use crate::tree::DefinitionListItem;

pub fn render_definition_list(ctx: &mut HtmlContext, items: &[DefinitionListItem]) {
    debug!("Rendering definition list (length {})", items.len());

    ctx.html().dl().inner(|ctx| {
        if ctx.layout().legacy() {
            ctx.push_raw('\n');
        }
        for DefinitionListItem {
            key_elements,
            value_elements,
            ..
        } in items
        {
            if !key_elements.is_empty() {
                ctx.html().dt().contents(key_elements);
                if ctx.layout().legacy() {
                    ctx.push_raw('\n');
                }
            }
            ctx.html().dd().contents(value_elements);
            if ctx.layout().legacy() {
                ctx.push_raw('\n');
            }
        }
    });
    if ctx.layout().legacy() {
        ctx.push_raw('\n');
    }
}

#[test]
fn wikidot_definition_list_omits_an_empty_term_tag() {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::render::html::HtmlRender;
    use crate::settings::{WikitextMode, WikitextSettings};

    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let tokenization = crate::tokenize(":  : Value");
    let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(
        HtmlRender.render(&tree, &page_info, &settings).body,
        "<dl>\n<dd>Value</dd>\n</dl>\n",
    );
}

#[test]
fn wikidot_definition_lists_keep_tag_separating_newlines() {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::render::html::HtmlRender;
    use crate::settings::{WikitextMode, WikitextSettings};

    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let tokenization = crate::tokenize(": Alpha : One\n: Beta : Two");
    let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
    assert!(errors.is_empty());

    let html = HtmlRender.render(&tree, &page_info, &settings).body;
    assert_eq!(
        html,
        "<dl>\n<dt>Alpha</dt>\n<dd>One</dd>\n<dt>Beta</dt>\n<dd>Two</dd>\n</dl>\n",
    );
}
