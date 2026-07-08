/*
 * parsing/rule/impls/blockquote.rs
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
use crate::parsing::paragraph::ParagraphStack;
use crate::parsing::{DepthItem, DepthList, process_depths};
use crate::tree::{AttributeMap, Container, ContainerType};

const MAX_BLOCKQUOTE_DEPTH: usize = 30;

pub const RULE_BLOCKQUOTE: Rule = Rule {
    name: "blockquote",
    position: LineRequirement::StartOfLine,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    // Context variables
    let mut depths = Vec::new();
    let mut errors = Vec::new();

    // Produce a depth list with elements
    loop {
        let current = parser.current();
        if current.token != Token::Quote {
            break std::convert::identity(());
        }

        // 1 or more ">"s in one token. Return ASCII length.
        let depth = current.slice.len();
        parser.step()?;
        parser.get_optional_space()?; // allow whitespace after ">"
        parser.mark_virtual_start_of_line();

        // Check that the depth isn't obscenely deep, to avoid DOS attacks via stack overflow.
        if depth > MAX_BLOCKQUOTE_DEPTH {
            return Err(parser.make_err(ParseErrorKind::BlockquoteDepthExceeded));
        }

        // Parse elements until we hit the end of the line
        let mut paragraph_safe = true;
        let close_conditions = [
            ParseCondition::current(Token::LineBreak),
            ParseCondition::current(Token::ParagraphBreak),
            ParseCondition::current(Token::InputEnd),
        ];
        let close = &close_conditions;
        let result = collect_consume(parser, RULE_BLOCKQUOTE, close, &[], None)?;
        let mut elements = result.chain(&mut errors, &mut paragraph_safe);

        // Add a line break for the end of the line
        elements.push(Element::LineBreak);

        // Append blockquote line
        //
        // Depth lists expect zero-based list depths, but tokens are one-based.
        // So, we subtract one.
        //
        // This will not overflow because Token::Quote requires at least one ">".
        depths.push((depth - 1, (), (elements, paragraph_safe)))
    }

    // This blockquote has no rows, so the rule fails
    if depths.is_empty() {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let depth_lists = process_depths((), depths);
    let elements: Vec<Element> = depth_lists
        .into_iter()
        .map(|(_, depth_list)| build_blockquote_element(depth_list))
        .collect();

    ok!(false; elements, errors)
}

fn build_blockquote_element(list: DepthList<(), (Vec<Element>, bool)>) -> Element {
    let mut stack = ParagraphStack::new();

    // Convert depth list into a list of elements
    for item in list {
        match item {
            DepthItem::Item((elements, paragraph_safe)) => {
                for element in elements {
                    stack.push_element(element, paragraph_safe);
                }
            }
            DepthItem::List(_, list) => {
                let blockquote = build_blockquote_element(list);
                stack.pop_line_break();
                stack.push_element(blockquote, false);
            }
        }
    }

    stack.pop_line_break();

    Element::Container(Container::new(
        ContainerType::Blockquote,
        stack.into_elements(),
        AttributeMap::new(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender, text::TextRender};
    use crate::settings::{WikitextMode, WikitextSettings};
    use std::sync::Once;

    #[derive(Debug)]
    struct TestLogger;

    impl log::Log for TestLogger {
        fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            let _ = record.args().to_string();
        }

        fn flush(&self) {}
    }

    static TEST_LOGGER: TestLogger = TestLogger;
    static INIT_LOGGER: Once = Once::new();

    fn enable_test_logging() {
        INIT_LOGGER.call_once(|| {
            let _ = log::set_logger(&TEST_LOGGER);
            log::set_max_level(log::LevelFilter::Trace);
        });
    }

    #[test]
    fn native_blockquote_rejects_excessive_depth() {
        enable_test_logging();

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = format!("{} too deep", ">".repeat(MAX_BLOCKQUOTE_DEPTH + 1));
        let tokenization = crate::tokenize(&input);
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("quote token should follow input start");
        parser.set_rule(RULE_BLOCKQUOTE);

        let error = RULE_BLOCKQUOTE
            .try_consume(&mut parser)
            .expect_err("excessive blockquote depth should fail");
        assert_eq!(error.kind(), ParseErrorKind::BlockquoteDepthExceeded);
    }

    #[test]
    fn native_blockquote_rejects_non_quote_start() {
        enable_test_logging();

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("plain");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("identifier token should follow input start");
        parser.set_rule(RULE_BLOCKQUOTE);

        let error = RULE_BLOCKQUOTE
            .try_consume(&mut parser)
            .expect_err("non-quote input should not produce a blockquote");
        assert_eq!(error.kind(), ParseErrorKind::RuleFailed);
    }

    #[test]
    fn native_blockquote_content_respects_virtual_line_start_for_headings() {
        enable_test_logging();

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = concat!(
            "> + {{WARNING}}\n",
            ">\n",
            "> {{Body}}\n",
            ">\n",
            "> ++ {{LEVEL 5 AUTHORIZATION REQUIRED}}\n",
        );
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");

        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("<h1"));
        assert!(html.contains("WARNING"));
        assert!(html.contains("<h2"));
        assert!(html.contains("LEVEL 5 AUTHORIZATION REQUIRED"));
        assert!(!html.contains("+ <tt>WARNING</tt>"));
        assert!(!html.contains("++ <tt>LEVEL 5 AUTHORIZATION REQUIRED</tt>"));

        let text = TextRender.render(&tree, &page_info, &settings);
        assert!(text.contains("WARNING"));
        assert!(text.contains("LEVEL 5 AUTHORIZATION REQUIRED"));
        assert!(!text.contains("+ WARNING"));
        assert!(!text.contains("++ LEVEL 5 AUTHORIZATION REQUIRED"));
    }
}
