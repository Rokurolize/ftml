/*
 * parsing/rule/impls/link_single.rs
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

//! Rules for single-bracket links.
//!
//! Wikidot, in its infinite wisdom, has two means for designating links.
//! This method allows any URL, either opening in a new tab or not.
//! Its syntax is `[https://example.com/ Label text]`.

use super::prelude::*;
use crate::tree::{AnchorTarget, LinkLabel, LinkLocation, LinkType};
use crate::url::is_url;

pub const RULE_LINK_SINGLE: Rule = Rule {
    name: "link-single",
    position: LineRequirement::Any,
    try_consume_fn: link,
};

pub const RULE_LINK_SINGLE_NEW_TAB: Rule = Rule {
    name: "link-single-new-tab",
    position: LineRequirement::Any,
    try_consume_fn: link_new_tab,
};

fn link<'r, 't>(parser: &mut Parser<'r, 't>) -> ParseResult<'r, 't, Elements<'t>> {
    assert_step(parser, Token::LeftBracket)?;
    try_consume_link(parser, RULE_LINK_SINGLE, None)
}

fn link_new_tab<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    assert_step(parser, Token::LeftBracketStar)?;
    try_consume_link(parser, RULE_LINK_SINGLE_NEW_TAB, Some(AnchorTarget::NewTab))
}

/// Build a single-bracket link with the given target.
fn try_consume_link<'r, 't>(
    parser: &mut Parser<'r, 't>,
    rule: Rule,
    target: Option<AnchorTarget>,
) -> ParseResult<'r, 't, Elements<'t>> {
    // Gather path for link
    let url_close = [ParseCondition::current(Token::Whitespace)];
    let url_invalid = [
        ParseCondition::current(Token::RightBracket),
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::current(Token::LineBreak),
    ];
    let url = collect_text(parser, rule, &url_close, &url_invalid, None)?;

    let wikidot_javascript_fallback =
        parser.settings().layout.legacy() && is_javascript_url(url);
    if !url_valid(url) && !wikidot_javascript_fallback {
        return Err(parser.make_err(ParseErrorKind::InvalidUrl));
    }

    // Gather label for link
    let label_close = [ParseCondition::current(Token::RightBracket)];
    let label_invalid = [
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::current(Token::LineBreak),
    ];
    let label = collect_text(parser, rule, &label_close, &label_invalid, None)?;

    // Trim label
    let label = label.trim();

    let element = if wikidot_javascript_fallback {
        Element::Text(cow!(label))
    } else {
        Element::Link {
            ltype: LinkType::Direct,
            link: LinkLocation::Url(cow!(url)),
            label: LinkLabel::Text(cow!(label)),
            target,
        }
    };

    // Return result
    success_elements(element)
}

fn is_javascript_url(url: &str) -> bool {
    url.trim_start()
        .get(.."javascript:".len())
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("javascript:"))
}

fn url_valid(url: &str) -> bool {
    // If url is an empty string
    if url.is_empty() {
        return false;
    }

    // If it's a relative link
    if url.starts_with('/') {
        return true;
    }

    // If it's a URL
    if is_url(url) {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::render::html::HtmlRender;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn wikidot_layout_does_not_weaken_dangerous_single_link_sanitization() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization =
            crate::tokenize("[javascript:alert(1) XSS] [data:text/html,alert(2) XSS]");
        let (tree, _errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(!html.contains("[javascript:alert(1) XSS]"), "{html}");
        assert!(html.contains("XSS"), "{html}");
        assert!(html.contains("[data:text/html,alert(2) XSS]"), "{html}");
        assert!(!html.contains("href=\"javascript:"), "{html}");
        assert!(!html.contains("href=\"data:"), "{html}");
    }
}
