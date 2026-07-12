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
use super::parser::QuoteBodyLineStatus;
use super::prelude::*;
use super::rule::{
    get_rules_for_token,
    impls::{RULE_FALLBACK, starts_own_line_rule},
};
use crate::tree::{LinkLabel, LinkLocation, LinkType, PartialElement};
use std::mem;

fn try_consume_inline_format_close<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<Option<Elements<'t>>, ParseError>
where
    'r: 't,
{
    if !parser.settings().layout.legacy() || parser.current().token != Token::LeftBlockEnd
    {
        return Ok(None);
    }

    let mut close = parser.clone();
    let Ok(name) = close.get_end_block() else {
        return Ok(None);
    };
    let normalized = name.strip_suffix('_').unwrap_or(name);
    let partial = if normalized.eq_ignore_ascii_case("size") {
        PartialElement::InlineSizeClose
    } else if normalized.eq_ignore_ascii_case("span") {
        let start = parser.current().span.start;
        let end = close.current().span.start;
        PartialElement::InlineSpanClose(cow!(&parser.full_text().inner()[start..end]))
    } else {
        return Ok(None);
    };

    parser.update(&close);
    Ok(Some(Element::Partial(partial).into()))
}

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

        // Wikidot leaves padded formatting openers as literal text. A
        // delimiter followed by whitespace cannot begin any of these inline
        // containers, so do not send it through a rule that is guaranteed to
        // fail and add a warning.
        Token::Underline => {
            matches!(
                parser.look_ahead(0).map(|token| token.token),
                Some(Token::Whitespace | Token::LeftBlockEnd)
            ) || repeated_underline_run_ends_at_literal_boundary(parser)
        }

        Token::Bold | Token::Italics | Token::Superscript | Token::Subscript => matches!(
            parser.look_ahead(0).map(|token| token.token),
            Some(Token::Whitespace)
        ),

        // These markers are structural only at the start of a line. Real
        // Wikidot pages also use repeated tildes as ordinary punctuation.
        Token::ClearFloatBoth | Token::ClearFloatLeft | Token::ClearFloatRight => {
            !parser.start_of_line()
        }

        // A closing raw marker is intercepted by the raw collector when it
        // has a matching opener. Outside a raw span Wikidot preserves it.
        Token::RightRaw => true,

        // A standalone closing bracket pair is literal Wikidot text. Valid
        // block collectors intercept their own closer before `consume()`.
        Token::RightBlock => parser.start_of_line(),

        Token::BulletItem
        | Token::NumberedItem
        | Token::Equals
        | Token::Colon
        | Token::TripleDash => !parser.start_of_line(),

        _ => false,
    }
}

fn repeated_underline_run_ends_at_literal_boundary(parser: &Parser<'_, '_>) -> bool {
    let start = parser.current().span.start;
    if let Some(literal) = parser.literal_underline_run(start) {
        return literal;
    }

    let mut repeated = false;
    let mut starts = vec![start];
    for token in parser.remaining() {
        if token.token == Token::Underline {
            repeated = true;
            starts.push(token.span.start);
            continue;
        }
        let literal =
            repeated && matches!(token.token, Token::Whitespace | Token::LeftBlockEnd);
        parser.cache_literal_underline_run(starts, literal);
        return literal;
    }
    parser.cache_literal_underline_run(starts, false);
    false
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

fn try_consume_line_break<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<Option<Elements<'t>>, ParseError> {
    if parser.current().token != Token::LineBreak {
        return Ok(None);
    }

    // A conditional quote cursor can interpret a marker remaining after its
    // required physical prefix as literal content. The raw lookahead fast path
    // sees only the outer Quote token, so defer that one shape to the generic
    // line-break rule after preparing the prefix.
    if parser.quote_body_has_literal_residuals() {
        let mut next = parser.clone();
        next.step()?;
        next.get_optional_space()?;
        if next.prepare_quote_body_line()? == QuoteBodyLineStatus::Prepared
            && next.current().token == Token::Quote
            && !next.start_of_line()
        {
            return Ok(None);
        }
    }

    match parser.next_three_tokens() {
        (Token::LineBreak, Some(Token::LeftBlock | Token::LeftBlockStar), _)
        | (Token::LineBreak, Some(Token::Colon), Some(Token::Whitespace)) => {
            return Ok(None);
        }
        _ => {}
    }

    let next_offset = if matches!(
        parser.look_ahead(0).map(|token| token.token),
        Some(Token::Whitespace)
    ) {
        1
    } else {
        0
    };
    let skip = parser
        .look_ahead(next_offset)
        .is_some_and(|token| starts_own_line_rule(token.token));

    parser.step()?;
    if skip {
        Ok(Some(Elements::None))
    } else {
        Ok(Some(Element::LineBreak.into()))
    }
}

