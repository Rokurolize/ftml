/*
 * parsing/rule/impls/url.rs
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
use crate::tree::{LinkLabel, LinkLocation, LinkType};

pub const RULE_URL: Rule = Rule {
    name: "url",
    position: LineRequirement::Any,
    try_consume_fn,
};

pub(crate) fn url_elements<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<Elements<'t>, ParseError> {
    let token = parser.current();
    let source = parser.full_text().inner();
    let start = token.span.start;
    let mut end = token.span.end;
    let mut extension_tokens = 0;

    if parser.settings().layout.legacy() {
        end = wikidot_automatic_url_end(source, end);
        extension_tokens = parser
            .remaining()
            .iter()
            .take_while(|token| token.token != Token::InputEnd && token.span.end <= end)
            .count();
    }

    let mut suffix = None;
    if parser.settings().layout.legacy() {
        let previous = start
            .checked_sub(1)
            .and_then(|index| source.as_bytes().get(index));
        let last = end
            .checked_sub(1)
            .and_then(|index| source.as_bytes().get(index));
        if matches!(
            (previous, last),
            (Some(b'('), Some(b')')) | (Some(b'['), Some(b']'))
        ) {
            suffix = source.get(end - 1..end);
            end -= 1;
        }
    }

    let url = &source[start..end];
    let split_terminal_period = parser.settings().layout.legacy()
        && url.ends_with('.')
        && matches!(
            parser.look_ahead(0).map(|next| next.token),
            Some(Token::Whitespace | Token::InputEnd)
        )
        || parser.settings().layout.legacy() && url.ends_with('.') && suffix.is_some();
    let url = if split_terminal_period {
        &url[..url.len() - 1]
    } else {
        url
    };
    let link = Element::Link {
        ltype: LinkType::Direct,
        link: LinkLocation::Url(cow!(url)),
        label: LinkLabel::Url,
        target: None,
    };

    parser.step_n(extension_tokens + 1)?;

    let mut elements = vec![link];
    if split_terminal_period {
        elements.push(text!("."));
    }
    if let Some(suffix) = suffix {
        elements.push(text!(suffix));
    }

    Ok(if elements.len() == 1 {
        Elements::Single(elements.pop().unwrap())
    } else {
        Elements::Multiple(elements)
    })
}

fn wikidot_automatic_url_end(source: &str, mut end: usize) -> usize {
    let bytes = source.as_bytes();
    while end < bytes.len() {
        if matches!(
            bytes[end],
            b'\n' | b'\r' | b' ' | b'\t' | b'"' | b'\'' | b'['
        ) || matches!(bytes[end], 0x00..=0x08 | 0x0b..=0x0c | 0x0e..=0x1a | 0x1c..=0x1f)
            || source[end..].starts_with(">@")
            || source[end..].starts_with("@@")
            || source[end..].starts_with("]]")
            || source[end..].starts_with("||")
        {
            break;
        }
        end += source[end..].chars().next().unwrap().len_utf8();
    }
    end
}

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Consuming token as a URL");
    ok!(url_elements(parser)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    fn assert_url_rule(source: &str, layout: Layout, expected: Elements<'static>) {
        let tokenization = crate::tokenize(source);
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser.step().expect("URL token should follow input start");
        let actual = RULE_URL
            .try_consume(&mut parser)
            .expect("URL rule should consume the URL")
            .item;
        assert_eq!(actual, expected);
    }

    fn link(url: &'static str) -> Element<'static> {
        Element::Link {
            ltype: LinkType::Direct,
            link: LinkLocation::Url(cow!(url)),
            label: LinkLabel::Url,
            target: None,
        }
    }

    #[test]
    fn wikidot_url_rule_splits_terminal_period_at_text_boundaries() {
        let expected =
            Elements::Multiple(vec![link("https://example.com/test"), text!(".")]);
        assert_url_rule(
            "https://example.com/test. next",
            Layout::Wikidot,
            expected.clone(),
        );
        assert_url_rule("https://example.com/test.", Layout::Wikidot, expected);
    }

    #[test]
    fn url_rule_keeps_other_periods_and_wikijump_behavior() {
        assert_url_rule(
            "https://example.com/a.b/c next",
            Layout::Wikidot,
            link("https://example.com/a.b/c").into(),
        );
        assert_url_rule(
            "https://example.com/test. next",
            Layout::Wikijump,
            link("https://example.com/test.").into(),
        );
    }

    #[test]
    fn wikidot_url_rule_extends_live_punctuation_and_stops_at_block_syntax() {
        assert_url_rule(
            "http://example.com/path|END next",
            Layout::Wikidot,
            link("http://example.com/path|END").into(),
        );
        assert_url_rule(
            "mailto:User@example.com|END next",
            Layout::Wikidot,
            link("mailto:User@example.com|END").into(),
        );

        let source = "https://example.com/video[[/embedvideo]]";
        let tokenization = crate::tokenize(source);
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser.step().expect("URL token should follow input start");
        let actual = RULE_URL
            .try_consume(&mut parser)
            .expect("URL rule should stop before block syntax")
            .item;

        assert_eq!(actual, link("https://example.com/video").into());
        assert_eq!(parser.current().token, Token::LeftBlockEnd);
    }
}
