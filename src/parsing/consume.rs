/*
 * parsing/consume.rs
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

//! Module for look-ahead checking.
//!
//! This contains implementations of eager functions that try to interpret the
//! upcoming tokens as a particular object (e.g. seeing a `[[` and you see if it's a module).
//!
//! The parser is not disambiguous because any string of tokens can be interpreted
//! as raw text as a fallback, which is how Wikidot does it.

use super::Parser;
use super::prelude::*;
use super::rule::{get_rules_for_token, impls::RULE_FALLBACK};
use std::mem;

fn can_consume_as_text_token<'r, 't>(parser: &Parser<'r, 't>) -> bool {
    // Only bypass generic rule dispatch where the current token cannot start
    // a structural rule in this position. This keeps the public AST shape
    // unchanged while avoiding parser forks for ordinary text tokens.
    match parser.current().token {
        Token::Identifier
        | Token::RightBracket
        | Token::RightParentheses
        | Token::Pipe
        | Token::DoubleQuote
        | Token::EscapedDoubleQuote
        | Token::EscapedBackslash
        | Token::Other => true,

        Token::Whitespace => {
            !parser.start_of_line()
                && !matches!(
                    parser.next_two_tokens(),
                    (Token::Whitespace, Some(Token::Underscore))
                )
        }

        Token::Underscore => {
            !(parser.start_of_line()
                && matches!(
                    parser.look_ahead(0).map(|token| token.token),
                    Some(Token::LineBreak | Token::ParagraphBreak)
                ))
        }

        Token::BulletItem | Token::NumberedItem | Token::Equals | Token::Colon => {
            !parser.start_of_line()
        }

        _ => false,
    }
}

fn try_consume_text_token<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<Option<Elements<'t>>, ParseError> {
    if !can_consume_as_text_token(parser) {
        return Ok(None);
    }

    let slice = parser.current().slice;
    parser.step()?;
    Ok(Some(text!(slice).into()))
}

/// Main function that consumes tokens to produce a single element, then returns.
///
/// It will use the fallback if all rules, fail, so the only failure case is if
/// the end of the input is reached.
pub fn consume<'r, 't>(parser: &mut Parser<'r, 't>) -> ParseResult<'r, 't, Elements<'t>> {
    let token_name = parser.current().token.name();
    let token_slice = parser.current().slice;
    debug!("Running consume attempt (token {token_name}, slice {token_slice:?})");

    // Incrementing recursion depth
    // Will fail if we're too many layers in
    parser.depth_increment()?;

    if let Some(elements) = try_consume_text_token(parser)? {
        parser.depth_decrement();
        return ok!(elements);
    }

    trace!("Looking for valid rules");
    let mut all_errors = Vec::new();
    let current = parser.current();

    for &rule in get_rules_for_token(current) {
        trace!("Trying rule consumption for tokens (rule {})", rule.name());

        let old_remaining = parser.remaining();
        let footnote_count = parser.footnote_count();
        match rule.try_consume(parser) {
            Ok(output) => {
                debug!("Rule {} matched, returning generated result", rule.name());

                // If the pointer hasn't moved, we step one token.
                if parser.same_pointer(old_remaining) {
                    parser.step()?;
                }

                // Explicitly drop errors
                //
                // We're returning the successful consumption
                // so these are going to be dropped as a previously
                // unsuccessful attempts.
                mem::drop(all_errors);

                // Decrement recursion depth
                parser.depth_decrement();

                return Ok(output);
            }
            Err(error) => {
                warn!("Rule failed, returning error: '{}'", error.kind().name());
                // Rollback footnotes added during failed rule attempt
                parser.truncate_footnotes(footnote_count);
                all_errors.push(error);
            }
        }
    }

    warn!("All rules exhausted, using generic text fallback");
    let element = text!(current.slice);
    parser.step()?;

    // If we've hit the recursion limit, just bail
    if let Some(error) = all_errors.last()
        && error.kind() == ParseErrorKind::RecursionDepthExceeded
    {
        error!("Found recursion depth error, failing");
        return Err(error.clone());
    }

    // Add fallback error to errors list
    let error = ParseError::new(ParseErrorKind::NoRulesMatch, RULE_FALLBACK, current);
    all_errors.push(error);

    // Decrement recursion depth
    parser.depth_decrement();

    ok!(element, all_errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    fn parser_for<'t>(
        input: &'t str,
    ) -> (
        crate::tokenizer::Tokenization<'t>,
        PageInfo<'static>,
        WikitextSettings,
    ) {
        (
            crate::tokenize(input),
            PageInfo::dummy(),
            WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump),
        )
    }

    fn parser_at<'r, 't>(
        tokenization: &'r crate::tokenizer::Tokenization<'t>,
        page_info: &'r PageInfo<'static>,
        settings: &'r WikitextSettings,
        steps: usize,
    ) -> Parser<'r, 't> {
        let mut parser = Parser::new(tokenization, page_info, settings);
        for _ in 0..steps {
            parser.step().expect("test token step should succeed");
        }
        parser
    }

    #[test]
    fn direct_text_fast_path_preserves_structural_starts() {
        let (tokens, page_info, settings) = parser_for("word text");
        let parser = parser_at(&tokens, &page_info, &settings, 1);
        assert!(can_consume_as_text_token(&parser));

        let (tokens, page_info, settings) = parser_for("word text");
        let parser = parser_at(&tokens, &page_info, &settings, 2);
        assert!(can_consume_as_text_token(&parser));

        let (tokens, page_info, settings) = parser_for(" * item");
        let parser = parser_at(&tokens, &page_info, &settings, 1);
        assert!(!can_consume_as_text_token(&parser));

        let (tokens, page_info, settings) = parser_for("word _\nnext");
        let parser = parser_at(&tokens, &page_info, &settings, 2);
        assert!(!can_consume_as_text_token(&parser));

        let (tokens, page_info, settings) = parser_for(": term\n: value");
        let parser = parser_at(&tokens, &page_info, &settings, 1);
        assert!(!can_consume_as_text_token(&parser));
    }
}
