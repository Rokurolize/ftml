/*
 * parsing/rule/impls/list.rs
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
use crate::parsing::{DepthItem, DepthList, process_depths};
use crate::tree::{AttributeMap, ListItem, ListType};

const MAX_LIST_DEPTH: usize = 20;

const fn get_list_type(token: Token) -> Option<ListType> {
    match token {
        Token::BulletItem => Some(ListType::Bullet),
        Token::NumberedItem => Some(ListType::Numbered),
        _ => None,
    }
}

enum ListItemStep<'t> {
    End,
    Skip {
        ends_run: bool,
    },
    Item {
        item: (usize, ListType, Vec<Element<'t>>),
        ends_run: bool,
    },
}

pub const RULE_LIST: Rule = Rule {
    name: "list",
    position: LineRequirement::StartOfLine,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    // Context variables
    let mut depths = Vec::new();
    let mut errors = Vec::new();

    // Blockquotes are always paragraph-unsafe,
    // but we need this binding for chain().
    let mut paragraph_safe = false;

    let mut ended = false;
    let mut skipped_empty_rows = false;
    while !ended {
        match parse_next_list_item(parser, &mut errors, &mut paragraph_safe)? {
            ListItemStep::End => ended = true,
            ListItemStep::Skip { ends_run } => {
                skipped_empty_rows = true;
                ended = ends_run;
            }
            ListItemStep::Item { item, ends_run } => {
                depths.push(item);
                ended = ends_run;
            }
        }
    }

    // This list has no rows, so the rule fails
    if depths.is_empty() {
        return if skipped_empty_rows {
            ok!(true; Elements::None, errors)
        } else {
            Err(parser.make_err(ParseErrorKind::RuleFailed))
        };
    }

    let depth_lists = process_depths(ListType::Generic, depths);
    let elements: Vec<Element> = depth_lists
        .into_iter()
        .map(|(ltype, depth_list)| build_list_element(ltype, depth_list))
        .collect();

    ok!(paragraph_safe; elements, errors)
}

fn parse_next_list_item<'r, 't>(
    parser: &mut Parser<'r, 't>,
    errors: &mut Vec<ParseError>,
    paragraph_safe: &mut bool,
) -> Result<ListItemStep<'t>, ParseError>
where
    'r: 't,
{
    let mut sub_parser = parser.clone();

    let Some(depth) = parse_list_depth(&mut sub_parser)? else {
        return Ok(ListItemStep::End);
    };

    if depth > MAX_LIST_DEPTH {
        return Err(parser.make_err(ParseErrorKind::ListDepthExceeded));
    }

    let Some(list_type) = get_list_type(sub_parser.current().token) else {
        return Ok(ListItemStep::End);
    };
    sub_parser.step()?;

    if sub_parser.current().token != Token::Whitespace {
        return Ok(ListItemStep::End);
    }
    sub_parser.step()?;

    let item_result = collect_list_item_elements(&mut sub_parser)?;
    let (elements, ends_run) = item_result.chain(errors, paragraph_safe);
    if elements.is_empty() {
        parser.update(&sub_parser);
        return Ok(ListItemStep::Skip { ends_run });
    }

    parser.update(&sub_parser);
    Ok(ListItemStep::Item {
        item: (depth, list_type, elements),
        ends_run,
    })
}

fn parse_list_depth<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<Option<usize>, ParseError> {
    let current = parser.current();
    let depth = match current.token {
        Token::Whitespace => {
            let depth = current.slice.len();
            parser.step()?;
            depth
        }
        Token::BulletItem | Token::NumberedItem => 0,
        _ => {
            return Ok(None);
        }
    };
    Ok(Some(depth))
}

fn collect_list_item_elements<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, (Vec<Element<'t>>, bool)> {
    let close_conditions = [
        ParseCondition::current(Token::LineBreak),
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::current(Token::InputEnd),
    ];

    let result = collect_consume_keep(parser, RULE_LIST, &close_conditions, &[], None)?;
    Ok(result.map(|(elements, last)| {
        let ends_run = matches!(last.token, Token::ParagraphBreak | Token::InputEnd);
        (elements, ends_run)
    }))
}

fn build_list_element(
    top_ltype: ListType,
    list: DepthList<ListType, Vec<Element>>,
) -> Element {
    let build_item = |item| match item {
        DepthItem::Item(elements) => ListItem::Elements {
            elements,
            attributes: AttributeMap::new(),
        },
        DepthItem::List(ltype, list) => ListItem::SubList {
            element: Box::new(build_list_element(ltype, list)),
        },
    };

    let items = list.into_iter().map(build_item).collect();
    let attributes = AttributeMap::new();

    // Return the Element::List object
    Element::List {
        ltype: top_ltype,
        items,
        attributes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
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

    fn settings() -> (PageInfo<'static>, WikitextSettings) {
        (
            PageInfo::dummy(),
            WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot),
        )
    }

    #[test]
    fn native_list_rejects_excessive_depth() {
        enable_test_logging();

        let (page_info, settings) = settings();
        let input = format!("{}* too deep", " ".repeat(MAX_LIST_DEPTH + 1));
        let tokenization = crate::tokenize(&input);
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("leading whitespace should follow input start");
        parser.set_rule(RULE_LIST);

        let error = RULE_LIST
            .try_consume(&mut parser)
            .expect_err("excessive list depth should fail");
        assert_eq!(error.kind(), ParseErrorKind::ListDepthExceeded);
    }

    #[test]
    fn native_list_rejects_inputs_without_items() {
        enable_test_logging();

        let (page_info, settings) = settings();
        let tokenization = crate::tokenize("plain");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("identifier token should follow input start");
        parser.set_rule(RULE_LIST);

        let error = RULE_LIST
            .try_consume(&mut parser)
            .expect_err("plain text should not produce a list");
        assert_eq!(error.kind(), ParseErrorKind::RuleFailed);

        let tokenization = crate::tokenize("  plain");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("whitespace token should follow input start");
        parser.set_rule(RULE_LIST);

        let error = RULE_LIST
            .try_consume(&mut parser)
            .expect_err("indented text without a bullet should not produce a list");
        assert_eq!(error.kind(), ParseErrorKind::RuleFailed);
    }

    #[test]
    fn native_list_stops_on_malformed_and_skips_empty_items() {
        enable_test_logging();

        let (page_info, settings) = settings();
        let tokenization = crate::tokenize("*missing-space");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("bullet token should follow input start");
        parser.set_rule(RULE_LIST);

        let error = RULE_LIST
            .try_consume(&mut parser)
            .expect_err("missing post-bullet whitespace should not produce a list");
        assert_eq!(error.kind(), ParseErrorKind::RuleFailed);
        assert_eq!(parser.current().token, Token::BulletItem);
        assert_eq!(parser.current().slice, "*");

        let tokenization = crate::tokenize("* \n* item");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("bullet token should follow input start");
        parser.set_rule(RULE_LIST);

        let success = RULE_LIST
            .try_consume(&mut parser)
            .expect("list should skip the empty item and keep the non-empty item");
        let Elements::Multiple(elements) = success.item else {
            panic!("expected one list, got {:?}", success.item);
        };
        let [Element::List { ltype, items, .. }] = elements.as_slice() else {
            panic!("expected one list, got {elements:?}");
        };
        assert_eq!(*ltype, ListType::Bullet);
        assert_eq!(items.len(), 1);
        match &items[0] {
            ListItem::Elements { elements, .. } => assert_eq!(elements, &[text!("item")]),
            other => panic!("expected a list item, got {other:?}"),
        }

        let tokenization = crate::tokenize("* good\n*bad");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("bullet token should follow input start");
        parser.set_rule(RULE_LIST);

        let success = RULE_LIST
            .try_consume(&mut parser)
            .expect("valid first item should produce a list");
        let Elements::Multiple(elements) = success.item else {
            panic!("expected one list, got {:?}", success.item);
        };
        let [Element::List { items, .. }] = elements.as_slice() else {
            panic!("expected one list, got {elements:?}");
        };
        assert_eq!(items.len(), 1);
        assert_eq!(parser.current().token, Token::BulletItem);
        assert_eq!(parser.current().slice, "*");
    }

    #[test]
    fn native_list_paragraph_break_starts_a_sibling_run() {
        enable_test_logging();

        let (page_info, settings) = settings();
        let tokenization = crate::tokenize("* first\n\n* second");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("bullet token should follow input start");
        parser.set_rule(RULE_LIST);

        let success = RULE_LIST
            .try_consume(&mut parser)
            .expect("first list run should parse");
        let Elements::Multiple(elements) = success.item else {
            panic!("expected one list, got {:?}", success.item);
        };
        let [Element::List { items, .. }] = elements.as_slice() else {
            panic!("expected one list, got {elements:?}");
        };
        assert_eq!(items.len(), 1);
        assert_eq!(parser.current().token, Token::BulletItem);
        assert_eq!(parser.current().slice, "*");

        let tokenization = crate::tokenize("* first\n* second");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("bullet token should follow input start");
        parser.set_rule(RULE_LIST);
        let success = RULE_LIST
            .try_consume(&mut parser)
            .expect("contiguous list run should parse");
        let Elements::Multiple(elements) = success.item else {
            panic!("expected one list, got {:?}", success.item);
        };
        let [Element::List { items, .. }] = elements.as_slice() else {
            panic!("expected one list, got {elements:?}");
        };
        assert_eq!(items.len(), 2);
    }
}
