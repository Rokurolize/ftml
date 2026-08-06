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
    impls::{RULE_FALLBACK, starts_own_line_rule, url_elements},
};
use crate::tree::{LinkLabel, LinkLocation, LinkType, PartialElement};
use std::mem;

fn try_consume_inline_format_close<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<Option<Elements<'t>>, ParseError>
where
    'r: 't,
{
    if parser.settings().layout.legacy()
        && parser.pending_wikidot_collapsible_closer()
        && parser.current().token == Token::LeftBlockEnd
    {
        let unquoted_close = parser.native_blockquote_depth().is_none();
        let mut close = parser.clone();
        if close
            .get_end_block()
            .is_ok_and(|name| name.eq_ignore_ascii_case("collapsible"))
        {
            parser.update(&close);
            parser.set_pending_wikidot_collapsible_closer(false);
            return Ok(Some(if unquoted_close {
                Element::LineBreak.into()
            } else {
                Elements::None
            }));
        }
    }

    if !parser.settings().layout.legacy() || parser.current().token != Token::LeftBlockEnd
    {
        return Ok(None);
    }

    let mut close = parser.clone();
    let Ok(name) = close.get_end_block() else {
        return Ok(None);
    };
    let normalized = name.strip_suffix('_').unwrap_or(name);
    let start = parser.current().span.start;
    let end = close.current().span.start;
    let close_source = cow!(&parser.full_text().inner()[start..end]);
    let partial = if normalized.eq_ignore_ascii_case("size") {
        PartialElement::InlineSizeClose(close_source)
    } else if normalized.eq_ignore_ascii_case("span") {
        PartialElement::InlineSpanClose(close_source)
    } else {
        return Ok(None);
    };

    parser.update(&close);
    Ok(Some(Element::Partial(partial).into()))
}

fn try_consume_wikidot_adjacent_unmatched_closes_as_link<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<Option<Elements<'t>>, ParseError>
where
    'r: 't,
{
    if !parser.settings().layout.legacy() || parser.current().token != Token::LeftBlockEnd
    {
        return Ok(None);
    }

    let source = parser.full_text().inner();
    let first_start = parser.current().span.start;
    let mut scan = parser.clone();
    if scan.get_end_block().is_err() || scan.current().token != Token::Whitespace {
        return Ok(None);
    }
    let first_end = scan.current().span.start;
    scan.step()?;
    while !matches!(
        scan.current().token,
        Token::LeftBlock
            | Token::LeftBlockEnd
            | Token::LineBreak
            | Token::ParagraphBreak
            | Token::InputEnd
    ) {
        scan.step()?;
    }
    if !matches!(scan.current().token, Token::LeftBlock | Token::LeftBlockEnd) {
        return Ok(None);
    }
    let second_start = scan.current().span.start;
    let second_is_close = scan.current().token == Token::LeftBlockEnd;
    let second_valid = if second_is_close {
        scan.get_end_block().is_ok()
    } else {
        scan.get_block_name(false).is_ok()
    };
    if !second_valid
        || second_is_close
            && !matches!(
                scan.current().token,
                Token::LineBreak | Token::ParagraphBreak | Token::InputEnd
            )
    {
        return Ok(None);
    }
    let second_end = scan.current().span.start;
    let first = &source[first_start..first_end];
    let second = &source[second_start..second_end];
    if !first.starts_with("[[/")
        || !first.ends_with("]]")
        || !(second.starts_with("[[/") || second.starts_with("[["))
        || !second.ends_with("]]")
    {
        return Ok(None);
    }

    let url = &first[2..];
    let label = source[first_end..second_end - 2].trim();
    parser.update(&scan);
    Ok(Some(Elements::Multiple(vec![
        text!("["),
        Element::Link {
            ltype: LinkType::Direct,
            link: LinkLocation::Url(cow!(url)),
            label: LinkLabel::Text(cow!(label)),
            target: None,
        },
        text!("]"),
    ])))
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
        | Token::RuntimeText
        | Token::Other => true,

        Token::Whitespace => {
            !parser.start_of_line()
                && !matches!(
                    parser.next_two_tokens(),
                    (Token::Whitespace, Some(Token::Underscore))
                )
        }

        Token::Underscore => {
            parser.settings().layout.legacy()
                || !(parser.start_of_line()
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
            )
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

        // Wikidot treats a nested monospace opener as literal text and closes
        // the outer span at the first following terminator.
        Token::LeftMonospace => parser.rule().name() == "monospace",

        // A closing raw marker is intercepted by the raw collector when it
        // has a matching opener. Outside a raw span Wikidot preserves it.
        Token::RightRaw => true,

        // A standalone closing bracket pair is literal Wikidot text. Valid
        // block collectors intercept their own closer before `consume()`.
        Token::RightBlock => parser.start_of_line(),

        Token::BulletItem | Token::NumberedItem | Token::Equals | Token::Colon => {
            !parser.start_of_line()
        }

        // Four-or-more hyphen runs can be horizontal rules only at an
        // immediate line boundary, but Wikidot still gives their inline
        // fallback structured dash/strikethrough semantics.
        Token::TripleDash => false,

        _ => false,
    }
}

