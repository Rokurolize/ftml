/*
 * parsing/rule/impls/block/blocks/html.rs
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

pub const BLOCK_HTML: BlockRule = BlockRule {
    name: "block-html",
    accepts_names: &["html"],
    accepts_star: false,
    accepts_score: false,
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
    debug!("Parsing HTML block (in-head {in_head})");
    assert!(!flag_star, "HTML doesn't allow star flag");
    assert!(!flag_score, "HTML doesn't allow score flag");
    assert_block_name(&BLOCK_HTML, name);

    let arguments = parser.get_head_map(&BLOCK_HTML, in_head)?;
    let html = parser.get_body_text(&BLOCK_HTML)?;
    let element = Element::Html {
        contents: html.clone(),
        attributes: arguments.to_attribute_map(parser.settings()),
    };
    parser.push_html_block(html);
    ok!(element)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn html_block_tracks_body_and_element_contents() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[html]]\n<strong>raw</strong>\n[[/html]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let [
            Element::Html {
                contents,
                attributes,
            },
        ] = tree.elements.as_slice()
        else {
            panic!("expected one HTML block element, got {:?}", tree.elements);
        };
        let [tracked_html] = tree.html_blocks.as_slice() else {
            panic!(
                "expected one tracked HTML block, got {:?}",
                tree.html_blocks
            );
        };

        assert_eq!(contents, "<strong>raw</strong>");
        assert!(attributes.get().is_empty());
        assert_eq!(tracked_html, "<strong>raw</strong>");
    }
}