fn try_consume_leaf_token<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<Option<Elements<'t>>, ParseError> {
    let element = match parser.current().token {
        Token::Email => Element::Email(cow!(parser.current().slice)),

        Token::Url => Element::Link {
            ltype: LinkType::Direct,
            link: LinkLocation::Url(cow!(parser.current().slice)),
            label: LinkLabel::Url,
            target: None,
        },

        Token::Variable => {
            let slice = parser.current().slice;
            let variable = &slice[2..slice.len() - 1];
            Element::Variable(cow!(variable))
        }

        _ => return Ok(None),
    };

    parser.step()?;
    Ok(Some(element.into()))
}

/// Main function that consumes tokens to produce a single element, then returns.
///
/// It will use the fallback if all rules, fail, so the only failure case is if
/// the end of the input is reached.
pub fn consume<'r, 't>(parser: &mut Parser<'r, 't>) -> ParseResult<'r, 't, Elements<'t>> {
    if parser.prepare_quote_body_line()? == QuoteBodyLineStatus::Boundary {
        return Err(parser.make_err(ParseErrorKind::EndOfInput));
    }

    // Incrementing recursion depth
    // Will fail if we're too many layers in
    parser.depth_increment()?;

    if let Some(elements) = try_consume_inline_format_close(parser)? {
        parser.depth_decrement();
        return ok!(elements);
    }

    if let Some(elements) = try_consume_line_break(parser)? {
        parser.depth_decrement();
        return ok!(elements);
    }

    if let Some(elements) = try_consume_text_token(parser)? {
        parser.depth_decrement();
        return ok!(elements);
    }

    if let Some(elements) = try_consume_leaf_token(parser)? {
        parser.depth_decrement();
        return ok!(elements);
    }

    let mut all_errors = Vec::new();
    let current = parser.current();

    for &rule in get_rules_for_token(current) {
        let old_remaining = parser.remaining();
        let footnote_count = parser.footnote_count();
        match rule.try_consume(parser) {
            Ok(output) => {
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
                // Rollback footnotes added during failed rule attempt
                parser.truncate_footnotes(footnote_count);

                if parser.discarding_hidden_body() {
                    if parser.at_hidden_body_boundary() {
                        parser.depth_decrement();
                        return Err(error);
                    }

                    if hidden_failure_must_close_to_eof(error.kind()) {
                        parser.skip_to_input_end()?;
                        parser.depth_decrement();
                        return Err(error);
                    }
                }

                all_errors.push(error);
            }
        }
    }

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

