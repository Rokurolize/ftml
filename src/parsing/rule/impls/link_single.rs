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

use super::RULE_COMMENT;
use super::prelude::*;
use crate::delayed::{DelayedElement, GeneratedKind};
use crate::tree::{AnchorTarget, LinkLabel, LinkLocation, LinkType};
use crate::url::is_url;
use std::borrow::Cow;

const MAX_WIKIDOT_SINGLE_LINK_TARGET_BYTES: usize = 8 * 1024;
const MAX_WIKIDOT_SINGLE_LINK_SCHEME_BYTES: usize = 64;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum WikidotLinkDisposition {
    Link,
    LabelOnly,
    Literal,
}

#[derive(Debug)]
struct WikidotLabel<'t> {
    text: Cow<'t, str>,
    residual_closer: &'t str,
}

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
    if parser.settings().layout.legacy() {
        return try_consume_wikidot_link(parser, target);
    }

    // Gather path for link
    let url_close = [ParseCondition::current(Token::Whitespace)];
    let url_invalid = [
        ParseCondition::current(Token::RightBracket),
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::current(Token::LineBreak),
    ];
    let url = collect_text(parser, rule, &url_close, &url_invalid, None)?;

    if !url_valid(url) {
        return Err(parser.make_err(ParseErrorKind::InvalidUrl));
    }

    if target.is_none()
        && let Some(generated) = parser.current_generated().cloned()
        && generated.kind == GeneratedKind::TagLinks
    {
        parser.step()?;
        if parser.current().token != Token::RightBracket {
            return Err(parser.make_err(ParseErrorKind::RuleFailed));
        }
        parser.step()?;
        return success_elements(Element::Delayed(DelayedElement::tag_external_label(
            generated.id,
            url,
        )));
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

    let element = Element::Link {
        ltype: LinkType::Direct,
        link: LinkLocation::Url(cow!(url)),
        label: LinkLabel::Text(cow!(label)),
        target,
    };

    // Return result
    success_elements(element)
}

