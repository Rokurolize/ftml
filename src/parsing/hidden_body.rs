/*
 * parsing/hidden_body.rs
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

//! Parser-local state for parsing conditional bodies into a discard sink.
use super::{ParseError, Parser, Token};

#[derive(Debug, Copy, Clone)]
pub(super) struct HiddenBodyBoundary {
    accepts_names: &'static [&'static str],
    accepts_newlines: bool,
}

impl<'r, 't> Parser<'r, 't> {
    #[inline]
    pub(crate) fn discarding_hidden_body(&self) -> bool {
        self.discarding_hidden_body
    }

    #[inline]
    pub(crate) fn set_discarding_hidden_body(&mut self, value: bool) {
        self.discarding_hidden_body = value;
    }

    pub(crate) fn push_hidden_body_boundary(
        &mut self,
        accepts_names: &'static [&'static str],
        accepts_newlines: bool,
    ) {
        self.hidden_body_boundaries.push(HiddenBodyBoundary {
            accepts_names,
            accepts_newlines,
        });
    }

    pub(crate) fn pop_hidden_body_boundary(&mut self) {
        self.hidden_body_boundaries
            .pop()
            .expect("hidden body boundary stack underflow");
    }

    pub(crate) fn at_hidden_body_boundary(&self) -> bool
    where
        'r: 't,
    {
        self.at_hidden_body_boundary_in(&self.hidden_body_boundaries)
    }

    pub(crate) fn at_hidden_body_ancestor_boundary(&self) -> bool
    where
        'r: 't,
    {
        let Some((_, ancestors)) = self.hidden_body_boundaries.split_last() else {
            return false;
        };

        self.at_hidden_body_boundary_in(ancestors)
    }

    fn at_hidden_body_boundary_in(&self, boundaries: &[HiddenBodyBoundary]) -> bool
    where
        'r: 't,
    {
        if boundaries.is_empty() {
            return false;
        }

        let after_line_break = self.current().token == Token::LineBreak;
        if !after_line_break && self.current().token != Token::LeftBlockEnd {
            return false;
        }

        let mut probe = self.clone();
        if after_line_break && probe.get_optional_line_break().is_err() {
            return false;
        }

        let Ok(name) = probe.get_end_block() else {
            return false;
        };
        let name = name.strip_suffix('_').unwrap_or(name);

        boundaries.iter().rev().any(|boundary| {
            (!after_line_break || boundary.accepts_newlines)
                && boundary
                    .accepts_names
                    .iter()
                    .any(|accepted| name.eq_ignore_ascii_case(accepted))
        })
    }

    pub(crate) fn skip_to_input_end(&mut self) -> Result<(), ParseError> {
        while self.current().token != Token::InputEnd {
            self.step()?;
        }

        Ok(())
    }
}
