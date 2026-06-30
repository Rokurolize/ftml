/*
 * parsing/rule/impls/comment.rs
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

pub const RULE_COMMENT: Rule = Rule {
    name: "comment",
    position: LineRequirement::Any,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Consuming tokens until end of comment");

    assert_step(parser, Token::LeftComment)?;

    loop {
        let token = parser.current().token;

        trace!("Received token '{}' inside comment", token.name());

        match token {
            // Hit the end of the comment, return
            Token::RightComment => {
                trace!("Reached end of comment, returning");
                parser.step()?;
                return ok!(Elements::None);
            }

            // Hit the end of the input, abort
            Token::InputEnd => {
                trace!("Reached end of input, aborting");
                return Err(parser.make_err(ParseErrorKind::EndOfInput));
            }

            // Consume any other token
            _ => {
                trace!("Token inside comment received. Discarding.");
                parser.step()?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn comment_rule_rejects_unterminated_comment() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[!-- unfinished");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("left comment token should follow input start");
        parser.set_rule(RULE_COMMENT);

        let error = RULE_COMMENT
            .try_consume(&mut parser)
            .expect_err("unterminated comment should fail");
        assert_eq!(error.kind(), ParseErrorKind::EndOfInput);
    }
}
