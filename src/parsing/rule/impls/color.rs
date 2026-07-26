/*
 * parsing/rule/impls/color.rs
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
use crate::parsing::ParseSuccess;
use crate::tree::Container;
use regex::Regex;
use std::borrow::Cow;
use std::sync::LazyLock;

static HEX_COLOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([a-fA-F0-9]{3}|[a-fA-F0-9]{6})$").unwrap());

pub const RULE_COLOR: Rule = Rule {
    name: "color",
    position: LineRequirement::Any,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create color container");
    assert_step(parser, Token::Color)?;

    // The pattern for color is:
    // ## [color-style] | [text to be colored] ##

    // Gather the color name until the separator
    let color_close = [ParseCondition::current(Token::Pipe)];
    let color_invalid = [
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::current(Token::LineBreak),
    ];
    let color = collect_text(parser, RULE_COLOR, &color_close, &color_invalid, None)?;
    if parser.settings().layout.legacy() && (color.is_empty() || !is_safe_color(color)) {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    trace!("Retrieved color descriptor, now building container ('{color}')");

    if parser.settings().layout.legacy() && has_crossed_bold_close(parser) {
        return collect_crossed_bold_color(parser, color);
    }

    // Build color container
    let close = [ParseCondition::current(Token::Color)];
    let invalid = [ParseCondition::current(Token::ParagraphBreak)];
    let body = collect_consume(parser, RULE_COLOR, &close, &invalid, None)?;
    let (elements, errors, paragraph_safe) = body.into();

    // Return result
    let element = Element::Color {
        color: normalize_color(color),
        elements,
    };

    ok!(paragraph_safe; element, errors)
}

fn has_crossed_bold_close(parser: &Parser<'_, '_>) -> bool {
    if parser.current().token != Token::Bold {
        return false;
    }

    let tokens = parser.remaining();
    let Some(color_close) = tokens.iter().position(|token| token.token == Token::Color)
    else {
        return false;
    };
    if tokens[..color_close]
        .iter()
        .any(|token| matches!(token.token, Token::Bold | Token::ParagraphBreak))
    {
        return false;
    }

    for token in &tokens[color_close + 1..] {
        match token.token {
            Token::Bold => return true,
            Token::ParagraphBreak | Token::InputEnd => return false,
            _ => {}
        }
    }
    false
}

fn collect_crossed_bold_color<'r, 't>(
    parser: &mut Parser<'r, 't>,
    color: &'t str,
) -> ParseResult<'r, 't, Elements<'t>> {
    assert_step(parser, Token::Bold)?;

    let invalid = [ParseCondition::current(Token::ParagraphBreak)];
    let colored = collect_consume(
        parser,
        RULE_COLOR,
        &[ParseCondition::current(Token::Color)],
        &invalid,
        None,
    )?;
    let trailing = collect_consume(
        parser,
        RULE_COLOR,
        &[ParseCondition::current(Token::Bold)],
        &invalid,
        None,
    )?;
    let (colored, mut errors, colored_safe) = colored.into();
    let (trailing, mut trailing_errors, trailing_safe) = trailing.into();
    errors.append(&mut trailing_errors);

    let colored = Element::Container(Container::new(
        ContainerType::Bold,
        colored,
        AttributeMap::new(),
    ));
    let color = Element::Color {
        color: normalize_color(color),
        elements: vec![colored],
    };
    let trailing = Element::Container(Container::new(
        ContainerType::Bold,
        trailing,
        AttributeMap::new(),
    ));

    Ok(ParseSuccess::new(
        vec![color, trailing].into(),
        errors,
        colored_safe && trailing_safe,
    ))
}

/// Prefix with `#`, if needed.
///
/// Normally we pass the color as-is, such as `blue` or `rgb(10, 12, 14)`,
/// but if a hex specification is passed, and it doesn't already begin with
/// `#`, then one should be prepended.
fn normalize_color(color: &str) -> Cow<'_, str> {
    if !is_safe_color(color) {
        return Cow::Borrowed("inherit");
    }

    if HEX_COLOR.is_match(color) {
        Cow::Owned(format!("#{color}"))
    } else {
        Cow::Borrowed(color)
    }
}

fn is_safe_color(color: &str) -> bool {
    color.chars().all(|ch| {
        !ch.is_control()
            && !matches!(ch, ';' | '{' | '}' | '<' | '>' | '"' | '\'' | '\\' | '&')
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};

    fn render_wikidot(source: &str) -> (String, Vec<ParseError>) {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        (HtmlRender.render(&tree, &page_info, &settings).body, errors)
    }

    #[test]
    fn color_normalization_rejects_css_declaration_breakout() {
        assert_eq!(normalize_color("abc"), "#abc");
        assert_eq!(normalize_color("").as_ref(), "");
        assert_eq!(normalize_color("red").as_ref(), "red");
        assert_eq!(
            normalize_color("rgb(10, 12, 14)").as_ref(),
            "rgb(10, 12, 14)"
        );
        assert_eq!(
            normalize_color("red;background:url(//x)").as_ref(),
            "inherit"
        );
        assert_eq!(
            normalize_color("red\nbackground:url(//x)").as_ref(),
            "inherit"
        );
        assert_eq!(
            normalize_color("red&#59background:url(//x)").as_ref(),
            "inherit"
        );
        assert_eq!(
            normalize_color("red&#x3bbackground:url(//x)").as_ref(),
            "inherit"
        );
    }

    #[test]
    fn wikidot_crossed_bold_color_closes_and_reopens_bold() {
        let (html, errors) = render_wikidot("##orange|**ORANGE##-PRIME:**");

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            "<p><span style=\"color: orange\"><strong>ORANGE</strong></span><strong>-PRIME:</strong></p>",
        );
    }

    #[test]
    fn ordinary_bold_inside_color_is_unchanged() {
        let (html, errors) = render_wikidot("##orange|before **bold** after##");

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            "<p><span style=\"color: orange\">before <strong>bold</strong> after</span></p>",
        );
    }
}
