/*
 * parsing/rule/impls/rawold.rs
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
use std::borrow::Cow;

pub const RULE_RAWOLD: Rule = Rule {
    name: "rawold",
    position: LineRequirement::Any,
    try_consume_fn,
};

pub(crate) fn backtick_starts_rawold(parser: &Parser<'_, '_>) -> bool {
    if !parser.settings().layout.legacy() {
        return false;
    }

    let source = parser.full_text().inner();
    let start = parser.current().span.start;
    let bytes = source.as_bytes();
    if start > 0 && bytes[start - 1] == b'`' {
        return false;
    }
    if bytes.get(start + 1) != Some(&b'`') {
        return false;
    }

    let body_start = start + 2;
    let Some(offset) = source.get(body_start..).and_then(|rest| rest.find("``")) else {
        return false;
    };
    let body_end = body_start + offset;
    let body = &source[body_start..body_end];
    !body.is_empty()
        && !body
            .chars()
            .any(|c| matches!(c, '\'' | '\u{2018}' | '\u{2019}'))
        && !body.ends_with('`')
}

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    if !backtick_starts_rawold(parser) {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let source = parser.full_text().inner();
    let body_start = parser.current().span.start + 2;
    let body_end = body_start + source[body_start..].find("``").unwrap();
    let body = &source[body_start..body_end];
    let end = body_end + 2;
    while parser.current().span.start < end {
        if matches!(
            parser.current().token,
            Token::InputEnd | Token::Raw | Token::LeftRaw | Token::RightRaw
        ) {
            return Err(parser.make_err(ParseErrorKind::RuleFailed));
        }
        parser.step()?;
    }
    ok!(Element::Text(Cow::Owned(body.to_owned())))
}
