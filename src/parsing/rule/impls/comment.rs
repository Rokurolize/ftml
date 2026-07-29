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
        if parser.settings().layout.legacy()
            && parser.discarding_hidden_body()
            && parser.at_hidden_body_boundary()
        {
            return Err(parser.make_err(ParseErrorKind::BlockExpectedEnd));
        }

        let token = parser.current().token;

        trace!("Received token '{}' inside comment", token.name());

        match token {
            // Hit the end of the comment, return
            Token::RightComment => {
                trace!("Reached end of comment, returning");
                parser.step()?;
                return ok!(Elements::None);
            }

            // Wikidot accepts any run of at least two hyphens immediately
            // followed by `]` as a comment closer while it is scanning a
            // comment. Keep this contextual: outside a comment, longer runs
            // retain their ordinary dash and horizontal-rule tokenization.
            _ if let Some(token_count) = wikidot_extended_closer_token_count(parser) => {
                trace!("Reached extended Wikidot comment closer, returning");
                parser.step_n(token_count)?;
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

fn wikidot_extended_closer_token_count(parser: &Parser<'_, '_>) -> Option<usize> {
    let mut hyphens = 0;

    for (index, token) in std::iter::once(parser.current())
        .chain(parser.remaining())
        .enumerate()
    {
        if !token.slice.is_empty() && token.slice.bytes().all(|byte| byte == b'-') {
            hyphens += token.slice.len();
            continue;
        }

        return (hyphens >= 2
            && token.token == Token::RightBracket
            && token.slice == "]")
            .then_some(index + 1);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
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

    #[test]
    fn wikidot_unterminated_comment_opener_falls_back_with_typographic_dash() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[!-- unfinished");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(!errors.is_empty());
        assert_eq!(html, "<p>[!\u{2014} unfinished</p>");
    }

    #[test]
    fn wikidot_unmatched_comment_closer_falls_back_with_typographic_dash() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("raw-comment --]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(!errors.is_empty());
        assert_eq!(html, "<p>raw-comment \u{2014}]</p>");
    }

    #[test]
    fn wikidot_unmatched_extended_comment_closers_remain_ordinary_dashes() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        for (source, expected) in [
            ("raw ---]", "<p>raw \u{2014}-]</p>"),
            ("raw ----]", "<p>raw \u{2014}\u{2014}]</p>"),
        ] {
            let tokenization = crate::tokenize(source);
            let (tree, _) = crate::parse(&tokenization, &page_info, &settings).into();
            let html = HtmlRender.render(&tree, &page_info, &settings).body;

            assert_eq!(html, expected, "{source:?}");
        }
    }

    #[test]
    fn wikidot_comment_closer_accepts_extra_leading_hyphens() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        for source in [
            "[!-- hidden ---]\nvisible",
            "[!----\nhidden module-shaped text\n---]\nvisible",
            "[!-- hidden ----]\nvisible",
            "[!-- hidden -----]\nvisible",
        ] {
            let tokenization = crate::tokenize(source);
            let (tree, errors) =
                crate::parse(&tokenization, &page_info, &settings).into();
            let html = HtmlRender.render(&tree, &page_info, &settings).body;

            assert!(errors.is_empty(), "{source:?}: {errors:?}");
            assert_eq!(html, "<p>visible</p>", "{source:?}");
        }
    }
}
