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

pub(crate) fn wikidot_code_token_body_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if start > 0 && bytes.get(start - 1) == Some(&b'`') {
        return None;
    }
    if bytes.get(start + 1) != Some(&b'`') {
        return None;
    }

    let body_start = start + 2;
    let mut cursor = body_start;
    while let Some(offset) = source.get(cursor..).and_then(|rest| rest.find("``")) {
        let body_end = cursor + offset;
        let body = &source[body_start..body_end];
        if !body.is_empty()
            && !body
                .chars()
                .any(|c| matches!(c, '\'' | '\u{2018}' | '\u{2019}'))
            && !body.ends_with('`')
        {
            return Some(body_end);
        }
        cursor = body_end + 1;
    }
    None
}

pub(crate) fn wikidot_code_token_at(source: &str, start: usize) -> bool {
    wikidot_code_token_body_end(source, start).is_some()
}

pub(crate) fn wikidot_code_token_end(source: &str, start: usize) -> Option<usize> {
    wikidot_code_token_body_end(source, start).map(|body_end| body_end + 2)
}

pub(crate) fn wikidot_code_token_contains(source: &str, position: usize) -> bool {
    let bytes = source.as_bytes();
    (0..source.len().min(position)).any(|start| {
        bytes[start] == b'`'
            && wikidot_code_token_body_end(source, start)
                .is_some_and(|body_end| position < body_end + 2)
    })
}

pub(crate) fn contains_wikidot_code_token(value: &str) -> bool {
    let bytes = value.as_bytes();
    (0..bytes.len())
        .any(|start| bytes[start] == b'`' && wikidot_code_token_at(value, start))
}

pub(crate) fn backtick_starts_rawold(parser: &Parser<'_, '_>) -> bool {
    if !parser.settings().layout.legacy() {
        return false;
    }

    wikidot_code_token_at(parser.full_text().inner(), parser.current().span.start)
}

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    if !backtick_starts_rawold(parser) {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let source = parser.full_text().inner();
    let body_start = parser.current().span.start + 2;
    let body_end = wikidot_code_token_body_end(source, parser.current().span.start)
        .expect("rawold opener check found a closing token");
    let body = &source[body_start..body_end];
    let end = body_end + 2;
    while parser.current().span.start < end {
        if parser.current().token == Token::InputEnd {
            return Err(parser.make_err(ParseErrorKind::RuleFailed));
        }
        parser.step()?;
    }
    let escapes = crate::wikidot_raw_escape::raw_escapes(body);
    let opaque_body = matches!(body, "**" | "//" | "__" | "^^" | ",," | "{{")
        || (!body.is_empty() && body.bytes().all(|byte| byte == b'-'));
    if escapes.is_empty() {
        let text = Cow::Owned(body.to_owned());
        return ok!(if opaque_body {
            Element::LiteralText(text)
        } else {
            Element::Text(text)
        });
    }
    let mut literal = String::with_capacity(body.len());
    let mut cursor = 0;
    for escape in escapes {
        literal.push_str(&body[cursor..escape.start]);
        cursor = escape.end;
    }
    literal.push_str(&body[cursor..]);
    let text = Cow::Owned(literal);
    ok!(if opaque_body {
        Element::LiteralText(text)
    } else {
        Element::Text(text)
    })
}
