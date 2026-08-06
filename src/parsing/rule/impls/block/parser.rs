/*
 * parsing/rule/impls/block/parser.rs
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

use super::BlockRule;
use super::arguments::Arguments;
use crate::parsing::collect::{collect_text, collect_text_keep};
use crate::parsing::condition::ParseCondition;
use crate::parsing::consume::consume;
use crate::parsing::parser::{QuoteBodyLineStatus, QuoteScanOutcome};
use crate::parsing::{
    ExtractedToken, ParseError, ParseErrorKind, ParseResult, Parser, Token,
    gather_paragraphs,
};
use crate::tree::{Element, Elements};
use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

static ARGUMENT_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z0-9_\-]+").unwrap());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockBodyStart {
    Inline,
    NextPhysicalLine,
}

fn token_ends_argument_key(token: Token) -> bool {
    matches!(
        token,
        Token::Whitespace | Token::LineBreak | Token::ParagraphBreak | Token::Equals,
    )
}

fn token_is_argument_spacing(token: Token) -> bool {
    [Token::Whitespace, Token::LineBreak, Token::ParagraphBreak].contains(&token)
}

fn token_is_body_boundary(token: Token) -> bool {
    matches!(
        token,
        Token::LineBreak | Token::ParagraphBreak | Token::InputEnd
    )
}

fn block_rule_accepts_name(block_rule: &BlockRule, name: &str) -> bool {
    block_rule
        .accepts_names
        .iter()
        .any(|accepted| name.eq_ignore_ascii_case(accepted))
}

fn wikidot_requires_next_physical_line(block_rule: &BlockRule) -> bool {
    matches!(
        block_rule.name,
        "block-div"
            | "block-note"
            | "block-align-left"
            | "block-align-right"
            | "block-align-center"
            | "block-align-justify"
    )
}

fn wikidot_trim_argument_fragment(value: &str) -> &str {
    value.trim_matches([' ', '\t', '\n', '\r', '\0', '\u{000B}'])
}

fn wikidot_stripslashes(value: &str) -> Cow<'_, str> {
    if !value.contains('\\') {
        return Cow::Borrowed(value);
    }

    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }

        match chars.next() {
            Some('0') => output.push('\0'),
            Some(escaped) => output.push(escaped),
            None => {}
        }
    }

    Cow::Owned(output)
}

fn parse_wikidot_attributes(value: &str) -> Arguments<'_> {
    // Wikidot's getAttrs grammar splits only on the exact ASCII delimiter
    // `="`, then treats the last quote before the next delimiter as the
    // current value terminator. This intentionally preserves its unusual
    // malformed-fragment recovery and differs from FTML's strict grammar.
    let value = wikidot_trim_argument_fragment(value);
    let mut segments = value.split("=\"");
    let mut key = wikidot_trim_argument_fragment(segments.next().unwrap_or_default());
    let mut arguments = Arguments::new_case_sensitive();
    if !value.is_empty() {
        arguments.mark_source_present();
    }

    for segment in segments {
        let (raw_value, next_key) = match segment.rfind('"') {
            Some(position) => (&segment[..position], &segment[position + 1..]),
            None => ("", segment.get(1..).unwrap_or_default()),
        };
        arguments.insert(key, wikidot_stripslashes(raw_value));
        key = wikidot_trim_argument_fragment(next_key);
    }

    arguments
}

impl<'r, 't> Parser<'r, 't>
where
    'r: 't,
{
    pub fn get_block_name(
        &mut self,
        flag_star: bool,
    ) -> Result<(&'t str, bool), ParseError> {
        if flag_star {
            self.get_optional_token(Token::LeftBlockStar)?;
        } else {
            self.get_optional_token(Token::LeftBlock)?;
        }

        self.get_optional_space()?;

        // Collect block name and determine whether the head is done
        self.get_block_name_internal(ParseErrorKind::BlockMissingName)
    }

    fn get_block_name_internal(
        &mut self,
        kind: ParseErrorKind,
    ) -> Result<(&'t str, bool), ParseError> {
        let end_conditions = [
            ParseCondition::current(Token::Whitespace),
            ParseCondition::current(Token::LineBreak),
            ParseCondition::current(Token::ParagraphBreak),
            ParseCondition::current(Token::RightBlock),
        ];
        let rule = self.rule();
        let stops = &end_conditions;
        collect_text_keep(self, rule, stops, &[], Some(kind)).map(|(name, last)| {
            let name = name.trim();
            let in_head = !matches!(last.token, Token::RightBlock);

            (name, in_head)
        })
    }

    /// Matches an ending block, returning the name present.
    pub fn get_end_block(&mut self) -> Result<&'t str, ParseError> {
        self.get_token(Token::LeftBlockEnd, ParseErrorKind::BlockExpectedEnd)?;
        self.get_optional_space()?;

        let (name, in_head) = self.get_block_name(false)?;
        if in_head {
            self.get_optional_space()?;
            self.get_token(Token::RightBlock, ParseErrorKind::BlockExpectedEnd)?;
        }

        Ok(name)
    }

    /// Consumes an entire block end, validating that the newline and names match.
    ///
    /// Used internally by the body parsing methods.
    fn verify_end_block(
        &mut self,
        first_iteration: bool,
        block_rule: &BlockRule,
        restrict_quote_close: bool,
        allow_inline_quote_close: bool,
    ) -> Option<&'r ExtractedToken<'t>> {
        self.save_evaluate_fn(|parser| {
            if restrict_quote_close
                && allow_inline_quote_close
                && parser.settings().layout.legacy()
                && !parser.discarding_hidden_body()
                && block_rule.name == "block-collapsible"
                && parser.wikidot_collapsible_closed_at_deeper_quote()
            {
                parser.set_wikidot_collapsible_closed_at_deeper_quote(false);
                return Ok(true);
            }

            // Check that the end block is on a new line, if required
            if block_rule.accepts_newlines {
                if !first_iteration
                    && parser.settings().layout.legacy()
                    && !parser.discarding_hidden_body()
                    && block_rule.name == "block-math"
                {
                    parser
                        .get_token(Token::LineBreak, ParseErrorKind::BlockExpectedEnd)?;
                } else if !first_iteration {
                    // Only check after the first, to permit empty blocks
                    parser.get_optional_line_break()?;
                }
            }

            if restrict_quote_close
                && parser.prepare_quote_body_line()? == QuoteBodyLineStatus::Boundary
            {
                return Ok(false);
            }
            if restrict_quote_close && !parser.quote_body_close_allowed_here() {
                let follows_only_literal_quotes =
                    parser.quote_body_close_follows_only_literal_quotes();
                if !follows_only_literal_quotes
                    && (!allow_inline_quote_close
                        || !matches!(
                            parser.current().token,
                            Token::Quote | Token::LeftBlockEnd
                        ))
                {
                    return Ok(false);
                }
                while parser.current().token == Token::Quote {
                    parser.step()?;
                    parser.get_optional_space()?;
                }
            }

            // Check if it's an end block
            //
            // This will ignore any errors produced,
            // since it's just more text
            let end_start = parser.current().span.start;
            let name = parser.get_end_block()?;

            if parser.settings().layout.legacy()
                && !parser.discarding_hidden_body()
                && name.ends_with('_')
            {
                return Ok(false);
            }

            if parser.settings().layout.legacy()
                && !parser.discarding_hidden_body()
                && block_rule.name.starts_with("block-list-")
            {
                let end = parser.current().span.start;
                let source = &parser.full_text().inner()[end_start..end];
                let expected = format!("[[/{name}]]");
                if !source.eq_ignore_ascii_case(&expected) {
                    return Ok(false);
                }
            }

            let score_close = name.ends_with('_');
            let name = name.strip_suffix('_').unwrap_or(name);
            if parser.settings().layout.legacy()
                && score_close
                && block_rule.name == "block-iftags"
            {
                return Ok(false);
            }

            // Check if it's valid
            for end_block_name in block_rule.accepts_names {
                if name.eq_ignore_ascii_case(end_block_name) {
                    if restrict_quote_close {
                        parser.get_optional_space()?;
                        if !allow_inline_quote_close
                            && !token_is_body_boundary(parser.current().token)
                        {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                }
            }

            Ok(false)
        })
    }

    // Body parsing

    /// Generic helper function that performs the primary block collection.
    ///
    /// Extended by the other, more specific functions.
    fn get_body_generic<F>(
        &mut self,
        block_rule: &BlockRule,
        mut process: F,
    ) -> Result<(&'r ExtractedToken<'t>, &'r ExtractedToken<'t>), ParseError>
    where
        F: FnMut(&mut Parser<'r, 't>) -> Result<(), ParseError>,
    {
        let has_end_names = !block_rule.accepts_names.is_empty();
        debug_assert!(has_end_names, "block body has no valid end names");

        // Keep iterating until we find the end.
        // Preserve parse progress if we've hit the end block.
        let mut first = true;
        let wikidot_math = self.settings().layout.legacy()
            && !self.discarding_hidden_body()
            && block_rule.name == "block-math";
        let mut nested_wikidot_math = 0;
        let start = self.current();

        loop {
            let before_end = self.clone();
            let at_end_block = self.verify_end_block(first, block_rule, false, false);

            // If there's a match, return the last body token
            if let Some(end) = at_end_block {
                if nested_wikidot_math > 0 {
                    nested_wikidot_math -= 1;
                    self.update(&before_end);
                } else {
                    return Ok((start, end));
                }
            } else if wikidot_math && self.at_wikidot_nested_math_opener() {
                nested_wikidot_math += 1;
            }

            if self.settings().layout.legacy()
                && self.discarding_hidden_body()
                && self.at_hidden_body_boundary()
                && (!self.has_body_end_block(block_rule)
                    || matches!(block_rule.name, "block-math" | "block-raw"))
            {
                return Err(self.make_err(ParseErrorKind::BlockExpectedEnd));
            }

            // Run the passed-in closure
            process(self)?;

            // Step and continue
            self.step()?;
            first = false;
        }
    }

    fn at_wikidot_nested_math_opener(&self) -> bool {
        if !matches!(
            self.current().token,
            Token::LineBreak | Token::ParagraphBreak
        ) {
            return false;
        }
        let start = self.current().span.end;
        let source = &self.full_text().inner()[start..];
        let Some(end) = source.find("]]") else {
            return false;
        };
        let mut parts = source[..end].trim_start_matches("[[").split_whitespace();
        if !parts
            .next()
            .is_some_and(|name| name.eq_ignore_ascii_case("math"))
        {
            return false;
        }
        let name = parts.next();
        parts.next().is_none() && name.is_none_or(super::blocks::wikidot_math_name)
    }

    /// Collect a block's body to its end, as string slice.
    ///
    /// This requires that the has already been parsed using
    /// one of the "get argument" methods.
    ///
    /// The `accepts_newlines` argument designates whether this
    /// block assumes multiline construction (e.g. `[[div]]`, `[[code]]`)
    /// or not (e.g. `[[span]]`).
    pub fn get_body_text(
        &mut self,
        block_rule: &BlockRule,
    ) -> Result<Cow<'t, str>, ParseError> {
        if let Some(required_depth) = self.native_blockquote_depth()
            && (self.quote_body_needs_prefix()
                || self.current().token == Token::LineBreak)
        {
            return self.get_native_blockquote_body_text(block_rule, required_depth);
        }

        // State variables for collecting span
        let (start, end) = self.get_body_generic(block_rule, |_| Ok(()))?;
        let slice = self.full_text().slice_partial(start, end);
        Ok(Cow::Borrowed(slice))
    }

    pub(crate) fn get_body_text_after_skipping_end_blocks(
        &mut self,
        block_rule: &BlockRule,
        mut end_blocks_to_skip: usize,
    ) -> Result<Cow<'t, str>, ParseError> {
        let start = self.current();
        let mut first = true;

        loop {
            if let Some(end) = self.verify_end_block(first, block_rule, false, false) {
                if end_blocks_to_skip == 0 {
                    let slice = self.full_text().slice_partial(start, end);
                    return Ok(Cow::Borrowed(slice));
                }
                end_blocks_to_skip -= 1;
                first = false;
                continue;
            }

            self.step()?;
            first = false;
        }
    }

    pub(crate) fn body_has_generated(&self, block_rule: &BlockRule) -> bool {
        let mut probe = self.clone();
        let Ok((start, end)) = probe.get_body_generic(block_rule, |_| Ok(())) else {
            return false;
        };
        probe.has_generated_in_range(start.span.start..end.span.end)
    }

    fn scan_absolute_quote_prefix(
        &self,
        required_depth: usize,
    ) -> Option<(usize, usize, Self)> {
        let mut parser = self.clone();
        let mut absolute_depth = 0;
        let mut content_start = None;

        while parser.current().token == Token::Quote {
            let quote = parser.current();
            let depth_before = absolute_depth;
            absolute_depth += quote.slice.len();
            parser.step().ok()?;
            parser.get_optional_space().ok()?;

            if content_start.is_none() && absolute_depth >= required_depth {
                content_start = Some(if absolute_depth == required_depth {
                    parser.current().span.start
                } else {
                    quote.span.start + (required_depth - depth_before)
                });
            }
        }

        Some((content_start?, absolute_depth, parser))
    }

    #[rustfmt::skip]
    fn get_native_blockquote_body_text(
        &mut self,
        block_rule: &BlockRule,
        required_depth: usize,
    ) -> Result<Cow<'t, str>, ParseError> {
        let mut close_scan = self.clone();
        if close_scan.current().token == Token::LineBreak {
            close_scan.step()?;
        }
        let has_close = close_scan.has_native_blockquote_body_end_with_mode(block_rule, required_depth, true);
        if !has_close {
            return Err(self.make_err(ParseErrorKind::RuleFailed));
        }

        let mut body = String::new();
        let mut trailing_line_break_len = 0;

        // Blocks such as [[raw]] retain their opening line break in the
        // ordinary contiguous-slice path. The normalized quoted result below
        // is returned without either outer line break, so consume it here.
        if self.current().token == Token::LineBreak {
            self.step()?;
        }

        loop {
            let mut outer_prepared = self.clone();
            if outer_prepared.prepare_quote_body_line()? != QuoteBodyLineStatus::Prepared
            {
                return Err(self.make_err(ParseErrorKind::EndOfInput));
            }

            let (content_start, absolute_depth, mut scan) = self.scan_absolute_quote_prefix(required_depth).ok_or_else(|| self.make_err(ParseErrorKind::EndOfInput))?;
            scan.set_quote_body_cursor(outer_prepared.quote_body_cursor());

            while !token_is_body_boundary(scan.current().token) {
                let marker_start = scan.current().span.start;
                let mut end = scan.clone();
                if absolute_depth == required_depth
                    && let Ok(name) = end.get_end_block()
                {
                    let name = name.strip_suffix('_').unwrap_or(name);
                    if block_rule_accepts_name(block_rule, name) {
                        if marker_start == content_start {
                            body.truncate(body.len() - trailing_line_break_len);
                        } else {
                            body.push_str(
                                &self.full_text().inner()[content_start..marker_start],
                            );
                        }
                        self.update(&end);
                        return Ok(Cow::Owned(body));
                    }
                }

                scan.step()?;
            }

            if scan.current().token != Token::LineBreak {
                return Err(self.make_err(ParseErrorKind::EndOfInput));
            }

            let content_end = scan.current().span.start;
            body.push_str(&self.full_text().inner()[content_start..content_end]);
            let line_break = scan.current().slice;
            body.push_str(line_break);
            trailing_line_break_len = line_break.len();
            self.update(&scan);
            self.step()?;
        }
    }

    #[inline]
    pub fn get_body_elements(
        &mut self,
        block_rule: &BlockRule,
        as_paragraphs: bool,
    ) -> ParseResult<'r, 't, Vec<Element<'t>>> {
        self.get_body_elements_internal(block_rule, as_paragraphs, false, false)
    }

    fn get_body_elements_internal(
        &mut self,
        block_rule: &BlockRule,
        as_paragraphs: bool,
        restrict_quote_close: bool,
        allow_inline_quote_close: bool,
    ) -> ParseResult<'r, 't, Vec<Element<'t>>> {
        let track_boundary = self.discarding_hidden_body();
        if track_boundary {
            self.push_hidden_body_boundary(
                block_rule.accepts_names,
                block_rule.accepts_newlines,
            );
        }

        let result = if as_paragraphs {
            self.get_body_elements_paragraphs(
                block_rule,
                restrict_quote_close,
                allow_inline_quote_close,
            )
        } else {
            self.get_body_elements_no_paragraphs(
                block_rule,
                restrict_quote_close,
                allow_inline_quote_close,
            )
        };

        if track_boundary {
            self.pop_hidden_body_boundary();
        }

        result
    }

    pub(crate) fn get_body_elements_with_context(
        &mut self,
        block_rule: &BlockRule,
        as_paragraphs: bool,
        body_start: BlockBodyStart,
    ) -> ParseResult<'r, 't, Vec<Element<'t>>> {
        self.get_body_elements_with_context_policy(
            block_rule,
            as_paragraphs,
            body_start,
            false,
        )
    }

    pub(crate) fn get_body_elements_with_literal_quote_context(
        &mut self,
        block_rule: &BlockRule,
        as_paragraphs: bool,
        body_start: BlockBodyStart,
    ) -> ParseResult<'r, 't, Vec<Element<'t>>> {
        self.get_body_elements_with_context_policy(
            block_rule,
            as_paragraphs,
            body_start,
            true,
        )
    }

    fn get_body_elements_with_context_policy(
        &mut self,
        block_rule: &BlockRule,
        as_paragraphs: bool,
        body_start: BlockBodyStart,
        literal_residual_quotes: bool,
    ) -> ParseResult<'r, 't, Vec<Element<'t>>> {
        if self.settings().layout.legacy()
            && !self.discarding_hidden_body()
            && body_start == BlockBodyStart::Inline
            && wikidot_requires_next_physical_line(block_rule)
        {
            return Err(self.make_err(ParseErrorKind::RuleFailed));
        }

        let Some(required_depth) = self.native_blockquote_depth() else {
            return self.get_body_elements(block_rule, as_paragraphs);
        };
        if body_start != BlockBodyStart::NextPhysicalLine {
            return self.get_body_elements(block_rule, as_paragraphs);
        }
        let allow_inline_quote_close = self.settings().layout.legacy()
            && !self.discarding_hidden_body()
            && block_rule.name == "block-collapsible";
        let has_close = self.has_native_blockquote_body_end_with_mode(
            block_rule,
            required_depth,
            allow_inline_quote_close,
        );
        let has_later_close =
            allow_inline_quote_close && !has_close && self.has_body_end_block(block_rule);
        if !has_close && !has_later_close {
            return Err(self.make_err(ParseErrorKind::RuleFailed));
        }

        let previous_cursor = self.quote_body_cursor();
        if literal_residual_quotes {
            self.install_quote_body_cursor_with_literal_residuals(required_depth);
        } else {
            self.install_quote_body_cursor(required_depth);
        }
        let previous_boundary_policy = self.quote_boundary_closes_body();
        self.set_quote_boundary_closes_body(has_later_close);
        let result = self.get_body_elements_internal(
            block_rule,
            as_paragraphs,
            true,
            allow_inline_quote_close,
        );
        let ended_at_boundary = if has_later_close && result.is_ok() {
            let mut boundary = self.clone();
            boundary.prepare_quote_body_line()? == QuoteBodyLineStatus::Boundary
        } else {
            false
        };
        if ended_at_boundary {
            self.set_pending_wikidot_collapsible_closer(true);
        }
        self.set_quote_boundary_closes_body(previous_boundary_policy);
        self.set_quote_body_cursor(previous_cursor);

        match result {
            Ok(success) => {
                if self.current().token == Token::LineBreak {
                    self.step()?;
                }
                Ok(success)
            }
            Err(error) => Err(error),
        }
    }

    /// Parse a hidden body with the normal grammar, discard its output and errors,
    /// and retain only the final token position.
    ///
    /// Parser metadata is shared by cheap parser clones, so it is restored from a
    /// transaction snapshot before the fork's token position is committed.
    pub fn discard_body_elements(
        &mut self,
        block_rule: &BlockRule,
    ) -> Result<(), ParseError> {
        let mutable_state = self.get_mutable_state();
        let mut fork = self.clone();
        let was_discarding = fork.discarding_hidden_body();
        fork.set_discarding_hidden_body(true);
        fork.push_hidden_body_boundary(
            block_rule.accepts_names,
            block_rule.accepts_newlines,
        );

        let discard_result =
            fork.consume_body_no_paragraphs(block_rule, false, false, |_| {});
        fork.pop_hidden_body_boundary();

        if discard_result.is_err() {
            // A malformed or unclosed hidden construct is ambiguous.  Keep the
            // conditional closed through EOF instead of falling back to visible
            // text outside an attacker-controlled delimiter.
            fork.skip_to_input_end()?;
        }

        fork.set_discarding_hidden_body(was_discarding);
        self.update(&fork);
        self.reset_mutable_state(mutable_state);
        Ok(())
    }

    pub(crate) fn discard_body_elements_with_literal_quote_context(
        &mut self,
        block_rule: &BlockRule,
        body_start: BlockBodyStart,
    ) -> Result<(), ParseError> {
        self.discard_body_elements_with_context_policy(block_rule, body_start, true)
    }

    fn discard_body_elements_with_context_policy(
        &mut self,
        block_rule: &BlockRule,
        body_start: BlockBodyStart,
        literal_residual_quotes: bool,
    ) -> Result<(), ParseError> {
        let Some(required_depth) = self.native_blockquote_depth() else {
            return self.discard_body_elements(block_rule);
        };
        if body_start != BlockBodyStart::NextPhysicalLine {
            return self.discard_body_elements(block_rule);
        }
        if !self.has_native_blockquote_body_end(block_rule, required_depth) {
            return Err(self.make_err(ParseErrorKind::RuleFailed));
        }

        let previous_cursor = self.quote_body_cursor();
        if literal_residual_quotes {
            self.install_quote_body_cursor_with_literal_residuals(required_depth);
        } else {
            self.install_quote_body_cursor(required_depth);
        }
        let result = self.discard_body_elements(block_rule);
        self.set_quote_body_cursor(previous_cursor);

        if result.is_ok() && self.current().token == Token::LineBreak {
            self.step()?;
        }
        result
    }

    fn get_body_elements_paragraphs(
        &mut self,
        block_rule: &BlockRule,
        restrict_quote_close: bool,
        allow_inline_quote_close: bool,
    ) -> ParseResult<'r, 't, Vec<Element<'t>>> {
        let mut first = true;
        let rule = self.rule();

        let is_end = move |parser: &mut Parser<'r, 't>| {
            let result = parser.verify_end_block(
                first,
                block_rule,
                restrict_quote_close,
                allow_inline_quote_close,
            );
            first = false;

            if result.is_none()
                && parser.discarding_hidden_body()
                && parser.at_hidden_body_ancestor_boundary()
            {
                return Err(parser.make_err(ParseErrorKind::BlockExpectedEnd));
            }

            Ok(result.is_some())
        };
        gather_paragraphs(self, rule, Some(is_end))
    }

    fn get_body_elements_no_paragraphs(
        &mut self,
        block_rule: &BlockRule,
        restrict_quote_close: bool,
        allow_inline_quote_close: bool,
    ) -> ParseResult<'r, 't, Vec<Element<'t>>> {
        let mut all_elements = Vec::new();
        let mut all_errors = Vec::new();
        let mut paragraph_safe = true;

        let process = |consumed: crate::parsing::ParseSuccess<'r, 't, Elements<'t>>| {
            let elements = consumed.chain(&mut all_errors, &mut paragraph_safe);
            all_elements.extend(elements);
        };
        self.consume_body_no_paragraphs(
            block_rule,
            restrict_quote_close,
            allow_inline_quote_close,
            process,
        )?;

        ok!(paragraph_safe; all_elements, all_errors)
    }

    fn consume_body_no_paragraphs<F>(
        &mut self,
        block_rule: &BlockRule,
        restrict_quote_close: bool,
        allow_inline_quote_close: bool,
        mut process: F,
    ) -> Result<(), ParseError>
    where
        F: FnMut(crate::parsing::ParseSuccess<'r, 't, Elements<'t>>),
    {
        let mut first = true;

        loop {
            if self.prepare_quote_body_line()? == QuoteBodyLineStatus::Boundary {
                return Err(self.make_err(ParseErrorKind::EndOfInput));
            }

            let result = self.verify_end_block(
                first,
                block_rule,
                restrict_quote_close,
                allow_inline_quote_close,
            );
            if result.is_some() {
                return Ok(());
            }

            if self.discarding_hidden_body() && self.at_hidden_body_ancestor_boundary() {
                return Err(self.make_err(ParseErrorKind::BlockExpectedEnd));
            }

            let wikidot_input_end = self.current().token == Token::InputEnd
                && self.settings().layout.legacy()
                && !self.discarding_hidden_body();
            if wikidot_input_end {
                return Ok(());
            }

            first = false;
            match consume(self) {
                Ok(consumed) => process(consumed),
                Err(_error)
                    if self.discarding_hidden_body()
                        && self.at_hidden_body_boundary() => {}
                Err(error) => return Err(error),
            }
        }
    }

    pub fn has_body_end_block(&self, block_rule: &BlockRule) -> bool {
        let mut parser = self.clone();
        let mut first = true;
        let mut traversed_token_states = Vec::new();

        loop {
            let token_start = parser.current().span.start;
            let exact_key = (block_rule.name, token_start, first);
            let equivalent_key = (block_rule.name, token_start, !first);
            let cached = self.block_end_scan_outcome(exact_key).or_else(|| {
                (parser.current().token != Token::LineBreak)
                    .then(|| self.block_end_scan_outcome(equivalent_key))
                    .flatten()
            });
            if let Some(outcome) = cached {
                let states = &traversed_token_states;
                self.cache_block_end_scan_outcomes(block_rule.name, states, outcome);
                return outcome;
            }
            traversed_token_states.push((token_start, first));

            if parser
                .verify_end_block(first, block_rule, false, false)
                .is_some()
            {
                self.cache_block_end_scan_outcomes(
                    block_rule.name,
                    &traversed_token_states,
                    true,
                );
                return true;
            }

            if parser.current().token == Token::InputEnd {
                self.cache_block_end_scan_outcomes(
                    block_rule.name,
                    &traversed_token_states,
                    false,
                );
                return false;
            }

            parser.step().expect("missing input end");
            first = false;
        }
    }

    pub(crate) fn has_two_body_end_blocks(&self, block_rule: &BlockRule) -> bool {
        let mut parser = self.clone();
        let mut first = true;
        let mut matches = 0_u8;
        let mut traversed_token_states = Vec::new();

        loop {
            let token_start = parser.current().span.start;
            let key = (block_rule.name, token_start, first);
            // Unlike the single-close scan, this result only counts raw close
            // markers in the suffix. Consuming an optional leading line break
            // changes where a close is recognized, but not how many exist.
            let cached = self.two_block_end_scan_outcome(key).or_else(|| {
                self.two_block_end_scan_outcome((block_rule.name, token_start, !first))
            });
            if let Some(suffix_matches) = cached {
                let total_matches = (matches + suffix_matches).min(2);
                self.cache_two_block_end_scan_outcomes(
                    block_rule.name,
                    &traversed_token_states,
                    total_matches,
                );
                return total_matches == 2;
            }
            traversed_token_states.push((token_start, first, matches));
            #[cfg(test)]
            self.increment_block_end_scan_token_visits();

            if parser
                .verify_end_block(first, block_rule, false, false)
                .is_some()
            {
                matches += 1;
                if matches == 2 {
                    self.cache_two_block_end_scan_outcomes(
                        block_rule.name,
                        &traversed_token_states,
                        matches,
                    );
                    return true;
                }
            } else if parser.current().token == Token::InputEnd {
                self.cache_two_block_end_scan_outcomes(
                    block_rule.name,
                    &traversed_token_states,
                    matches,
                );
                return false;
            } else {
                parser.step().expect("missing input end");
            }
            first = false;
        }
    }

    pub(crate) fn consume_body_end_block(
        &mut self,
        first: bool,
        block_rule: &BlockRule,
    ) -> bool {
        self.verify_end_block(first, block_rule, false, false)
            .is_some()
    }

    /// Whether the matching block end occurs before the current line ends.
    pub fn has_body_end_block_on_line(&self, block_rule: &BlockRule) -> bool {
        let mut parser = self.clone();

        loop {
            match parser.current().token {
                Token::InputEnd | Token::LineBreak | Token::ParagraphBreak => {
                    return false;
                }
                Token::LeftBlockEnd => {
                    let mut end = parser.clone();
                    if let Ok(name) = end.get_end_block() {
                        let name = name.strip_suffix('_').unwrap_or(name);
                        if block_rule
                            .accepts_names
                            .iter()
                            .any(|accepted| name.eq_ignore_ascii_case(accepted))
                        {
                            return true;
                        }
                    }
                }
                _ => {}
            }
            parser.step().expect("missing input end");
        }
    }

    /// Scan raw physical quote lines for a close at `required_depth`.
    ///
    /// The result depends only on immutable tokens, the block rule, and the
    /// required physical quote depth, so exact line-start outcomes are shared
    /// safely across speculative parser clones.
    pub(crate) fn has_native_blockquote_body_end(
        &self,
        block_rule: &BlockRule,
        required_depth: usize,
    ) -> bool {
        self.has_native_blockquote_body_end_with_mode(block_rule, required_depth, false)
    }

    #[rustfmt::skip]
    fn cache_native_quote_scan(
        &self,
        block_rule: &BlockRule,
        required_depth: usize,
        allow_inline_close: bool,
        line_starts: &[usize],
        outcome: QuoteScanOutcome,
    ) {
        self.cache_quote_scan_outcomes(block_rule.name, required_depth, allow_inline_close, line_starts, outcome);
    }

    #[rustfmt::skip]
    fn has_native_blockquote_body_end_with_mode(
        &self,
        block_rule: &BlockRule,
        required_depth: usize,
        allow_inline_close: bool,
    ) -> bool {
        let mut parser = self.clone();
        let mut traversed_line_starts = Vec::new();

        loop {
            let line_start = parser.current().span.start;
            let key = (block_rule.name, required_depth, allow_inline_close, line_start);
            if let Some(outcome) = self.quote_scan_outcome(key) {
                self.cache_native_quote_scan(block_rule, required_depth, allow_inline_close, &traversed_line_starts, outcome);
                return outcome == QuoteScanOutcome::HasCandidateClose;
            }
            traversed_line_starts.push(line_start);

            #[cfg(test)]
            self.increment_quote_scan_token_visits();

            let Some((_, absolute_depth, parser_after_prefix)) = parser.scan_absolute_quote_prefix(required_depth) else {
                self.cache_native_quote_scan(block_rule, required_depth, allow_inline_close, &traversed_line_starts, QuoteScanOutcome::Missing);
                return false;
            };
            parser.update(&parser_after_prefix);

            let mut first_candidate = true;
            while first_candidate
                || (allow_inline_close && !token_is_body_boundary(parser.current().token))
            {
                first_candidate = false;
                let mut end = parser.clone();
                let matching_close = end.get_end_block().is_ok_and(|name| block_rule_accepts_name(block_rule, name.strip_suffix('_').unwrap_or(name)));
                let valid_close = if matching_close
                    && absolute_depth == required_depth
                    && allow_inline_close
                {
                    true
                } else if matching_close && absolute_depth == required_depth {
                    let _ = end.get_optional_space();
                    token_is_body_boundary(end.current().token)
                } else {
                    false
                };
                if valid_close {
                    self.cache_native_quote_scan(block_rule, required_depth, allow_inline_close, &traversed_line_starts, QuoteScanOutcome::HasCandidateClose);
                    return true;
                }

                if allow_inline_close && !token_is_body_boundary(parser.current().token) {
                    #[cfg(test)]
                    self.increment_quote_scan_token_visits();
                    parser.step().expect("tokenization always ends with input-end");
                }
            }

            while !token_is_body_boundary(parser.current().token) {
                #[cfg(test)]
                self.increment_quote_scan_token_visits();
                parser.step().expect("tokenization always ends with input-end");
            }

            if parser.current().token != Token::LineBreak {
                self.cache_native_quote_scan(block_rule, required_depth, allow_inline_close, &traversed_line_starts, QuoteScanOutcome::Missing);
                return false;
            }

            #[cfg(test)]
            self.increment_quote_scan_token_visits();
            parser.step().expect("line break must precede input-end");
        }
    }

    // Block head / argument parsing
    pub fn get_head_map(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
    ) -> Result<Arguments<'t>, ParseError> {
        self.get_head_map_with_body_start(block_rule, in_head)
            .map(|(arguments, _)| arguments)
    }

    pub(crate) fn get_head_map_with_body_start(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
    ) -> Result<(Arguments<'t>, BlockBodyStart), ParseError> {
        let mut map = Arguments::new();
        if in_head {
            // Only process if the block isn't done yet
            loop {
                while token_is_argument_spacing(self.current().token) {
                    self.step()?;
                }

                // Try to get the argument key
                // Allows any token that matches the regular expression
                // i.e., alphanumeric, dash, or underscore
                //
                // This logic determines if we stop or keep getting arguments
                //
                // We could use collect_text_keep() here, but it messes with
                // get_head_block() so we just have it inline. Also it's a bit
                // strange since one of the outcomes is to break out of the loop.

                let start = self.current();
                let mut args_finished = false;
                loop {
                    let current = self.current();
                    match current.token {
                        // End parsing block head
                        Token::RightBlock => {
                            args_finished = true;
                            break;
                        }

                        // End parsing argument key
                        token if token_ends_argument_key(token) => break,

                        // Continue iterating to gather key
                        _ if ARGUMENT_KEY.is_match(current.slice) => {
                            self.step()?;
                        }

                        // Invalid token
                        _ => {
                            return Err(
                                self.make_err(ParseErrorKind::BlockMalformedArguments)
                            );
                        }
                    }
                }

                // Stop iterating for more argument key-value pairs
                if args_finished {
                    break std::convert::identity(());
                }

                // Gather argument key string slice
                let end = self.current();
                let key = self.full_text().slice_partial(start, end);
                if key.is_empty() {
                    return Err(self.make_err(ParseErrorKind::BlockMalformedArguments));
                }

                // Equal sign
                let space_before_equals = self.current().token == Token::Whitespace;
                self.get_optional_space()?;
                self.get_token(Token::Equals, ParseErrorKind::BlockMalformedArguments)?;

                // Get the argument value
                let space_after_equals = self.current().token == Token::Whitespace;
                self.get_optional_space()?;
                let bare = self.current().token != Token::DoubleQuote;
                let value = self.get_block_argument_value(block_rule, key)?;

                // Add to argument map
                if bare {
                    map.insert_bare(key, value);
                } else {
                    map.insert(key, value);
                }
                if space_before_equals || space_after_equals {
                    map.mark_spaced_equals();
                }
            }
        }

        let body_start = self.get_head_block_with_body_start(block_rule, in_head)?;
        Ok((map, body_start))
    }

    /// Parses a key-value block head using Wikidot's legacy `getAttrs`
    /// grammar in `Layout::Wikidot`, while retaining FTML's strict grammar in
    /// other layouts.
    pub fn get_head_map_wikidot(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
    ) -> Result<Arguments<'t>, ParseError> {
        self.get_head_map_with_body_start_wikidot(block_rule, in_head)
            .map(|(arguments, _)| arguments)
    }

    pub(crate) fn get_head_map_with_body_start_wikidot(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
    ) -> Result<(Arguments<'t>, BlockBodyStart), ParseError> {
        if !self.settings().layout.legacy() {
            return self.get_head_map_with_body_start(block_rule, in_head);
        }

        let arguments = if in_head {
            let start = self.current();
            while !matches!(self.current().token, Token::RightBlock | Token::InputEnd) {
                self.step()?;
            }
            let head_text = self.full_text().slice_partial(start, self.current());
            parse_wikidot_attributes(head_text)
        } else {
            Arguments::new_case_sensitive()
        };
        let body_start = self.get_head_block_with_body_start(block_rule, in_head)?;
        Ok((arguments, body_start))
    }

    pub fn get_head_name_map(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
    ) -> Result<(&'t str, Arguments<'t>), ParseError> {
        if !in_head {
            return Err(self.make_err(ParseErrorKind::BlockMissingName));
        }

        // Get module's name
        let missing_name = ParseErrorKind::ModuleMissingName;
        let (subname, in_head) = self.get_block_name_internal(missing_name)?;

        // Get arguments and end of block
        let arguments = self.get_head_map(block_rule, in_head)?;

        Ok((subname, arguments))
    }

    /// Parses a positional block-head value followed by Wikidot `getAttrs`
    /// arguments in `Layout::Wikidot`.
    pub fn get_head_name_map_wikidot(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
    ) -> Result<(&'t str, Arguments<'t>), ParseError> {
        if !self.settings().layout.legacy() {
            return self.get_head_name_map(block_rule, in_head);
        }
        if !in_head {
            return Err(self.make_err(ParseErrorKind::BlockMissingName));
        }

        let missing_name = ParseErrorKind::ModuleMissingName;
        let (subname, in_head) = self.get_block_name_internal(missing_name)?;
        let arguments = self.get_head_map_wikidot(block_rule, in_head)?;

        Ok((subname, arguments))
    }

    pub fn get_head_value<F, T>(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
        convert: F,
    ) -> Result<T, ParseError>
    where
        F: FnOnce(&Self, Option<&'t str>) -> Result<T, ParseError>,
    {
        self.get_head_value_with_body_start(block_rule, in_head, convert)
            .map(|(value, _)| value)
    }

    pub(crate) fn get_head_value_with_body_start<F, T>(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
        convert: F,
    ) -> Result<(T, BlockBodyStart), ParseError>
    where
        F: FnOnce(&Self, Option<&'t str>) -> Result<T, ParseError>,
    {
        let argument = if in_head {
            // Gather slice of tokens in value
            let end_conditions = [ParseCondition::current(Token::RightBlock)];
            let reject_conditions = [
                ParseCondition::current(Token::ParagraphBreak),
                ParseCondition::current(Token::LineBreak),
            ];
            let rule = self.rule();
            let kind = ParseErrorKind::BlockMalformedArguments;
            let malformed_arguments = Some(kind);
            let stops = &end_conditions;
            let rejects = &reject_conditions;
            let slice = collect_text(self, rule, stops, rejects, malformed_arguments)?;

            Some(slice)
        } else {
            None
        };

        // Convert the value into a type of the caller's choosing
        let value = convert(self, argument)?;

        // Set to false because the collection will always end the block
        let body_start = self.get_head_block_with_body_start(block_rule, false)?;
        Ok((value, body_start))
    }

    pub fn get_head_none(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
    ) -> Result<(), ParseError> {
        self.get_head_none_with_body_start(block_rule, in_head)
            .map(drop)
    }

    pub(crate) fn get_head_none_with_body_start(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
    ) -> Result<BlockBodyStart, ParseError> {
        self.get_optional_space()?;
        self.get_head_block_with_body_start(block_rule, in_head)
    }

    // Helper function to finish up the head block
    fn get_head_block_with_body_start(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
    ) -> Result<BlockBodyStart, ParseError> {
        // If we're still in the head, finish
        if in_head {
            self.get_token(Token::RightBlock, ParseErrorKind::BlockMissingCloseBrackets)?;
        }

        // If the block wants a newline after, take it
        //
        // It's fine if we're at the end of the input,
        // it could be an empty block type.
        if block_rule.accepts_newlines {
            if self.current().token == Token::LineBreak {
                self.step()?;
                return Ok(BlockBodyStart::NextPhysicalLine);
            }
            if self.current().token == Token::ParagraphBreak {
                return Ok(BlockBodyStart::NextPhysicalLine);
            }
        }

        Ok(BlockBodyStart::Inline)
    }

    // Utilities
    #[inline]
    pub fn set_block(&mut self, block_rule: &BlockRule) {
        self.set_rule(block_rule.rule());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::parsing::ParseErrorKind;
    use crate::parsing::rule::impls::block::blocks::{
        BLOCK_COLLAPSIBLE, BLOCK_DIV, BLOCK_IFTAGS,
    };
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn wikijump_block_head_rejects_invalid_argument_key_token() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        for input in [
            "[[collapsible @=\"value\"]]body[[/collapsible]]",
            "[[collapsible =\"value\"]]body[[/collapsible]]",
        ] {
            let tokenization = crate::tokenize(input);
            let (_, errors) = crate::parse(&tokenization, &page_info, &settings).into();

            assert!(
                errors
                    .iter()
                    .any(|error| error.kind() == ParseErrorKind::BlockMalformedArguments),
                "{input} should report BlockMalformedArguments: {errors:?}",
            );
        }
    }

    #[test]
    fn block_head_allows_line_break_before_argument() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[div\nclass=\"value\"]]\nbody\n[[/div]]");
        let (_, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn native_quote_close_scan_caches_every_traversed_line_start() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = (0..256)
            .map(|index| format!("> body-{index}\n"))
            .collect::<String>();
        let tokenization = crate::tokenize(&input);
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("first quote should follow input start");

        assert!(!parser.has_native_blockquote_body_end(&BLOCK_COLLAPSIBLE, 1));
        let first_scan_visits = parser.quote_scan_token_visits();
        assert!(first_scan_visits > 0);
        assert!(first_scan_visits <= tokenization.tokens().len() * 2);

        loop {
            if parser.current().token == Token::InputEnd {
                break;
            }
            assert_eq!(parser.current().token, Token::Quote);
            assert!(!parser.has_native_blockquote_body_end(&BLOCK_COLLAPSIBLE, 1));
            assert_eq!(parser.quote_scan_token_visits(), first_scan_visits);

            while !matches!(parser.current().token, Token::LineBreak | Token::InputEnd) {
                parser.step().expect("input end must remain available");
            }
            if parser.current().token == Token::LineBreak {
                parser.step().expect("next line or input end must exist");
            }
        }
    }

    #[test]
    fn block_end_scan_propagates_cached_suffix_outcomes() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("prefix suffix");

        let mut suffix = Parser::new(&tokenization, &page_info, &settings);
        suffix.step_n(3).expect("suffix token should exist");
        assert_eq!(suffix.current().slice, "suffix");
        assert!(!suffix.has_body_end_block(&BLOCK_DIV));

        let mut prefix = Parser::new(&tokenization, &page_info, &settings);
        prefix.step().expect("prefix token should exist");
        assert_eq!(prefix.current().slice, "prefix");
        assert!(!prefix.has_body_end_block(&BLOCK_DIV));
        assert_eq!(
            prefix.block_end_scan_outcome((
                BLOCK_DIV.name,
                prefix.current().span.start,
                true
            )),
            Some(false),
        );
    }

    #[test]
    fn two_block_end_scan_reuses_cached_suffix_outcomes() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input =
            format!("{}hidden\n[[/iftags]]", "[[iftags +missing]]\n".repeat(256),);
        let tokenization = crate::tokenize(&input);
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser.step().expect("the first nested opener should exist");

        assert!(!parser.has_two_body_end_blocks(&BLOCK_IFTAGS));
        let first_scan_visits = parser.block_end_scan_token_visits();
        assert!(first_scan_visits > 0);
        assert!(first_scan_visits <= tokenization.tokens().len());

        while parser.current().token != Token::InputEnd {
            assert!(!parser.has_two_body_end_blocks(&BLOCK_IFTAGS));
            parser.step().expect("input end must remain available");
        }
        assert!(parser.block_end_scan_token_visits() <= tokenization.tokens().len() * 2);
    }

    #[test]
    fn native_quote_prefix_and_close_scan_cover_modes_boundaries_and_depths() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        for input in [">> body", "> > body"] {
            let tokenization = crate::tokenize(input);
            let mut parser = Parser::new(&tokenization, &page_info, &settings);
            parser.step().unwrap();
            let (content_start, depth, after_prefix) =
                parser.scan_absolute_quote_prefix(2).unwrap();
            assert_eq!(depth, 2);
            assert_eq!(&input[content_start..], "body");
            assert_eq!(after_prefix.current().slice, "body");
        }

        let tokenization = crate::tokenize("> body");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser.step().unwrap();
        assert!(parser.scan_absolute_quote_prefix(2).is_none());

        for (input, inline, expected) in [
            ("> before [[/collapsible]] trailing\n", true, true),
            ("> before [[/collapsible]] trailing\n", false, false),
            ("> [[/collapsible]]\n", false, true),
            ("> body", false, false),
            ("> body\n\n", false, false),
            (">> [[/collapsible]]\n> [[/collapsible]]\n", false, true),
            ("plain", false, false),
        ] {
            let tokenization = crate::tokenize(input);
            let mut parser = Parser::new(&tokenization, &page_info, &settings);
            parser.step().unwrap();
            assert_eq!(
                parser.has_native_blockquote_body_end_with_mode(
                    &BLOCK_COLLAPSIBLE,
                    1,
                    inline,
                ),
                expected,
                "{input:?}, inline={inline}",
            );
            assert_eq!(
                parser.has_native_blockquote_body_end_with_mode(
                    &BLOCK_COLLAPSIBLE,
                    1,
                    inline,
                ),
                expected,
                "cached {input:?}, inline={inline}",
            );
        }
    }

    #[test]
    fn restricted_quote_close_rejects_trailing_content_and_no_paragraph_boundary() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("> [[/collapsible]] trailing");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser.step().unwrap();
        parser.install_quote_body_cursor(1);
        assert_eq!(
            parser.prepare_quote_body_line().unwrap(),
            QuoteBodyLineStatus::Prepared
        );
        assert!(
            parser
                .verify_end_block(true, &BLOCK_COLLAPSIBLE, true, false)
                .is_none()
        );

        let tokenization = crate::tokenize("plain");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser.step().unwrap();
        parser.install_quote_body_cursor(1);
        let error = parser
            .get_body_elements_no_paragraphs(&BLOCK_DIV, true, false)
            .expect_err("unquoted content must end an adapted body");
        assert_eq!(error.kind(), ParseErrorKind::EndOfInput);
    }

    #[test]
    fn block_body_generic_accepts_matching_end_block() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[/div]]");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser.step().expect("end block should follow input start");

        let (start, end) = parser
            .get_body_generic(&BLOCK_DIV, |_| Ok(()))
            .expect("matching end block should terminate the body");

        assert_eq!(start.slice, "[[/");
        assert_eq!(end.slice, "[[/");
    }
}
