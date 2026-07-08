/*
 * parsing/rule/impls/section_marker.rs
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

pub const RULE_SECTION_MARKER: Rule = Rule {
    name: "section-marker",
    position: LineRequirement::StartOfLine,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to consume section marker");

    let mut count = 0;
    while parser.current().token == Token::Equals {
        count += 1;
        parser.step()?;
    }

    if count < 4 {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    match parser.current().token {
        Token::LineBreak => {
            parser.step()?;
            ok!(Elements::None)
        }
        Token::ParagraphBreak | Token::InputEnd => ok!(Elements::None),
        _ => Err(parser.make_err(ParseErrorKind::RuleFailed)),
    }
}