fn try_consume_text_token<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<Option<Elements<'t>>, ParseError> {
    if parser.settings().layout.legacy()
        && parser.current().slice == "\u{fffd}"
        && parser
            .look_ahead(0)
            .is_some_and(|token| token.token == Token::LineBreak && token.slice == "\\")
    {
        parser.step()?;
        return Ok(Some(Elements::None));
    }
    if parser.current().token == Token::DiscardedControl {
        parser.step()?;
        return Ok(Some(Elements::None));
    }
    // Wikidot discards adjacent underline delimiters as empty containers. Do
    // that one pair at a time so a long run never needs a forward scan at
    // every delimiter.
    if parser.settings().layout.legacy() && parser.current().token == Token::Underline {
        #[cfg(test)]
        parser.increment_underline_fast_path_visits();

        if parser
            .look_ahead(0)
            .is_some_and(|token| token.token == Token::Underline)
        {
            parser.step()?;
            parser.step()?;
            return Ok(Some(Elements::None));
        }
    }
    if !can_consume_as_text_token(parser) {
        return Ok(None);
    }

    let token = parser.current().token;
    let slice = parser.current().slice;
    let trailing_whitespace = token == Token::Whitespace
        && matches!(
            parser.look_ahead(0).map(|next| next.token),
            Some(Token::LineBreak | Token::ParagraphBreak | Token::InputEnd)
        );
    parser.step()?;
    if trailing_whitespace {
        Ok(Some(Elements::None))
    } else if token == Token::Whitespace {
        Ok(Some(text!(" ").into()))
    } else {
        Ok(Some(text!(slice).into()))
    }
}

fn try_consume_line_break<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<Option<Elements<'t>>, ParseError> {
    if parser.current().token != Token::LineBreak {
        return Ok(None);
    }
    if parser.settings().layout.legacy()
        && parser.current().slice != "\\"
        && parser
            .look_ahead(0)
            .is_some_and(|token| token.token == Token::InputEnd)
    {
        parser.step()?;
        return Ok(Some(Elements::None));
    }

    // A conditional quote cursor can interpret a marker remaining after its
    // required physical prefix as literal content. The raw lookahead fast path
    // sees only the outer Quote token, so defer that one shape to the generic
    // line-break rule after preparing the prefix.
    if parser.quote_body_cursor().is_some() {
        let mut next = parser.clone();
        next.step()?;
        next.get_optional_space()?;
        if next.prepare_quote_body_line()? == QuoteBodyLineStatus::Prepared {
            return Ok(None);
        }
    }

    let next = parser.next_three_tokens();
    let starts_definition =
        next.1 == Some(Token::Colon) && next.2 == Some(Token::Whitespace);
    let starts_block = matches!(next.1, Some(Token::LeftBlock | Token::LeftBlockStar));
    if starts_definition
        || (starts_block && !upcoming_block_ends_with_single_bracket(parser))
    {
        return Ok(None);
    }

    let next_offset = if matches!(
        parser.look_ahead(0).map(|token| token.token),
        Some(Token::Whitespace)
    ) {
        1
    } else {
        0
    };
    let skip = parser.look_ahead(next_offset).is_some_and(|token| {
        let following = parser.look_ahead(next_offset + 1).map(|next| next.token);
        let valid_shape = match token.token {
            Token::Heading | Token::BulletItem | Token::NumberedItem => {
                following == Some(Token::Whitespace)
            }
            Token::Equals => {
                following == Some(Token::Whitespace)
                    || parser
                        .remaining()
                        .iter()
                        .skip(next_offset + 1)
                        .take_while(|next| next.token == Token::Equals)
                        .count()
                        >= 3
            }
            Token::TripleDash => matches!(
                following,
                Some(Token::LineBreak | Token::ParagraphBreak | Token::InputEnd)
            ),
            _ => true,
        };
        starts_own_line_rule(token.token)
            && valid_shape
            && !(next_offset == 1
                && matches!(
                    token.token,
                    Token::Quote | Token::BulletItem | Token::NumberedItem
                ))
    });

    parser.step()?;
    if skip {
        Ok(Some(Elements::None))
    } else {
        Ok(Some(Element::LineBreak.into()))
    }
}