fn try_consume_wikidot_link<'r, 't>(
    parser: &mut Parser<'r, 't>,
    target: Option<AnchorTarget>,
) -> ParseResult<'r, 't, Elements<'t>>
where
    'r: 't,
{
    let url = collect_wikidot_target(parser)?;
    let disposition = wikidot_link_disposition(&url);
    if disposition == WikidotLinkDisposition::Literal {
        return Err(parser.make_err(ParseErrorKind::InvalidUrl));
    }

    if target.is_none()
        && let Cow::Borrowed(url) = &url
        && let Some(generated) = parser.current_generated().cloned()
        && generated.kind == GeneratedKind::TagLinks
    {
        parser.step()?;
        if parser.current().token != Token::RightBracket {
            return Err(parser.make_err(ParseErrorKind::RuleFailed));
        }
        parser.step()?;
        return success_elements(Element::Delayed(DelayedElement::tag_external_label(
            generated.id,
            url,
        )));
    }

    let WikidotLabel {
        text: label,
        residual_closer,
    } = collect_wikidot_label(parser)?;
    let label = trim_cow(label);
    if label.is_empty() {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let element = match disposition {
        WikidotLinkDisposition::Link => Element::Link {
            ltype: LinkType::Direct,
            link: LinkLocation::Url(url),
            label: LinkLabel::Text(label),
            target,
        },
        WikidotLinkDisposition::LabelOnly => Element::Text(label),
        WikidotLinkDisposition::Literal => unreachable!(),
    };

    if residual_closer.is_empty() {
        success_elements(element)
    } else {
        success_elements(vec![element, text!(residual_closer)])
    }
}

fn collect_wikidot_target<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<Cow<'t, str>, ParseError>
where
    'r: 't,
{
    let source = parser.full_text().inner();
    let start = parser.current().span.start;
    let mut copied_until = start;
    let mut without_comments = None::<String>;
    loop {
        if parser.current().span.start - start > MAX_WIKIDOT_SINGLE_LINK_TARGET_BYTES {
            return Err(parser.make_err(ParseErrorKind::InvalidUrl));
        }
        match parser.current().token {
            Token::Whitespace => {
                let end = parser.current().span.start;
                if let Some(output) = without_comments.as_mut() {
                    output.push_str(&source[copied_until..end]);
                }
                parser.step()?;
                let target = match without_comments {
                    Some(output) => Cow::Owned(output),
                    None => Cow::Borrowed(&source[start..end]),
                };
                if target.len() > MAX_WIKIDOT_SINGLE_LINK_TARGET_BYTES {
                    return Err(parser.make_err(ParseErrorKind::InvalidUrl));
                }
                return Ok(target);
            }
            Token::LeftComment => {
                let comment_start = parser.current().span.start;
                let output = without_comments
                    .get_or_insert_with(|| String::with_capacity(comment_start - start));
                output.push_str(&source[copied_until..comment_start]);

                let mut comment = parser.clone_with_rule(RULE_COMMENT);
                let _comment = RULE_COMMENT.try_consume(&mut comment)?;
                parser.update(&comment);
                copied_until = parser.current().span.start;
            }
            Token::LineBreak | Token::ParagraphBreak => {
                return Err(parser.make_err(ParseErrorKind::RuleFailed));
            }
            Token::InputEnd => return Err(parser.make_err(ParseErrorKind::EndOfInput)),
            Token::GeneratedPageLink | Token::GeneratedTagLinks => {
                return Err(parser.make_err(ParseErrorKind::RuleFailed));
            }
            _ => {
                parser.step()?;
            }
        }
    }
}

fn collect_wikidot_label<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<WikidotLabel<'t>, ParseError>
where
    'r: 't,
{
    if wikidot_label_has_complete_owner(parser) {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let source = parser.full_text().inner();
    let start = parser.current().span.start;
    let mut copied_until = start;
    let mut without_comments = None::<String>;

    loop {
        match parser.current().token {
            Token::RightBracket | Token::RightBlock | Token::RightLink => {
                let end = parser.current().span.start;
                if let Some(output) = without_comments.as_mut() {
                    output.push_str(&source[copied_until..end]);
                }
                let residual_closer = &parser.current().slice[1..];
                parser.step()?;
                return Ok(WikidotLabel {
                    text: match without_comments {
                        Some(output) => Cow::Owned(output),
                        None => Cow::Borrowed(&source[start..end]),
                    },
                    residual_closer,
                });
            }
            Token::LeftComment => {
                let comment_start = parser.current().span.start;
                let output = without_comments
                    .get_or_insert_with(|| String::with_capacity(comment_start - start));
                output.push_str(&source[copied_until..comment_start]);

                let mut comment = parser.clone_with_rule(RULE_COMMENT);
                let _comment = RULE_COMMENT.try_consume(&mut comment)?;
                parser.update(&comment);
                copied_until = parser.current().span.start;
            }
            Token::LineBreak | Token::ParagraphBreak => {
                return Err(parser.make_err(ParseErrorKind::RuleFailed));
            }
            Token::InputEnd => return Err(parser.make_err(ParseErrorKind::EndOfInput)),
            Token::GeneratedPageLink | Token::GeneratedTagLinks => {
                return Err(parser.make_err(ParseErrorKind::RuleFailed));
            }
            _ => {
                parser.step()?;
            }
        }
    }
}

fn wikidot_label_has_complete_owner<'r, 't>(parser: &Parser<'r, 't>) -> bool
where
    'r: 't,
{
    let mut scan = parser.clone();
    let mut raw_open = false;
    let mut alternate_raw_open = false;
    let mut triple_link_open = false;
    let mut span_open = false;
    let mut footnote_open = false;
    loop {
        match scan.current().token {
            Token::Raw if raw_open => return true,
            Token::Raw => raw_open = true,
            Token::LeftRaw => alternate_raw_open = true,
            Token::RightRaw if alternate_raw_open => return true,
            Token::LeftLink | Token::LeftLinkStar => triple_link_open = true,
            Token::RightLink if triple_link_open => return true,
            Token::LeftBlock => {
                let mut opener = scan.clone();
                if let Ok((name, _)) = opener.get_block_name(false) {
                    span_open |= name.eq_ignore_ascii_case("span");
                    footnote_open |= name.eq_ignore_ascii_case("footnote");
                    scan.update(&opener);
                    continue;
                }
            }
            Token::LeftBlockEnd => {
                let mut close = scan.clone();
                if let Ok((name, residual)) = close.get_wikidot_end_block_with_residual()
                {
                    if span_open && name.eq_ignore_ascii_case("span") {
                        return true;
                    }
                    if footnote_open && !residual && name.eq_ignore_ascii_case("footnote")
                    {
                        return true;
                    }
                    scan.update(&close);
                    continue;
                }
            }
            Token::RightBracket
            | Token::RightBlock
            | Token::RightLink
            | Token::LineBreak
            | Token::ParagraphBreak
            | Token::InputEnd => {
                return false;
            }
            _ => {}
        }
        if scan.step().is_err() {
            return false;
        }
    }
}

fn trim_cow(value: Cow<'_, str>) -> Cow<'_, str> {
    match value {
        Cow::Borrowed(value) => Cow::Borrowed(value.trim()),
        Cow::Owned(value) => Cow::Owned(value.trim().to_owned()),
    }
}

fn wikidot_link_disposition(url: &str) -> WikidotLinkDisposition {
    if url.is_empty() || url.len() > MAX_WIKIDOT_SINGLE_LINK_TARGET_BYTES {
        return WikidotLinkDisposition::Literal;
    }
    if url.starts_with('/')
        || url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("ftp://")
        || url.starts_with("mailto:")
    {
        return WikidotLinkDisposition::Link;
    }

    let Some((scheme, _)) = url.split_once(':') else {
        return WikidotLinkDisposition::Literal;
    };
    if !valid_bounded_scheme(scheme) || scheme.eq_ignore_ascii_case("data") {
        return WikidotLinkDisposition::Literal;
    }

    WikidotLinkDisposition::LabelOnly
}

fn valid_bounded_scheme(scheme: &str) -> bool {
    if scheme.is_empty() || scheme.len() > MAX_WIKIDOT_SINGLE_LINK_SCHEME_BYTES {
        return false;
    }
    let mut bytes = scheme.bytes();
    bytes.next().is_some_and(|byte| byte.is_ascii_alphabetic())
        && bytes.all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.')
        })
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
    use super::{
        MAX_WIKIDOT_SINGLE_LINK_SCHEME_BYTES, WikidotLinkDisposition,
        valid_bounded_scheme, wikidot_link_disposition,
    };
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::render::html::HtmlRender;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn wikidot_single_link_disposition_separates_compatibility_and_security() {
        for url in [
            "http://example.com",
            "https://example.com",
            "ftp://example.com/a",
            "mailto:user@example.com",
            "/start",
            "//example.com/a",
        ] {
            assert_eq!(
                wikidot_link_disposition(url),
                WikidotLinkDisposition::Link,
                "{url}",
            );
        }
        for url in [
            "HTTPS://example.com",
            "HtTpS://example.com",
            "https:example.com",
            "javascript:void(0)",
            "foo:bar",
        ] {
            assert_eq!(
                wikidot_link_disposition(url),
                WikidotLinkDisposition::LabelOnly,
                "{url}",
            );
        }
        for url in ["", "example.com", "data:text/plain,x", "DATA:text/html,x"] {
            assert_eq!(
                wikidot_link_disposition(url),
                WikidotLinkDisposition::Literal,
                "{url}",
            );
        }
    }

    #[test]
    fn wikidot_single_link_scheme_scan_has_an_explicit_bound() {
        assert!(valid_bounded_scheme("web+demo"));
        assert!(valid_bounded_scheme(
            &"a".repeat(MAX_WIKIDOT_SINGLE_LINK_SCHEME_BYTES),
        ));
        assert!(!valid_bounded_scheme(
            &"a".repeat(MAX_WIKIDOT_SINGLE_LINK_SCHEME_BYTES + 1),
        ));
        assert!(!valid_bounded_scheme("1http"));
        assert!(!valid_bounded_scheme("java_script"));
    }

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
