/*
 * parsing/rule/impls/strikethrough.rs
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

//! Rules for strikethrough.
//!
//! Wikidot had implemented strikethrough using --text--
//! however we also added the more conventional way ~~text~~

use super::prelude::*;

pub const RULE_STRIKETHROUGH_DASH: Rule = Rule {
    name: "strikethrough-dash",
    position: LineRequirement::Any,
    try_consume_fn: dash,
};

pub const RULE_STRIKETHROUGH_TILDE: Rule = Rule {
    name: "strikethrough-tilde",
    position: LineRequirement::Any,
    try_consume_fn: tilde,
};

fn dash<'r, 't>(parser: &mut Parser<'r, 't>) -> ParseResult<'r, 't, Elements<'t>> {
    trace!("Trying to create a double dash strikethrough");
    try_consume_strikethrough(parser, RULE_STRIKETHROUGH_DASH, Token::DoubleDash)
}

fn tilde<'r, 't>(parser: &mut Parser<'r, 't>) -> ParseResult<'r, 't, Elements<'t>> {
    trace!("Trying to create a double tilde strikethrough");
    try_consume_strikethrough(parser, RULE_STRIKETHROUGH_TILDE, Token::DoubleTilde)
}

/// Build a strikethrough with the given rule and token.
fn try_consume_strikethrough<'r, 't>(
    parser: &mut Parser<'r, 't>,
    rule: Rule,
    token: Token,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create a strikethrough (token {})", token.name());
    validate_reachable_close(parser, token)?;
    assert_step(parser, token)?;
    let close = [ParseCondition::current(token)];
    let invalid = [
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::token_pair(token, Token::Whitespace),
        ParseCondition::token_pair(Token::Whitespace, token),
    ];
    let ctype = ContainerType::Strikethrough;
    collect_container(parser, rule, ctype, &close, &invalid, None)
}

/// Check that collection can reach a closing delimiter.
///
/// Without this cheap look-ahead, every prose double dash can recursively
/// retry all later block and inline rules before eventually discovering that
/// no closing dash exists. Large advanced tables amplify that backtracking
/// into minutes of CPU time.
fn validate_reachable_close(
    parser: &Parser<'_, '_>,
    token: Token,
) -> Result<(), ParseError> {
    let mut scan = parser.clone();
    scan.step()?;
    let mut previous = parser.clone();
    let mut paragraph_error = None;
    loop {
        match scan.current().token {
            Token::InputEnd => {
                return Err(paragraph_error
                    .unwrap_or_else(|| scan.make_err(ParseErrorKind::EndOfInput)));
            }
            Token::ParagraphBreak => {
                paragraph_error
                    .get_or_insert_with(|| scan.make_err(ParseErrorKind::RuleFailed));
            }
            current if current == token => {
                if previous.current().token == Token::Whitespace {
                    return Err(previous.make_err(ParseErrorKind::RuleFailed));
                }
                return Ok(());
            }
            _ => {}
        }
        previous = scan.clone();
        scan.step()?;
    }
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender, text::TextRender};
    use crate::settings::{WikitextMode, WikitextSettings};

    fn render_text(input: &str) -> String {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        assert!(errors.is_empty(), "{errors:?}");
        TextRender.render(&tree, &page_info, &settings)
    }

    fn render_html(input: &str) -> String {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        assert!(errors.is_empty(), "{errors:?}");
        HtmlRender.render(&tree, &page_info, &settings).body
    }

    #[test]
    fn paired_double_dashes_still_render_as_strikethrough() {
        assert_eq!(
            render_text("before --removed-- after"),
            "before removed after"
        );
    }

    #[test]
    fn dense_table_prose_dashes_remain_literal_without_recursive_backtracking() {
        let row = "[[row]]\n[[cell]]\nterm -- explanatory prose\n[[/cell]]\n[[/row]]\n";
        let input = format!("[[table]]\n{}[[/table]]", row.repeat(200));
        let html = render_html(&input);

        assert_eq!(html.matches("term").count(), 200);
        assert_eq!(html.matches("explanatory prose").count(), 200);
        assert!(!html.contains("<s>"), "{html}");
    }
}