fn upcoming_block_ends_with_single_bracket(parser: &Parser<'_, '_>) -> bool {
    let mut last = None;
    for token in parser.remaining() {
        if matches!(
            token.token,
            Token::LineBreak | Token::ParagraphBreak | Token::InputEnd
        ) {
            break;
        }
        if token.token == Token::RightBlock {
            return false;
        }
        if token.token != Token::Whitespace {
            last = Some(token.token);
        }
    }
    last == Some(Token::RightBracket)
}

fn try_consume_leaf_token<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<Option<Elements<'t>>, ParseError> {
    if parser.current().token == Token::Url {
        let elements = url_elements(parser)?;
        return Ok(Some(elements));
    }

    let element = match parser.current().token {
        Token::Email => Element::Email(cow!(parser.current().slice)),

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

    let pending_unquoted_collapsible_close = parser.settings().layout.legacy()
        && parser.pending_wikidot_collapsible_closer()
        && parser.native_blockquote_depth().is_none();
    if let Some(elements) = try_consume_inline_format_close(parser)? {
        parser.depth_decrement();
        if pending_unquoted_collapsible_close {
            return ok!(false; elements);
        }
        return ok!(elements);
    }

    if let Some(elements) = try_consume_wikidot_adjacent_unmatched_closes_as_link(parser)?
    {
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
        if rule.name() == "delayed-conditional"
            && parser.generated_until_right_block().is_empty()
        {
            continue;
        }
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

    let element = if parser.settings().layout.legacy() {
        match current.token {
            Token::LeftComment => text!("[!\u{2014}"),
            Token::RightComment => text!("\u{2014}]"),
            _ => text!(current.slice),
        }
    } else {
        text!(current.slice)
    };
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

        let (tokens, page_info, settings) = parser_for("text~~~~!!!");
        let mut parser = parser_at(&tokens, &page_info, &settings, 2);
        let elements = try_consume_text_token(&mut parser)
            .expect("mid-line clear-float marker should not fail")
            .expect("mid-line clear-float marker should use the text fast path");
        assert_eq!(elements, text!("~~~~").into());

        let (tokens, page_info, mut settings) = parser_for("______ ______");
        settings.layout = Layout::Wikidot;
        let mut parser = parser_at(&tokens, &page_info, &settings, 1);
        let elements = try_consume_text_token(&mut parser)
            .expect("repeated underline spacer should not fail")
            .expect("repeated underline spacer should use the text fast path");
        assert_eq!(elements, Elements::None);

        let (tokens, page_info, mut settings) = parser_for("______[[/span]]");
        settings.layout = Layout::Wikidot;
        let mut parser = parser_at(&tokens, &page_info, &settings, 1);
        let elements = try_consume_text_token(&mut parser)
            .expect("repeated underline before a block closer should not fail")
            .expect("repeated underline before a block closer should stay literal");
        assert_eq!(elements, Elements::None);

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

        let (tokens, page_info, settings) = parser_for("alpha\n[[iftags +alphaX]\nomega");
        let mut parser = parser_at(&tokens, &page_info, &settings, 2);
        let elements = try_consume_line_break(&mut parser)
            .expect("line break before malformed block should not fail")
            .expect("single-bracket block fallback must keep its line break");
        assert_eq!(elements, Element::LineBreak.into());
        assert_eq!(parser.current().token, Token::LeftBlock);

        let (tokens, page_info, settings) = parser_for("alpha\n[[code]]]");
        let mut parser = parser_at(&tokens, &page_info, &settings, 2);
        assert!(
            try_consume_line_break(&mut parser)
                .expect("valid block followed by literal bracket should not fail")
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
    fn wikidot_underline_pair_consumption_has_linear_deterministic_work() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        for marker_count in [1, 2, 3, 4, 255, 256, 511, 512] {
            let input = format!("{}x", "__".repeat(marker_count));
            let tokenization = crate::tokenize(&input);
            let mut parser = Parser::new(&tokenization, &page_info, &settings);
            parser
                .step()
                .expect("the first underline token should exist");

            while parser.current().token != Token::InputEnd {
                let _ = consume(&mut parser).expect("the underline run should parse");
            }

            assert_eq!(
                parser.underline_fast_path_visits(),
                marker_count.div_ceil(2),
                "{marker_count} underline tokens",
            );
        }
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

    #[test]
    fn wikidot_url_leaf_fast_path_splits_terminal_period() {
        let (tokens, page_info, mut settings) =
            parser_for("https://example.com/test. next");
        settings.layout = Layout::Wikidot;
        let mut parser = parser_at(&tokens, &page_info, &settings, 1);
        let elements = try_consume_leaf_token(&mut parser)
            .expect("URL fast path should not fail")
            .expect("URL should use leaf fast path");
        assert_eq!(
            elements,
            Elements::Multiple(vec![
                Element::Link {
                    ltype: LinkType::Direct,
                    link: LinkLocation::Url(cow!("https://example.com/test")),
                    label: LinkLabel::Url,
                    target: None,
                },
                text!("."),
            ]),
        );
        assert_eq!(parser.current().token, Token::Whitespace);
    }

    #[test]
    fn wikidot_adjacent_unmatched_block_closes_reenter_single_link_lexing() {
        let (tokens, page_info, mut settings) = parser_for("[[/cell]] [[/table]]");
        settings.layout = Layout::Wikidot;
        let mut parser = parser_at(&tokens, &page_info, &settings, 1);
        let elements = try_consume_wikidot_adjacent_unmatched_closes_as_link(&mut parser)
            .expect("adjacent close fallback should not fail")
            .expect("adjacent unmatched closes should reenter link lexing");

        assert_eq!(
            elements,
            Elements::Multiple(vec![
                text!("["),
                Element::Link {
                    ltype: LinkType::Direct,
                    link: LinkLocation::Url(cow!("/cell]]")),
                    label: LinkLabel::Text(cow!("[[/table")),
                    target: None,
                },
                text!("]"),
            ]),
        );
        assert_eq!(parser.current().token, Token::InputEnd);

        let (tokens, page_info, mut settings) = parser_for("[[/cell]]");
        settings.layout = Layout::Wikidot;
        let mut parser = parser_at(&tokens, &page_info, &settings, 1);
        assert!(
            try_consume_wikidot_adjacent_unmatched_closes_as_link(&mut parser)
                .expect("single unmatched close should not fail")
                .is_none(),
        );

        let (tokens, page_info, mut settings) = parser_for("[[/cell]][[/table]]");
        settings.layout = Layout::Wikidot;
        let mut parser = parser_at(&tokens, &page_info, &settings, 1);
        assert!(
            try_consume_wikidot_adjacent_unmatched_closes_as_link(&mut parser)
                .expect("unspaced unmatched closes should not fail")
                .is_none(),
        );

        let (tokens, page_info, settings) = parser_for("[[/cell]] [[/table]]");
        let mut parser = parser_at(&tokens, &page_info, &settings, 1);
        assert!(
            try_consume_wikidot_adjacent_unmatched_closes_as_link(&mut parser)
                .expect("Wikijump layout fallback should not fail")
                .is_none(),
        );
    }
}
