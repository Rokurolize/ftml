/*
 * parsing/rule/impls/bibcite.rs
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

pub const RULE_BIBCITE: Rule = Rule {
    name: "bibcite",
    position: LineRequirement::Any,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create bibcite element");
    assert_step(parser, Token::LeftParentheses)?;

    // This is like a poor man's block, it's "((bibcite <label>))"
    let current = parser.current();
    if current.token != Token::Identifier
        || !current.slice.eq_ignore_ascii_case("bibcite")
    {
        warn!("'((' not followed by 'bibcite', failing rule");
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    parser.step()?;

    // Then check the next token is a space
    if !matches!(parser.current().token, Token::Whitespace) {
        warn!("'((bibcite' not followed by a space, failing rule");
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    parser.step()?;

    let label = collect_text(
        parser,
        RULE_BIBCITE,
        &[ParseCondition::current(Token::RightParentheses)],
        &[
            ParseCondition::current(Token::Whitespace),
            ParseCondition::current(Token::ParagraphBreak),
            ParseCondition::current(Token::LineBreak),
        ],
        None,
    )?;

    ok!(Element::BibliographyCite {
        label: cow!(label),
        brackets: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn inline_bibcite_rejects_wrong_marker() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("(( label))");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("left parentheses should follow input start");
        parser.set_rule(RULE_BIBCITE);

        let error = RULE_BIBCITE
            .try_consume(&mut parser)
            .expect_err("non-bibcite marker should fail");
        assert_eq!(error.kind(), ParseErrorKind::RuleFailed);
    }

    #[test]
    fn inline_bibcite_rejects_wrong_identifier() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("((notbibcite label))");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("left parentheses should follow input start");
        parser.set_rule(RULE_BIBCITE);

        let error = RULE_BIBCITE
            .try_consume(&mut parser)
            .expect_err("wrong marker identifier should fail");
        assert_eq!(error.kind(), ParseErrorKind::RuleFailed);
    }

    #[test]
    fn inline_bibcite_requires_label_separator() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("((bibcite))");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("left parentheses should follow input start");
        parser.set_rule(RULE_BIBCITE);

        let error = RULE_BIBCITE
            .try_consume(&mut parser)
            .expect_err("missing separator should fail");
        assert_eq!(error.kind(), ParseErrorKind::RuleFailed);
    }
}
