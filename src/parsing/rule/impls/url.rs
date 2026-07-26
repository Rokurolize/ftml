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

pub(crate) fn url_elements<'r, 't>(parser: &Parser<'r, 't>) -> Elements<'t> {
    let token = parser.current();
    let split_terminal_period = parser.settings().layout.legacy()
        && token.slice.ends_with('.')
        && matches!(
            parser.look_ahead(0).map(|next| next.token),
            Some(Token::Whitespace | Token::InputEnd)
        );
    let url = if split_terminal_period {
        &token.slice[..token.slice.len() - 1]
    } else {
        token.slice
    };
    let link = Element::Link {
        ltype: LinkType::Direct,
        link: LinkLocation::Url(cow!(url)),
        label: LinkLabel::Url,
        target: None,
    };

    if split_terminal_period {
        Elements::Multiple(vec![link, text!(".")])
    } else {
        link.into()
    }
}

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Consuming token as a URL");
    success_elements(url_elements(parser))
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
}
