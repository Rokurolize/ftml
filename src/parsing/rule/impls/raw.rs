/*
 * parsing/rule/impls/raw.rs
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

macro_rules! raw {
    ($value:expr) => {
        Element::Raw(cow!($value))
    };
}

pub const RULE_RAW: Rule = Rule {
    name: "raw",
    position: LineRequirement::Any,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Consuming tokens until end of raw");

    // Are we in a @@..@@ type raw, or a @<..>@ type?
    let ending_token = match parser.current().token {
        Token::Raw => Token::Raw,
        Token::LeftRaw => Token::RightRaw,
        _ => panic!("Current token is not a starting raw"),
    };

    // Check for four special cases:
    // * Raw Raw  "@" -> Element::Raw("@")
    // * Raw Raw !Raw -> Element::Raw("")
    // * Raw Raw  Raw -> Element::Raw("@@")
    // * Raw ??   Raw -> Element::Raw(slice)
    if ending_token == Token::Raw {
        trace!("First token is '@@', checking for special cases");

        // Get next two tokens. If they don't exist, exit early
        let next_1 = parser.look_ahead_err(0)?;
        let next_2 = parser.look_ahead_err(1)?;

        // Determine which case they fall under
        let special_case = match (next_1.token, next_2.token) {
            // "@@@@@@" -> Element::Raw("@@")
            (Token::Raw, Token::Raw) => {
                trace!("Found meta-raw (\"@@@@@@\"), returning");
                parser.step_n(3)?;
                Some(raw!("@@"))
            }

            // "@@@@@" -> Element::Raw("@")
            // This case is strange since the lexer returns Raw Raw Other (@@ @@ @)
            // So we capture this and return the intended output
            (Token::Raw, Token::Other) => {
                if next_2.slice == "@" {
                    trace!("Found single-raw (\"@@@@@\"), returning");
                    parser.step_n(3)?;
                    Some(raw!("@"))
                } else {
                    trace!("Found empty raw (\"@@@@\"), followed by other text");
                    parser.step_n(2)?;
                    Some(raw!(""))
                }
            }

            // "@@@@" -> Element::Raw("")
            // Only consumes two tokens.
            (Token::Raw, _) => {
                trace!("Found empty raw (\"@@@@\"), returning");
                parser.step_n(2)?;
                Some(raw!(""))
            }

            // "@@ \n @@" -> Abort
            (Token::LineBreak, Token::Raw) | (Token::ParagraphBreak, Token::Raw) => {
                trace!("Found interrupted raw, aborting");
                return Err(parser.make_err(ParseErrorKind::RuleFailed));
            }

            // "@@ [something] @@" -> Element::Raw(token)
            (_, Token::Raw) => {
                trace!("Found single-element raw, returning");
                parser.step_n(3)?;
                Some(raw!(next_1.slice))
            }

            // Other, proceed with rule logic
            _ => None,
        };

        if let Some(element) = special_case {
            return success_elements(element);
        }
    }

    // Handle the other cases, which are:
    // * "@@ [tokens] @@"
    // * "@< [tokens] >@"
    //
    // Collect the first and last token to build a slice of its contents.
    // The last will be updated with each step in the iterator.

    let current = parser.step()?;
    let (start, mut end) = (current, current);

    loop {
        let token = parser.current().token;

        trace!("Received token '{}' inside raw", token.name());

        if matches!(token, Token::RightRaw | Token::Raw) {
            if token == ending_token {
                trace!("Reached end of raw, returning");

                let slice = parser.full_text().slice_partial(start, end);
                parser.step()?;

                let element = Element::Raw(cow!(slice));
                return success_elements(element);
            }

            trace!("Wasn't end of raw, continuing");
        } else if matches!(token, Token::LineBreak | Token::ParagraphBreak) {
            trace!("Reached newline, aborting");
            return Err(parser.make_err(ParseErrorKind::RuleFailed));
        } else if token == Token::InputEnd {
            trace!("Reached end of input, aborting");
            return Err(parser.make_err(ParseErrorKind::EndOfInput));
        }

        trace!("Appending present token to raw");

        // Update last token and step.
        end = parser.step()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::parsing::Parser;
    use crate::settings::{WikitextMode, WikitextSettings};

    fn with_raw_elements<R>(
        input: &str,
        assert_result: impl FnOnce(Result<Elements<'_>, ParseError>) -> R,
    ) -> R {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(input);
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        let result = parser
            .step()
            .and_then(|_| try_consume_fn(&mut parser))
            .map(|success| success.item);

        assert_result(result)
    }

    #[test]
    fn raw_rule_handles_short_raw_special_cases() {
        with_raw_elements("@@@@@@", |result| {
            assert_eq!(result.unwrap(), Elements::Single(raw!("@@")));
        });
        with_raw_elements("@@@@@", |result| {
            assert_eq!(result.unwrap(), Elements::Single(raw!("@")));
        });
        with_raw_elements("@@@@", |result| {
            assert_eq!(result.unwrap(), Elements::Single(raw!("")));
        });
        with_raw_elements("@@token@@", |result| {
            assert_eq!(result.unwrap(), Elements::Single(raw!("token")));
        });
        with_raw_elements("@@\n@@", |result| {
            assert!(matches!(
                result.unwrap_err().kind(),
                ParseErrorKind::RuleFailed
            ));
        });
    }

    #[test]
    fn raw_rule_collects_long_raw_and_left_raw_forms() {
        with_raw_elements("@@a >@ b@@", |result| {
            assert_eq!(result.unwrap(), Elements::Single(raw!("a >@ b")));
        });
        with_raw_elements("@<a @@ b>@", |result| {
            assert_eq!(result.unwrap(), Elements::Single(raw!("a @@ b")));
        });
    }

    #[test]
    #[should_panic(expected = "Current token is not a starting raw")]
    fn raw_rule_panics_if_called_on_non_raw_token() {
        with_raw_elements("plain text", |_| {});
    }
}
