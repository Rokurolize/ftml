/*
 * parsing/rule/impls/horizontal_rule.rs
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

pub const RULE_HORIZONTAL_RULE: Rule = Rule {
    name: "horizontal-rule",
    position: LineRequirement::StartOfLine,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Consuming token to create a horizontal rule");
    let source = parser.full_text().inner().as_bytes();
    let start = parser.current().span.start;
    let physical_line_start =
        start == 0 || matches!(source.get(start - 1), Some(b'\n' | b'\r'));
    if (!physical_line_start && !parser.in_native_blockquote_line())
        || !matches!(
            parser.look_ahead(0).map(|token| token.token),
            Some(Token::LineBreak | Token::ParagraphBreak | Token::InputEnd)
        )
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    assert_step(parser, Token::TripleDash)?;
    parser.get_optional_line_break()?;
    ok!(Element::HorizontalRule)
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};

    fn render_html(input: &str) -> String {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        assert!(errors.is_empty(), "{input:?}: {errors:?}");
        HtmlRender.render(&tree, &page_info, &settings).body
    }

    #[test]
    fn wikidot_horizontal_rule_requires_an_immediate_line_boundary() {
        assert_eq!(render_html("----"), "<hr>");
        assert_eq!(render_html("----\nnext"), "<hr><p>next</p>");
        assert_eq!(render_html("---- tail"), "<p>—— tail</p>");
        assert_eq!(render_html("----   "), "<p>——</p>");
        assert_eq!(render_html("---- \nnext"), "<p>——<br>\nnext</p>");
        assert_eq!(render_html(" ----"), "<p>——</p>");
    }
}