fn hidden_failure_must_close_to_eof(kind: ParseErrorKind) -> bool {
    matches!(
        kind,
        ParseErrorKind::RecursionDepthExceeded
            | ParseErrorKind::EndOfInput
            | ParseErrorKind::BlockDisallowsStar
            | ParseErrorKind::BlockDisallowsScore
            | ParseErrorKind::BlockMissingName
            | ParseErrorKind::BlockMissingCloseBrackets
            | ParseErrorKind::BlockMalformedArguments
            | ParseErrorKind::BlockMissingArguments
            | ParseErrorKind::ModuleMissingName
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};
    use crate::tree::{LinkLabel, LinkLocation, LinkType};

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

        let (tokens, page_info, settings) =
            parser_for("# // This is literal punctuation");
        let mut parser = parser_at(&tokens, &page_info, &settings, 3);
        let elements = try_consume_text_token(&mut parser)
            .expect("padded italics marker should not fail")
            .expect("padded italics marker should use the text fast path");
        assert_eq!(elements, text!("//").into());

        let (tokens, page_info, settings) = parser_for("text~~~!!!");
        let mut parser = parser_at(&tokens, &page_info, &settings, 2);
        let elements = try_consume_text_token(&mut parser)
            .expect("mid-line clear-float marker should not fail")
            .expect("mid-line clear-float marker should use the text fast path");
        assert_eq!(elements, text!("~~~").into());

        let (tokens, page_info, settings) = parser_for("______ ______");
        let mut parser = parser_at(&tokens, &page_info, &settings, 1);
        let elements = try_consume_text_token(&mut parser)
            .expect("repeated underline spacer should not fail")
            .expect("repeated underline spacer should use the text fast path");
        assert_eq!(elements, text!("__").into());

        let (tokens, page_info, settings) = parser_for("______[[/span]]");
        let mut parser = parser_at(&tokens, &page_info, &settings, 1);
        let elements = try_consume_text_token(&mut parser)
            .expect("repeated underline before a block closer should not fail")
            .expect("repeated underline before a block closer should stay literal");
        assert_eq!(elements, text!("__").into());

        let (tokens, page_info, settings) = parser_for("x>@");
        let mut parser = parser_at(&tokens, &page_info, &settings, 2);
        let elements = try_consume_text_token(&mut parser)
            .expect("unmatched raw closer should not fail")
            .expect("unmatched raw closer should use the text fast path");
        assert_eq!(elements, text!(">@").into());

        let (tokens, page_info, settings) = parser_for("alpha\n]]");
        let mut parser = parser_at(&tokens, &page_info, &settings, 3);
        let elements = try_consume_text_token(&mut parser)
            .expect("standalone closing brackets should not fail")
            .expect("standalone closing brackets should use the text fast path");
        assert_eq!(elements, text!("]]").into());
    }

    #[test]
    fn direct_line_break_fast_path_preserves_skips_and_block_fallbacks() {
        let (tokens, page_info, settings) = parser_for("alpha\nbeta");
        let mut parser = parser_at(&tokens, &page_info, &settings, 2);
        let elements = try_consume_line_break(&mut parser)
            .expect("line break fast path should not fail")
            .expect("plain line break should use fast path");
        assert_eq!(elements, Element::LineBreak.into());
        assert_eq!(parser.current().token, Token::Identifier);

        let (tokens, page_info, settings) = parser_for("alpha\n+ heading");
        let mut parser = parser_at(&tokens, &page_info, &settings, 2);
        let elements = try_consume_line_break(&mut parser)
            .expect("line break before heading should not fail")
            .expect("line break before heading should use fast path");
        assert_eq!(elements, Elements::None);
        assert_eq!(parser.current().token, Token::Heading);

        let (tokens, page_info, settings) = parser_for("alpha\n> > quoted");
        let mut parser = parser_at(&tokens, &page_info, &settings, 2);
        parser.install_quote_body_cursor_with_literal_residuals(1);
        assert!(
            try_consume_line_break(&mut parser)
                .expect("quote-aware line break deferral should not fail")
                .is_none(),
        );
        assert_eq!(parser.current().token, Token::LineBreak);

        let (tokens, page_info, settings) = parser_for("alpha\n[[code]]");
        let mut parser = parser_at(&tokens, &page_info, &settings, 2);
        assert!(
            try_consume_line_break(&mut parser)
                .expect("line break block fallback check should not fail")
                .is_none(),
        );
        assert_eq!(parser.current().token, Token::LineBreak);

        let (tokens, page_info, settings) = parser_for("alpha\n: term");
        let mut parser = parser_at(&tokens, &page_info, &settings, 2);
        assert!(
            try_consume_line_break(&mut parser)
                .expect("definition-list fallback check should not fail")
                .is_none(),
        );
        assert_eq!(parser.current().token, Token::LineBreak);
    }

    #[test]
    fn direct_leaf_fast_path_preserves_leaf_elements() {
        let (tokens, page_info, settings) = parser_for("https://example.com");
        let mut parser = parser_at(&tokens, &page_info, &settings, 1);
        let elements = try_consume_leaf_token(&mut parser)
            .expect("url fast path should not fail")
            .expect("url should use leaf fast path");
        assert_eq!(
            elements,
            Element::Link {
                ltype: LinkType::Direct,
                link: LinkLocation::Url(cow!("https://example.com")),
                label: LinkLabel::Url,
                target: None,
            }
            .into(),
        );
        assert_eq!(parser.current().token, Token::InputEnd);

        let (tokens, page_info, settings) = parser_for("abc@example.com");
        let mut parser = parser_at(&tokens, &page_info, &settings, 1);
        let elements = try_consume_leaf_token(&mut parser)
            .expect("email fast path should not fail")
            .expect("email should use leaf fast path");
        assert_eq!(elements, Element::Email(cow!("abc@example.com")).into());
        assert_eq!(parser.current().token, Token::InputEnd);

        let (tokens, page_info, settings) = parser_for("{$title}");
        let mut parser = parser_at(&tokens, &page_info, &settings, 1);
        let elements = try_consume_leaf_token(&mut parser)
            .expect("variable fast path should not fail")
            .expect("variable should use leaf fast path");
        assert_eq!(elements, Element::Variable(cow!("title")).into());
        assert_eq!(parser.current().token, Token::InputEnd);
    }
}
