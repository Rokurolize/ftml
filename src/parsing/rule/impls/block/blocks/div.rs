/*
 * parsing/rule/impls/block/blocks/div.rs
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

pub const BLOCK_DIV: BlockRule = BlockRule {
    name: "block-div",
    accepts_names: &["div"],
    accepts_star: false,
    accepts_score: true,
    accepts_newlines: true,
    parse_fn,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Parsing div block (name '{name}', in-head {in_head}, score {flag_score})");
    assert!(!flag_star, "Div doesn't allow star flag");
    assert_block_name(&BLOCK_DIV, name);

    let (arguments, body_start) =
        parser.get_head_map_with_body_start(&BLOCK_DIV, in_head)?;

    // "div" means we wrap in paragraphs, like normal
    // "div_" means we don't wrap it
    let wrap_paragraphs = !flag_score;

    // Get body content, based on whether we want paragraphs or not.
    // Discard paragraph_safe, since divs never are.
    let (elements, errors, _) = parser
        .get_body_elements_with_context(&BLOCK_DIV, wrap_paragraphs, body_start)?
        .into();

    // Build element and return
    let element = Element::Container(Container::new(
        ContainerType::Div,
        elements,
        arguments.to_attribute_map(parser.settings()),
    ));

    ok!(element, errors)
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn quoted_multiline_div_with_quoted_close_remains_native_and_bounded() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = concat!(
            "> [[div style=\"font-weight: bold;\"]]\n",
            "> First quoted line.\n",
            "> \n",
            "> Second quoted line.\n",
            "> [[/div]]\n",
        );
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains("<blockquote>"), "{html}");
        assert!(html.contains("<div style="), "{html}");
        assert!(html.contains("First quoted line."), "{html}");
        assert!(html.contains("Second quoted line."), "{html}");
        assert!(!html.contains("[[div"), "{html}");
        assert!(!html.contains("[[/div]]"), "{html}");
    }

    #[test]
    fn quoted_div_with_close_on_same_line_remains_native() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = "> [[div class=\"notice\"]]Quoted body.[[/div]]\n";
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains("<blockquote>"), "{html}");
        assert!(html.contains("<div class=\"notice\">"), "{html}");
        assert!(html.contains("Quoted body."), "{html}");
        assert!(!html.contains("[[div"), "{html}");
    }

    #[test]
    fn quoted_scored_div_closes_without_absorbing_following_page() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = concat!(
            "> [[div_ class=\"notice\"]]\n",
            "> body\n",
            "> [[/div]]\n",
            "following page\n",
        );
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains("<div class=\"notice\">body</div>"), "{html}");
        assert!(html.contains("following page"), "{html}");
        assert!(!html.contains("[[/div]]"), "{html}");
        let div_end = html.find("</div>").expect("div close missing");
        let following = html.find("following page").expect("following text missing");
        assert!(div_end < following, "{html}");
    }
}
