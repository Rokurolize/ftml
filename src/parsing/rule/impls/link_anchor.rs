/*
 * parsing/rule/impls/link_anchor.rs
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

//! Rule for links to anchors on the same document.
//!
//! A variant on single-bracket links which targets an anchor
//! on the current page, or is a fake link.

use super::prelude::*;
use crate::id_prefix::isolate_ids;
use crate::parsing::ParseSuccess;
use crate::tree::{LinkLabel, LinkLocation, LinkType};
use std::borrow::Cow;
use wikidot_normalize::normalize;

pub const RULE_LINK_ANCHOR: Rule = Rule {
    name: "link-anchor",
    position: LineRequirement::Any,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create a single-bracket anchor link");
    assert_step(parser, Token::LeftBracketAnchor)?;

    // Gather path for link
    let url_close = [ParseCondition::current(Token::Whitespace)];
    let url_invalid = [
        ParseCondition::current(Token::RightBracket),
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::current(Token::LineBreak),
    ];
    let url = collect_text(parser, RULE_LINK_ANCHOR, &url_close, &url_invalid, None)?;

    // Determine if this is an anchor link or fake link
    let url = if url.is_empty() {
        Cow::Borrowed("javascript:;")
    } else {
        // Make URL "#name", where 'name' is normalized.
        let mut url = str!(url);
        normalize(&mut url);
        if parser.settings().isolate_user_ids {
            url = isolate_ids(&url);
        }
        url.insert(0, '#');

        Cow::Owned(url)
    };

    // Gather label for link
    let label_close = [ParseCondition::current(Token::RightBracket)];
    let label_invalid = [
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::current(Token::LineBreak),
    ];
    let label = collect_anchor_text(parser, &label_close, &label_invalid)?;

    trace!("Retrieved label ('{label}') for link, building element");

    // Trim label
    let label = label.trim();

    // Build and return link element
    let element = Element::Link {
        ltype: LinkType::Anchor,
        link: LinkLocation::Url(url),
        label: LinkLabel::Text(cow!(label)),
        target: None,
    };
    let elements: Elements = element.into();
    let paragraph_safe = elements.paragraph_safe();
    Ok(ParseSuccess::new(elements, Vec::new(), paragraph_safe))
}

fn collect_anchor_text<'r, 't>(
    parser: &mut Parser<'r, 't>,
    close: &[ParseCondition],
    invalid: &[ParseCondition],
) -> Result<&'t str, ParseError>
where
    'r: 't,
{
    collect_text(parser, RULE_LINK_ANCHOR, close, invalid, None)
}
