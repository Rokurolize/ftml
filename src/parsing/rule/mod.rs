/*
 * parsing/rule/mod.rs
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
use crate::parsing::Parser;
use std::fmt::{self, Debug};

mod mapping;

pub mod impls;

pub use self::mapping::get_rules_for_token;

/// Defines a rule that can possibly match tokens and return an `Element`.
#[derive(Copy, Clone)]
pub struct Rule {
    /// The name for this rule, in kebab-case.
    ///
    /// It must be globally unique.
    name: &'static str,

    /// What requirements this rule needs regarding its position in a line.
    position: LineRequirement,

    /// The consumption attempt function for this rule.
    try_consume_fn: TryConsumeFn,
}

impl Rule {
    pub fn name(self) -> &'static str {
        self.name
    }

    #[inline]
    pub fn try_consume<'r, 't>(
        self,
        parser: &mut Parser<'r, 't>,
    ) -> ParseResult<'r, 't, Elements<'t>> {
        debug!("Trying to consume for parse rule {}", self.name);

        // Check that the line position matches what the rule wants.
        if let LineRequirement::StartOfLine = self.position
            && !parser.start_of_line()
        {
            return Err(parser.make_err(ParseErrorKind::NotStartOfLine));
        }

        // Fork parser and try running the rule.
        let parser_state = parser.get_mutable_state();
        let mut sub_parser = parser.clone_with_rule(self);
        let result = (self.try_consume_fn)(&mut sub_parser);

        match result {
            // Rule succeeded, ensure that changes from the subparser are persisted.
            Ok(ref output) => {
                // First, ensure there aren't any partial elements in the result.
                output.check_partials(parser)?;

                // Now, finally save the parser state since it succeeded.
                parser.update(&sub_parser);
            }

            // Rule failed, ensure that any changes are rolled back.
            //
            // While normally discarding the subparser is sufficient,
            // some annoying mutable fields are
            Err(_) => parser.reset_mutable_state(parser_state),
        }

        result
    }
}

impl Debug for Rule {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Rule")
            .field("name", &self.name)
            .field("position", &self.position)
            .field("try_consume_fn", &(self.try_consume_fn as *const ()))
            .finish()
    }
}

/// The enum describing what requirements a rule has regarding lines.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub enum LineRequirement {
    /// This rule does not care where it is in a line.
    Any,

    /// This rule may only activate when it is at the start of a line.
    ///
    /// This includes situations which are not technically line breaks,
    /// such as start of input and paragraph breaks.
    StartOfLine,
}

/// The function type for actually trying to consume tokens
pub type TryConsumeFn = for<'p, 'r, 't> fn(
    parser: &'p mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>>;

#[cfg(test)]
mod tests {
    use super::impls::{RULE_HEADER, RULE_TEXT};
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::parsing::{ParseErrorKind, Parser};
    use crate::settings::{WikitextMode, WikitextSettings};
    use crate::tree::Elements;

    #[test]
    fn rule_metadata_and_line_requirements_are_exercised() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        assert_eq!(RULE_TEXT.name(), "text");
        let debug = format!("{RULE_TEXT:?}");
        assert!(debug.contains("Rule"));
        assert!(debug.contains("text"));

        let text_tokens = crate::tokenize("alpha");
        let mut text_parser = Parser::new(&text_tokens, &page_info, &settings);
        text_parser
            .step()
            .expect("identifier should follow input start");
        let text = RULE_TEXT
            .try_consume(&mut text_parser)
            .expect("text rule should accept any line position");
        assert_eq!(text.item, Elements::Single(text!("alpha")));

        let header_tokens = crate::tokenize("alpha + heading");
        let mut header_parser = Parser::new(&header_tokens, &page_info, &settings);
        header_parser
            .step()
            .expect("identifier should follow input start");
        header_parser
            .step()
            .expect("space should follow identifier");
        header_parser.step().expect("heading should follow space");
        let error = RULE_HEADER
            .try_consume(&mut header_parser)
            .expect_err("header rule should require start of line");
        assert_eq!(error.kind(), ParseErrorKind::NotStartOfLine);
    }
}
