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

use super::arguments::Arguments;
use super::{BlockRule, RULE_BLOCK};
use crate::parsing::collect::{collect_text, collect_text_keep};
use crate::parsing::condition::ParseCondition;
use crate::parsing::consume::consume;
use crate::parsing::parser::{QuoteBodyLineStatus, QuoteScanOutcome};
use crate::parsing::{
    ExtractedToken, ParseError, ParseErrorKind, ParseResult, Parser, Token,
    gather_paragraphs,
};
use crate::tree::Element;
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
    ) -> Option<&'r ExtractedToken<'t>> {
        self.save_evaluate_fn(|parser| {
            if restrict_quote_close && !parser.quote_body_close_allowed_here() {
                return Ok(false);
            }

            // Check that the end block is on a new line, if required
            if block_rule.accepts_newlines {
                // Only check after the first, to permit empty blocks
                if !first_iteration {
                    parser.get_optional_line_break()?;
                }
            }

            // Check if it's an end block
            //
            // This will ignore any errors produced,
            // since it's just more text
            let name = parser.get_end_block()?;

            // Remove underscore for score flag
            let name = name.strip_suffix('_').unwrap_or(name);

            // Check if it's valid
            for end_block_name in block_rule.accepts_names {
                if name.eq_ignore_ascii_case(end_block_name) {
                    if restrict_quote_close {
                        parser.get_optional_space()?;
                        if !matches!(
                            parser.current().token,
                            Token::LineBreak | Token::ParagraphBreak | Token::InputEnd,
                        ) {
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
        let start = self.current();

        loop {
            let at_end_block = self.verify_end_block(first, block_rule, false);

            // If there's a match, return the last body token
            if let Some(end) = at_end_block {
                return Ok((start, end));
            }

            // Run the passed-in closure
            process(self)?;

            // Step and continue
            self.step()?;
            first = false;
        }
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

    fn get_native_blockquote_body_text(
        &mut self,
        block_rule: &BlockRule,
        required_depth: usize,
    ) -> Result<Cow<'t, str>, ParseError> {
        let has_close = if self.current().token == Token::LineBreak {
            let mut scan = self.clone();
            scan.step()?;
            scan.has_native_blockquote_body_end_with_mode(
                block_rule,
                required_depth,
                true,
            )
        } else {
            self.has_native_blockquote_body_end_with_mode(
                block_rule,
                required_depth,
                true,
            )
        };
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

            let Some((content_start, absolute_depth, mut scan)) =
                self.scan_absolute_quote_prefix(required_depth)
            else {
                return Err(self.make_err(ParseErrorKind::EndOfInput));
            };
            scan.set_quote_body_cursor(outer_prepared.quote_body_cursor());

            while !matches!(
                scan.current().token,
                Token::LineBreak | Token::ParagraphBreak | Token::InputEnd,
            ) {
                let marker_start = scan.current().span.start;
                let mut end = scan.clone();
                if absolute_depth == required_depth
                    && let Ok(name) = end.get_end_block()
                {
                    let name = name.strip_suffix('_').unwrap_or(name);
                    if block_rule
                        .accepts_names
                        .iter()
                        .any(|accepted| name.eq_ignore_ascii_case(accepted))
                    {
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
        self.get_body_elements_internal(block_rule, as_paragraphs, false)
    }

    fn get_body_elements_internal(
        &mut self,
        block_rule: &BlockRule,
        as_paragraphs: bool,
        restrict_quote_close: bool,
    ) -> ParseResult<'r, 't, Vec<Element<'t>>> {
        if as_paragraphs {
            self.get_body_elements_paragraphs(block_rule, restrict_quote_close)
        } else {
            self.get_body_elements_no_paragraphs(block_rule, restrict_quote_close)
        }
    }

    pub(crate) fn get_body_elements_with_context(
        &mut self,
        block_rule: &BlockRule,
        as_paragraphs: bool,
        body_start: BlockBodyStart,
    ) -> ParseResult<'r, 't, Vec<Element<'t>>> {
        let Some(required_depth) = self.native_blockquote_depth() else {
            return self.get_body_elements(block_rule, as_paragraphs);
        };
        if body_start != BlockBodyStart::NextPhysicalLine {
            return self.get_body_elements(block_rule, as_paragraphs);
        }
        if !self.has_native_blockquote_body_end(block_rule, required_depth) {
            return Err(self.make_err(ParseErrorKind::RuleFailed));
        }

        let previous_cursor = self.quote_body_cursor();
        self.install_quote_body_cursor(required_depth);
        let result = self.get_body_elements_internal(block_rule, as_paragraphs, true);
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

    fn get_body_elements_paragraphs(
        &mut self,
        block_rule: &BlockRule,
        restrict_quote_close: bool,
    ) -> ParseResult<'r, 't, Vec<Element<'t>>> {
        let mut first = true;
        let rule = self.rule();

        let is_end = move |parser: &mut Parser<'r, 't>| {
            let result = parser.verify_end_block(first, block_rule, restrict_quote_close);
            first = false;

            Ok(result.is_some())
        };
        gather_paragraphs(self, rule, Some(is_end))
    }

    fn get_body_elements_no_paragraphs(
        &mut self,
        block_rule: &BlockRule,
        restrict_quote_close: bool,
    ) -> ParseResult<'r, 't, Vec<Element<'t>>> {
        let mut all_elements = Vec::new();
        let mut all_errors = Vec::new();
        let mut paragraph_safe = true;
        let mut first = true;

        loop {
            if self.prepare_quote_body_line()? == QuoteBodyLineStatus::Boundary {
                return Err(self.make_err(ParseErrorKind::EndOfInput));
            }

            let result = self.verify_end_block(first, block_rule, restrict_quote_close);
            if result.is_some() {
                return ok!(paragraph_safe; all_elements, all_errors);
            }

            let wikidot_input_end = self.current().token == Token::InputEnd
                && self.settings().layout.legacy();
            if wikidot_input_end {
                return ok!(paragraph_safe; all_elements, all_errors);
            }

            first = false;
            let elements = consume(self)?.chain(&mut all_errors, &mut paragraph_safe);
            all_elements.extend(elements);
        }
    }

    pub fn has_body_end_block(&self, block_rule: &BlockRule) -> bool {
        let mut parser = self.clone();
        let mut first = true;

        loop {
            if parser.verify_end_block(first, block_rule, false).is_some() {
                return true;
            }

            if parser.current().token == Token::InputEnd {
                return false;
            }

            parser.step().expect("missing input end");
            first = false;
        }
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
            let key = (
                block_rule.name,
                required_depth,
                allow_inline_close,
                line_start,
            );
            if let Some(outcome) = self.quote_scan_outcome(key) {
                self.cache_quote_scan_outcomes(
                    block_rule.name,
                    required_depth,
                    allow_inline_close,
                    &traversed_line_starts,
                    outcome,
                );
                return outcome == QuoteScanOutcome::HasCandidateClose;
            }
            traversed_line_starts.push(line_start);

            #[cfg(test)]
            self.increment_quote_scan_token_visits();

            let Some((_, absolute_depth, parser_after_prefix)) =
                parser.scan_absolute_quote_prefix(required_depth)
            else {
                self.cache_quote_scan_outcomes(
                    block_rule.name,
                    required_depth,
                    allow_inline_close,
                    &traversed_line_starts,
                    QuoteScanOutcome::Missing,
                );
                return false;
            };
            parser.update(&parser_after_prefix);

            loop {
                let mut end = parser.clone();
                let matching_close = end.get_end_block().is_ok_and(|name| {
                    let name = name.strip_suffix('_').unwrap_or(name);
                    block_rule
                        .accepts_names
                        .iter()
                        .any(|accepted| name.eq_ignore_ascii_case(accepted))
                });
                let valid_close = if matching_close
                    && absolute_depth == required_depth
                    && allow_inline_close
                {
                    true
                } else if matching_close && absolute_depth == required_depth {
                    let _ = end.get_optional_space();
                    matches!(
                        end.current().token,
                        Token::LineBreak | Token::ParagraphBreak | Token::InputEnd,
                    )
                } else {
                    false
                };
                if valid_close {
                    self.cache_quote_scan_outcomes(
                        block_rule.name,
                        required_depth,
                        allow_inline_close,
                        &traversed_line_starts,
                        QuoteScanOutcome::HasCandidateClose,
                    );
                    return true;
                }

                if !allow_inline_close
                    || matches!(
                        parser.current().token,
                        Token::LineBreak | Token::ParagraphBreak | Token::InputEnd,
                    )
                {
                    break;
                }

                #[cfg(test)]
                self.increment_quote_scan_token_visits();
                if parser.step().is_err() {
                    break;
                }
            }

            while !matches!(
                parser.current().token,
                Token::LineBreak | Token::ParagraphBreak | Token::InputEnd,
            ) {
                #[cfg(test)]
                self.increment_quote_scan_token_visits();
                if parser.step().is_err() {
                    self.cache_quote_scan_outcomes(
                        block_rule.name,
                        required_depth,
                        allow_inline_close,
                        &traversed_line_starts,
                        QuoteScanOutcome::Missing,
                    );
                    return false;
                }
            }

            if parser.current().token != Token::LineBreak {
                self.cache_quote_scan_outcomes(
                    block_rule.name,
                    required_depth,
                    allow_inline_close,
                    &traversed_line_starts,
                    QuoteScanOutcome::Missing,
                );
                return false;
            }

            #[cfg(test)]
            self.increment_quote_scan_token_visits();
            if parser.step().is_err() {
                self.cache_quote_scan_outcomes(
                    block_rule.name,
                    required_depth,
                    allow_inline_close,
                    &traversed_line_starts,
                    QuoteScanOutcome::Missing,
                );
                return false;
            }
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
                self.get_optional_space()?;
                self.get_token(Token::Equals, ParseErrorKind::BlockMalformedArguments)?;

                // Get the argument value
                self.get_optional_space()?;
                let value = self.get_quoted_string(RULE_BLOCK)?;

                // Add to argument map
                map.insert(key, value);
            }
        }

        let body_start = self.get_head_block_with_body_start(block_rule, in_head)?;
        Ok((map, body_start))
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

    pub fn get_head_value<F, T>(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
        convert: F,
    ) -> Result<T, ParseError>
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
        self.get_head_block(block_rule, false)?;
        Ok(value)
    }

    pub fn get_head_none(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
    ) -> Result<(), ParseError> {
        self.get_optional_space()?;
        self.get_head_block(block_rule, in_head)?;
        Ok(())
    }

    // Helper function to finish up the head block
    fn get_head_block(
        &mut self,
        block_rule: &BlockRule,
        in_head: bool,
    ) -> Result<(), ParseError> {
        self.get_head_block_with_body_start(block_rule, in_head)
            .map(drop)
    }

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
    use crate::parsing::rule::impls::block::blocks::{BLOCK_COLLAPSIBLE, BLOCK_DIV};
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn block_head_rejects_invalid_argument_key_token() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        for input in [
            "[[div @=\"value\"]]body[[/div]]",
            "[[div =\"value\"]]body[[/div]]",
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
