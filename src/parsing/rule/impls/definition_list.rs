/*
 * parsing/rule/impls/definition_list.rs
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
use crate::parsing::{Token, strip_whitespace};
use crate::tree::DefinitionListItem;

pub const RULE_DEFINITION_LIST: Rule = Rule {
    name: "definition-list",
    position: LineRequirement::StartOfLine,
    try_consume_fn: parse_definition_list,
};

pub const RULE_DEFINITION_LIST_SKIP_NEWLINE: Rule = Rule {
    name: "definition-list-skip-newline",
    position: LineRequirement::Any,
    try_consume_fn: skip_newline,
};

fn skip_newline<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Seeing if we skip due to an upcoming definition list");

    match parser.next_three_tokens() {
        // It looks like a definition list is upcoming
        (Token::LineBreak, Some(Token::Colon), Some(Token::Whitespace)) => {
            ok!(Elements::None)
        }

        // Anything else
        _ => Err(parser.make_err(ParseErrorKind::RuleFailed)),
    }
}

fn parse_definition_list<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create a definition list");

    let mut items = Vec::new();
    let mut errors = Vec::new();
    let mut _paragraph_safe = false;

    // Definition list needs at least one item
    let (item, at_end) = parse_item(parser)?.chain(&mut errors, &mut _paragraph_safe);

    items.push(item);

    // Collect remainder, halting if there's a failure
    let mut at_end = at_end;
    while !at_end {
        let next = parse_next_item(parser, &mut errors, &mut _paragraph_safe);
        if let Some((item, item_at_end)) = next {
            items.push(item);
            at_end = item_at_end;
        } else {
            at_end = true;
        }
    }

    // Build and return element
    ok!(Element::DefinitionList(items))
}

fn parse_next_item<'r, 't>(
    parser: &mut Parser<'r, 't>,
    errors: &mut Vec<ParseError>,
    paragraph_safe: &mut bool,
) -> Option<(DefinitionListItem<'t>, bool)>
where
    'r: 't,
{
    let sub_parser = &mut parser.clone();
    match parse_item(sub_parser) {
        Ok(success) => {
            trace!("Retrieved definition list item");
            let item = success.chain(errors, paragraph_safe);
            parser.update(sub_parser);
            Some(item)
        }
        Err(error) => {
            warn!("Definition list ended: {error:?}");
            None
        }
    }
}

fn parse_item<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, (DefinitionListItem<'t>, bool)> {
    trace!("Trying to parse a definition list item pair");

    let mut errors = Vec::new();
    let mut _paragraph_safe = false;

    // The pattern for a definition list row is:
    // : key : value \n

    // Ensure the start of the line
    if !parser.start_of_line() {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    // Ensure that it matches expected token state
    if !starts_definition_item(parser) {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    parser.step_n(2)?;

    let key = collect_key(parser, &mut errors, &mut _paragraph_safe)?;
    let value = collect_value(parser, &mut errors, &mut _paragraph_safe)?;
    let (key_string, key_elements) = key;
    let (value_elements, last) = value;

    // collect_value only stops on line, paragraph, or input boundaries.
    let should_break = matches!(last.token, Token::ParagraphBreak | Token::InputEnd);

    // Build and return
    let key_string = std::borrow::Cow::Borrowed(key_string);
    let item = DefinitionListItem {
        key_string,
        key_elements,
        value_elements,
    };

    ok!(false; (item, should_break), errors)
}

fn starts_definition_item<'r, 't>(parser: &Parser<'r, 't>) -> bool {
    parser.next_two_tokens() == (Token::Colon, Some(Token::Whitespace))
}

fn collect_key<'r, 't>(
    parser: &mut Parser<'r, 't>,
    errors: &mut Vec<ParseError>,
    paragraph_safe: &mut bool,
) -> Result<(&'t str, Vec<Element<'t>>), ParseError>
where
    'r: 't,
{
    let start_token = parser.current();
    let close = [ParseCondition::token_pair(Token::Whitespace, Token::Colon)];
    let invalid = [
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::current(Token::LineBreak),
    ];
    let rule = RULE_DEFINITION_LIST;

    let collected = collect_consume(parser, rule, &close, &invalid, None)?;
    let mut key_elements = collected.chain(errors, paragraph_safe);
    let end_token = parser.current();

    strip_whitespace(&mut key_elements);
    parser.step_n(2)?;

    let key_string = parser
        .full_text()
        .slice_partial(start_token, end_token)
        .trim();
    Ok((key_string, key_elements))
}

fn collect_value<'r, 't>(
    parser: &mut Parser<'r, 't>,
    errors: &mut Vec<ParseError>,
    paragraph_safe: &mut bool,
) -> Result<(Vec<Element<'t>>, &'r ExtractedToken<'t>), ParseError>
where
    'r: 't,
{
    let close = [
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::current(Token::LineBreak),
        ParseCondition::current(Token::InputEnd),
    ];
    let rule = RULE_DEFINITION_LIST;

    let collected = collect_consume_keep(parser, rule, &close, &[], None)?;
    let (mut value_elements, last) = collected.chain(errors, paragraph_safe);

    strip_whitespace(&mut value_elements);
    Ok((value_elements, last))
}

#[cfg(test)]
#[derive(Debug)]
struct TestLogger;

#[cfg(test)]
impl log::Log for TestLogger {
    fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
        true
    }

    // This test logger only exercises logging call sites; it does not capture output.
    fn log(&self, _record: &log::Record<'_>) {}

    fn flush(&self) {}
}

#[cfg(test)]
static TEST_LOGGER: TestLogger = TestLogger;

#[cfg(test)]
static TEST_LOGGER_INIT: std::sync::Once = std::sync::Once::new();

#[cfg(test)]
fn enable_test_logging() {
    TEST_LOGGER_INIT.call_once(|| {
        let _ = log::set_logger(&TEST_LOGGER);
        log::set_max_level(log::LevelFilter::Trace);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn definition_list_stops_before_following_paragraph() {
        enable_test_logging();

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(": Key : Value\nPlain paragraph");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty());
        assert_eq!(tree.elements.len(), 2);

        match &tree.elements[0] {
            Element::DefinitionList(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].key_string, "Key");
                assert_eq!(items[0].key_elements, [text!("Key")]);
                assert_eq!(items[0].value_elements, [text!("Value")]);
            }
            other => panic!("expected definition list, got {other:?}"),
        }

        match &tree.elements[1] {
            Element::Container(container) => {
                assert_eq!(
                    container.elements(),
                    &[text!("Plain"), text!(" "), text!("paragraph")],
                );
            }
            other => panic!("expected following paragraph, got {other:?}"),
        }
    }

    #[test]
    fn definition_list_collects_multiple_items_through_input_end() {
        enable_test_logging();

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(": Alpha : One\n: Beta : Two");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty());
        assert_eq!(tree.elements.len(), 1);

        match &tree.elements[0] {
            Element::DefinitionList(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].key_string, "Alpha");
                assert_eq!(items[0].key_elements, [text!("Alpha")]);
                assert_eq!(items[0].value_elements, [text!("One")]);
                assert_eq!(items[1].key_string, "Beta");
                assert_eq!(items[1].key_elements, [text!("Beta")]);
                assert_eq!(items[1].value_elements, [text!("Two")]);
            }
            other => panic!("expected definition list, got {other:?}"),
        }
    }

    #[test]
    fn parse_item_rejects_non_definition_starts() {
        enable_test_logging();

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        let tokenization = crate::tokenize("alpha beta");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser.step().expect("identifier should follow input start");
        parser.step().expect("whitespace should follow identifier");

        let error = parse_item(&mut parser)
            .expect_err("mid-line whitespace should not start an item");
        assert_eq!(error.kind(), ParseErrorKind::RuleFailed);

        let tokenization = crate::tokenize(":Key:Value");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser.step().expect("colon should follow input start");
        parser.set_rule(RULE_DEFINITION_LIST);

        let error =
            parse_item(&mut parser).expect_err("missing space should reject the item");
        assert_eq!(error.kind(), ParseErrorKind::RuleFailed);
    }
}
