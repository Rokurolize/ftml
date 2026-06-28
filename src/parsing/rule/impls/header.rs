/*
 * parsing/rule/impls/header.rs
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
use crate::tree::Heading;
use std::convert::TryInto;

pub const RULE_HEADER: Rule = Rule {
    name: "header",
    position: LineRequirement::StartOfLine,
    try_consume_fn,
};

fn step_expected<'r, 't>(
    parser: &mut Parser<'r, 't>,
    token: Token,
) -> Result<&'r ExtractedToken<'t>, ParseError> {
    let current = parser.current();
    if current.token != token {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    parser.step()?;
    Ok(current)
}

fn heading_from_token(token: &ExtractedToken<'_>) -> Heading {
    token
        .slice
        .try_into()
        .expect("Received invalid heading length token slice")
}

fn consume_header_once<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create header container");

    // Get header depth
    let heading_token = step_expected(parser, Token::Heading)?;
    let heading = heading_from_token(heading_token);

    // Step over whitespace
    step_expected(parser, Token::Whitespace)?;

    let (elements, all_errors, _) = collect_container(
        parser,
        RULE_HEADER,
        ContainerType::Header(heading),
        &[
            ParseCondition::current(Token::InputEnd),
            ParseCondition::current(Token::LineBreak),
            ParseCondition::current(Token::ParagraphBreak),
        ],
        &[],
        None,
    )?
    .into();

    // If this heading wants a table of contents (TOC) entry, then add one
    if heading.has_toc {
        // collect_container() always produces one Element::Container.
        // We unwrap it so we can get the elements composing the name.
        let elements = match elements {
            Elements::Single(Element::Container(ref container)) => container.elements(),
            _ => panic!("Collected heading produced a non-single non-container element"),
        };

        // Create table of contents entry with the given level and name.
        parser.push_table_of_contents_entry(heading.level, elements);
    }

    // Build final Elements object
    ok!(false; elements, all_errors)
}

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    let success = consume_header_once(parser)?;
    let (elements, mut all_errors, _) = success.into();
    let mut all_elements: Vec<_> = elements.into_iter().collect();

    loop {
        let parser_state = parser.get_mutable_state();
        let mut sub_parser = parser.clone_with_rule(RULE_HEADER);

        let Ok(success) = consume_header_once(&mut sub_parser) else {
            parser.reset_mutable_state(parser_state);
            break;
        };

        parser.update(&sub_parser);

        let (elements, mut errors, _) = success.into();
        all_elements.extend(elements);
        all_errors.append(&mut errors);
    }

    ok!(false; all_elements, all_errors)
}
