/*
 * parsing/rule/impls/block/blocks/align.rs
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
use crate::tree::{Alignment, AttributeMap};

macro_rules! make_align_block {
    ($block_const:ident, $block_name:expr, $symbol:expr, $align:ident) => {
        use super::align::parse_alignment_block;
        use super::prelude::*;
        use crate::tree::Alignment;

        pub const $block_const: BlockRule = BlockRule {
            name: $block_name,
            accepts_names: &[$symbol],
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
            parse_alignment_block(
                (&$block_const, Alignment::$align),
                parser,
                name,
                flag_star,
                flag_score,
                in_head,
            )
        }
    };
}

pub fn parse_alignment_block<'r, 't>(
    (block_rule, alignment): (&BlockRule, Alignment),
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    let block_name = block_rule.name;
    debug!(
        "Parsing align block {name}/{block_name}/{}, in-head {in_head}",
        alignment.name(),
    );
    assert!(!flag_star, "Alignment block doesn't allow star flag");
    assert!(!flag_score, "Alignment block doesn't allow score flag");
    assert_block_name(block_rule, name);

    let body_start = parser.get_head_none_with_body_start(block_rule, in_head)?;
    if !parser.has_body_end_block(block_rule) {
        return Err(parser.make_end_of_input_err());
    }

    // Get body content, with paragraphs
    let (elements, errors, _) = parser
        .get_body_elements_with_context(block_rule, true, body_start)?
        .into();

    // Build element
    let element = Element::Container(Container::new(
        ContainerType::Align(alignment),
        elements,
        AttributeMap::new(),
    ));

    ok!(element, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};
    use std::time::{Duration, Instant};

    #[test]
    fn alignment_block_wraps_body_in_align_container() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[=]]centered[[/=]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let [Element::Container(container)] = tree.elements.as_slice() else {
            panic!("expected alignment container, got {:?}", tree.elements);
        };
        assert_eq!(container.ctype(), ContainerType::Align(Alignment::Center));
        let [Element::Container(paragraph)] = container.elements() else {
            panic!("expected paragraph body, got {:?}", container.elements());
        };
        assert_eq!(paragraph.ctype(), ContainerType::Paragraph);
        assert_eq!(paragraph.elements(), &[text!("centered")]);
    }

    #[test]
    fn quoted_alignment_blocks_remain_native_and_bounded() {
        const BLOCK_COUNT: usize = 64;

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let mut input = String::new();
        for index in 0..BLOCK_COUNT {
            input.push_str("> [[>]]\n> right-");
            input.push_str(&index.to_string());
            input.push_str("\n> [[/>]]\n> after-");
            input.push_str(&index.to_string());
            input.push('\n');
        }
        input.push_str("outside-sentinel\n");

        let started = Instant::now();
        let tokenization = crate::tokenize(&input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html.matches("text-align: right;").count(), BLOCK_COUNT);
        for index in 0..BLOCK_COUNT {
            assert!(html.contains(&format!("right-{index}")), "{html}");
            assert!(html.contains(&format!("after-{index}")), "{html}");
        }
        assert!(html.contains("outside-sentinel"), "{html}");
        assert!(!html.contains("[[>]]"), "{html}");
        assert!(!html.contains("[[/>]]"), "{html}");
    }

    #[test]
    fn unclosed_quoted_alignment_blocks_fail_closed_in_bounded_time() {
        const BLOCK_COUNT: usize = 512;

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let mut input = String::new();
        for index in 0..BLOCK_COUNT {
            input.push_str("> [[>]] malformed-");
            input.push_str(&index.to_string());
            input.push('\n');
        }
        input.push_str("outside-sentinel\n");

        let started = Instant::now();
        let tokenization = crate::tokenize(&input);
        let (tree, _errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(started.elapsed() < Duration::from_secs(5));
        for index in 0..BLOCK_COUNT {
            assert!(html.contains(&format!("malformed-{index}")), "{html}");
        }
        assert!(html.contains("outside-sentinel"), "{html}");
        assert_eq!(html.matches("[[&gt;]]").count(), BLOCK_COUNT, "{html}");
        assert!(!html.contains("text-align: right;"), "{html}");
    }
}
