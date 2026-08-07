/*
 * parsing/rule/impls/math.rs
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
use super::raw::RULE_RAW;

pub const RULE_MATH: Rule = Rule {
    name: "math",
    position: LineRequirement::Any,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create inline math equation");
    assert_step(parser, Token::LeftMath)?;
    let close = [ParseCondition::current(Token::RightMath)];
    let invalid = [
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::current(Token::LineBreak),
    ];
    let source = if parser.settings().layout.legacy() {
        collect_wikidot_math_source(parser, &close, &invalid)?
    } else {
        collect_text(parser, RULE_MATH, &close, &invalid, None)?.trim()
    };

    let element = Element::MathInline {
        latex_source: std::borrow::Cow::Borrowed(source),
    };
    success_elements(element)
}

fn collect_wikidot_math_source<'r, 't>(
    parser: &mut Parser<'r, 't>,
    closes: &[ParseCondition],
    invalids: &[ParseCondition],
) -> Result<&'t str, ParseError>
where
    'r: 't,
{
    let start = parser.current().span.start;
    let mut saw_complete_authored_raw = false;

    loop {
        if parser.evaluate_any(closes) {
            let end = parser.current().span.start;
            parser.step()?;
            return if saw_complete_authored_raw {
                Ok("")
            } else {
                Ok(parser.full_text().inner()[start..end].trim())
            };
        }
        if parser.evaluate_any(invalids) {
            return Err(parser.make_err(ParseErrorKind::RuleFailed));
        }
        if parser.current().token == Token::InputEnd {
            return Err(parser.make_err(ParseErrorKind::EndOfInput));
        }

        if parser.current().token == Token::Raw && parser.current_generated().is_none() {
            let mut raw = parser.clone();
            if RULE_RAW.try_consume(&mut raw).is_ok() {
                let raw_end = raw.current().span.start;
                let mut owner = parser.clone();
                let mut has_non_authored = false;

                while owner.current().span.start < raw_end {
                    if owner.current().token == Token::RightMath {
                        return Err(parser.make_err(ParseErrorKind::RuleFailed));
                    }
                    has_non_authored |= owner.current_generated().is_some()
                        || matches!(
                            owner.current().token,
                            Token::RuntimeText
                                | Token::GeneratedPageLink
                                | Token::GeneratedTagLinks
                        );
                    owner.step()?;
                }

                // A delayed value inside raw must survive outer rollback as
                // delayed raw data. It cannot be erased into an empty math
                // node or flattened into an authored math field.
                if has_non_authored {
                    return Err(parser.make_err(ParseErrorKind::RuleFailed));
                }

                saw_complete_authored_raw = true;
                parser.update(&raw);
                continue;
            }
        }
        parser.step()?;
    }
}
