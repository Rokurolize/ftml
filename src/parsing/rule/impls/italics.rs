/*
 * parsing/rule/impls/italics.rs
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

use super::inline_delimiter::assert_unpadded_open;
use super::prelude::*;
use crate::parsing::ParseSuccess;
use crate::tree::Container;

pub const RULE_ITALICS: Rule = Rule {
    name: "italics",
    position: LineRequirement::Any,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create italics (emphasis) container");

    // Wikidot canonicalizes the real-world crossed pair
    // `//**text//**` as bold containing italics. Handle only the unambiguous
    // single-pair shape here; ordinary nested formatting continues through
    // the standard collector below.
    if has_crossed_bold_pair(parser) {
        return collect_crossed_bold_italics(parser);
    }

    assert_unpadded_open(parser, Token::Italics)?;
    let close = [ParseCondition::current(Token::Italics)];
    let invalid = [
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::token_pair(Token::Italics, Token::Whitespace),
        ParseCondition::token_pair(Token::Whitespace, Token::Italics),
    ];
    let ctype = ContainerType::Italics;
    collect_container(parser, RULE_ITALICS, ctype, &close, &invalid, None)
}

fn has_crossed_bold_pair(parser: &Parser<'_, '_>) -> bool {
    if !matches!(
        parser.next_two_tokens(),
        (Token::Italics, Some(Token::Bold))
    ) {
        return false;
    }

    let tokens = parser.remaining();
    for (index, token) in tokens.iter().enumerate().skip(1) {
        match token.token {
            Token::Italics => {
                return tokens
                    .get(index + 1)
                    .is_some_and(|next| next.token == Token::Bold);
            }
            Token::Bold | Token::ParagraphBreak | Token::InputEnd => return false,
            _ => {}
        }
    }

    false
}

fn collect_crossed_bold_italics<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    assert_step(parser, Token::Italics)?;
    assert_step(parser, Token::Bold)?;

    let close = [ParseCondition::token_pair(Token::Italics, Token::Bold)];
    let invalid = [ParseCondition::current(Token::ParagraphBreak)];
    let inner = collect_container(
        parser,
        RULE_ITALICS,
        ContainerType::Italics,
        &close,
        &invalid,
        None,
    )?;
    assert_step(parser, Token::Bold)?;

    let (inner, errors, paragraph_safe) = inner.into();
    let elements = inner.into_iter().collect();
    let outer = Element::Container(Container::new(
        ContainerType::Bold,
        elements,
        AttributeMap::new(),
    ));

    Ok(ParseSuccess::new(outer.into(), errors, paragraph_safe))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};

    fn render(source: &str) -> (String, Vec<ParseError>) {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        (html, errors)
    }

    #[test]
    fn wikidot_crossed_bold_italics_pair_is_canonicalized() {
        let (html, errors) = render("//**You liked this//**");

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(
            html.contains("<strong><em>You liked this</em></strong>"),
            "{html}",
        );
    }

    #[test]
    fn ordinary_nested_bold_inside_italics_is_unchanged() {
        let (html, errors) = render("//before **bold** after//");

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(
            html.contains("<em>before <strong>bold</strong> after</em>"),
            "{html}",
        );
    }

    #[test]
    fn ambiguous_crossed_delimiters_are_not_rewritten() {
        let (_, errors) = render("//**one **two//**");

        assert!(!errors.is_empty());
    }
}
