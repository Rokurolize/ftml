/*
 * parsing/rule/impls/link_triple.rs
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

//! Rules for triple-bracket links.
//!
//! This method of designating links is for local pages.
//! The syntax here uses a pipe to separate the destination from the label.
//! However, this method also works for regular URLs, for some reason.
//!
//! Wikidot, in its infinite wisdom, has two means for designating links.
//! This method allows any URL, either opening in a new tab or not.
//! Its syntax is `[[[page-name | Label text]`.

use super::prelude::*;
use crate::data::PageRef;
use crate::tree::{AnchorTarget, LinkLabel, LinkLocation, LinkType};
use crate::url::is_url;
use std::borrow::Cow;
use wikidot_normalize::normalize;

pub const RULE_LINK_TRIPLE: Rule = Rule {
    name: "link-triple",
    position: LineRequirement::Any,
    try_consume_fn: link,
};

pub const RULE_LINK_TRIPLE_NEW_TAB: Rule = Rule {
    name: "link-triple-new-tab",
    position: LineRequirement::Any,
    try_consume_fn: link_new_tab,
};

fn link<'r, 't>(parser: &mut Parser<'r, 't>) -> ParseResult<'r, 't, Elements<'t>> {
    assert_step(parser, Token::LeftLink)?;
    try_consume_link(parser, RULE_LINK_TRIPLE, None)
}

fn link_new_tab<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    assert_step(parser, Token::LeftLinkStar)?;
    try_consume_link(parser, RULE_LINK_TRIPLE_NEW_TAB, Some(AnchorTarget::NewTab))
}

/// Build a triple-bracket link with the given target.
fn try_consume_link<'r, 't>(
    parser: &mut Parser<'r, 't>,
    rule: Rule,
    target: Option<AnchorTarget>,
) -> ParseResult<'r, 't, Elements<'t>> {
    // Gather path for link
    let url_close = [
        ParseCondition::current(Token::Pipe),
        ParseCondition::current(Token::RightLink),
    ];
    let url_invalid = [ParseCondition::current(Token::ParagraphBreak)];
    let native_url_invalid = [
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::current(Token::LineBreak),
    ];
    let url_invalid = if parser.settings().layout.legacy() {
        &url_invalid[..]
    } else {
        &native_url_invalid[..]
    };
    let (url, last) = collect_text_keep(parser, rule, &url_close, url_invalid, None)?;

    // Trim text
    let trimmed_url = url.trim();
    let surrounded_by_space = trimmed_url.len() != url.len();
    let url = trimmed_url;

    // If url is an empty string, parsing should fail, there's nothing here
    if url.is_empty()
        && parser.settings().layout.legacy()
        && target.is_some()
        && last.token == Token::Pipe
    {
        return build_separate(parser, rule, "/", false, target);
    }
    if url.is_empty() {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    // Determine what token we ended on, i.e. which [[[ variant it is.
    match last.token {
        // [[[name]]] type links
        Token::RightLink => build_same(parser, url, surrounded_by_space, target),

        // [[[url|label]]] type links
        Token::Pipe => build_separate(parser, rule, url, surrounded_by_space, target),

        // Token was already checked in collect_text(), impossible case
        _ => unreachable!(),
    }
}

/// Helper to build link with the same URL and label.
/// e.g. `[[[name]]]`
fn build_same<'r, 't>(
    parser: &mut Parser<'r, 't>,
    url: &'t str,
    surrounded_by_space: bool,
    target: Option<AnchorTarget>,
) -> ParseResult<'r, 't, Elements<'t>> {
    // Remove category, if present.
    // If None, then the label is the original URL.
    let label = match strip_category(url) {
        Some(_)
            if parser.settings().layout.legacy()
                && url.starts_with(':')
                && (url.contains(" :") || url.contains(": ")) =>
        {
            cow!(url)
        }
        Some(stripped) => cow!(stripped),
        None => Cow::Borrowed(url),
    };
    let label = if parser.settings().layout.legacy()
        && target.is_some()
        && strip_category(url).is_none()
    {
        Cow::Owned(format!("*{label}"))
    } else {
        label
    };

    // Parse out link location
    let parsed_link = parse_link_location(parser, url, surrounded_by_space);
    let Some((link, ltype)) = parsed_link else {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    };

    // Build and return element
    let element = Element::Link {
        ltype,
        link,
        label: LinkLabel::Slug(label),
        target: wikidot_target(parser, target),
    };

    success_elements(element)
}

/// Helper to build link with separate URL and label.
/// e.g. `[[[page|label]]]`, or `[[[page|]]]`
fn build_separate<'r, 't>(
    parser: &mut Parser<'r, 't>,
    rule: Rule,
    url: &'t str,
    surrounded_by_space: bool,
    target: Option<AnchorTarget>,
) -> ParseResult<'r, 't, Elements<'t>> {
    // Gather label for link
    let label_close = [ParseCondition::current(Token::RightLink)];
    let legacy = parser.settings().layout.legacy();
    let label_invalid = [ParseCondition::current(Token::ParagraphBreak)];
    let native_label_invalid = [
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::current(Token::LineBreak),
    ];
    let label_invalid = if legacy {
        &label_invalid[..]
    } else {
        &native_label_invalid[..]
    };
    let label = collect_text(parser, rule, &label_close, label_invalid, None)?;

    // Trim label
    let label = label.trim();

    // Parse out link location
    let parsed_link = parse_link_location(parser, url, surrounded_by_space);
    let Some((link, ltype)) = parsed_link else {
        if legacy && (url.contains("###") || url.contains("/##/")) {
            return ok!(Element::Text(Cow::Owned(format!("[[[{url}|{label}]]]"))));
        }
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    };

    // Wikidot derives an empty external-link label from its normalized URL slug.
    let label = if label.is_empty() && legacy {
        let mut normalized = str!(url);
        if normalized.starts_with(':') {
            normalized.remove(0);
        }
        normalize(&mut normalized);
        LinkLabel::Text(Cow::Owned(normalized))
    } else if label.is_empty() {
        LinkLabel::Page
    } else {
        LinkLabel::Text(cow!(label))
    };

    // Build link element
    let element = Element::Link {
        ltype,
        link,
        label,
        target: wikidot_target(parser, target),
    };

    // Return result
    success_elements(element)
}

fn wikidot_target(
    parser: &Parser<'_, '_>,
    target: Option<AnchorTarget>,
) -> Option<AnchorTarget> {
    if parser.settings().layout.legacy() {
        None
    } else {
        target
    }
}

fn parse_link_location<'r, 't>(
    parser: &Parser<'r, 't>,
    url: &'t str,
    surrounded_by_space: bool,
) -> Option<(LinkLocation<'t>, LinkType)> {
    if !parser.settings().layout.legacy() {
        return LinkLocation::parse_with_interwiki(cow!(url), parser.settings());
    }

    if url.contains("###") || url.contains("/##/") {
        return None;
    }

    if let Some(interwiki) = url.strip_prefix('!') {
        if let Some(expanded) = parser.settings().interwiki.build(interwiki) {
            return Some((LinkLocation::Url(Cow::Owned(expanded)), LinkType::Interwiki));
        }
        let local = interwiki.strip_prefix(':').unwrap_or(interwiki);
        return Some((
            LinkLocation::Page(PageRef::page_only(local)),
            LinkType::Page,
        ));
    }

    if let Some(cross_site) = url.strip_prefix(':') {
        let mut page = cross_site
            .split(':')
            .map(|part| {
                let mut part = part.trim().to_owned();
                normalize(&mut part);
                part
            })
            .collect::<Vec<_>>()
            .join(":");
        if page.is_empty() {
            return None;
        }
        page.make_ascii_lowercase();
        return Some((
            LinkLocation::Page(PageRef {
                site: None,
                page,
                extra: None,
            }),
            LinkType::Page,
        ));
    }

    if surrounded_by_space && is_url(url) {
        let page = url.replace("://", ":");
        let page = page.trim_end_matches('/');
        return Some((LinkLocation::Page(PageRef::page_only(page)), LinkType::Page));
    }

    if let Some((page, extra)) = url.split_once("/#/") {
        let mut page_ref = PageRef::page_only(page);
        page_ref.extra = Some(format!("#/{extra}"));
        return Some((LinkLocation::Page(page_ref), LinkType::Page));
    }

    if url.contains('/') && !url.starts_with('/') && !is_url(url) {
        return Some((
            LinkLocation::Page(PageRef::page_only(url.replace('/', "-"))),
            LinkType::Page,
        ));
    }

    LinkLocation::parse_with_interwiki(cow!(url), parser.settings())
}

/// Strip off the category for use in URL triple-bracket links.
///
/// The label for a URL link is its URL, but without its category.
/// For instance, `theme: Sigma-9` becomes just `Sigma-9`.
///
/// It returns `Some(_)` if a slice was performed, and `None` if
/// the string would have been returned as-is.
fn strip_category(url: &str) -> Option<&str> {
    match url.find(':') {
        // Link with site, e.g. :scp-wiki:component:image-block.
        Some(0) => {
            let url = &url[1..];

            // If there is no colon, it's malformed, return None.
            // Else, return a stripped version
            url.find(':').map(|idx| {
                let url = url[idx + 1..].trim_start();

                // Skip past the site portion, then use the regular strip case.
                //
                // We unwrap_or() here because, at minimum, we return the substring
                // not containing the site.
                strip_category(url).unwrap_or(url)
            })
        }

        // Link with category but no site, e.g. theme:sigma-9.
        Some(idx) => Some(url[idx + 1..].trim_start()),

        // No stripping necessary
        None => None,
    }
}

#[test]
fn test_strip_category() {
    macro_rules! test {
        ($input:expr, $expected:expr $(,)?) => {{
            let actual = strip_category($input);

            assert_eq!(
                actual, $expected,
                "Actual stripped URL label doesn't match expected",
            );
        }};
    }

    test!("", None);
    test!("scp-001", None);
    test!("Guide Hub", None);
    test!("theme:just-girly-things", Some("just-girly-things"));
    test!("theme: just-girly-things", Some("just-girly-things"));
    test!("theme: Just Girly Things", Some("Just Girly Things"));
    test!("component:fancy-sidebar", Some("fancy-sidebar"));
    test!("component:Fancy Sidebar", Some("Fancy Sidebar"));
    test!("component: Fancy Sidebar", Some("Fancy Sidebar"));
    test!(
        "multiple:categories:here:test",
        Some("categories:here:test"),
    );
    test!(
        "multiple: categories: here: test",
        Some("categories: here: test"),
    );
    test!(":scp-wiki:scp-001", Some("scp-001"));
    test!(":scp-wiki : SCP-001", Some("SCP-001"));
    test!(":scp-wiki:system:recent-changes", Some("recent-changes"));
    test!(
        ":scp-wiki : system : Recent Changes",
        Some("Recent Changes"),
    );
    test!(": snippets : redirect", Some("redirect"));
    test!(":", None);
}

#[cfg(test)]
mod wikidot_tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::render::html::HtmlRender;
    use crate::settings::{WikitextMode, WikitextSettings};

    fn render(source: &str) -> String {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(source);
        let (tree, _errors) = crate::parse(&tokenization, &page_info, &settings).into();
        HtmlRender.render(&tree, &page_info, &settings).body
    }

    #[test]
    fn star_is_label_syntax_not_new_tab_in_wikidot() {
        let html = render("[[[*SCP-001]]] [[[*some-page|Label]]]");
        assert!(html.contains(r#"href="/scp-001">*SCP-001</a>"#), "{html}");
        assert!(html.contains(r#"href="/some-page">Label</a>"#), "{html}");
        assert!(!html.contains("target="), "{html}");
    }

    #[test]
    fn empty_label_uses_normalized_slug_in_wikidot() {
        let html = render("[[[some-page|]]] [[[:scp-wiki:scp-series|]]]");
        assert!(html.contains(">some-page</a>"), "{html}");
        assert!(html.contains(">scp-wiki:scp-series</a>"), "{html}");
        assert!(!html.contains("<page-title"), "{html}");
    }

    #[test]
    fn wikidot_cross_site_and_subpath_forms_are_local_slugs() {
        let html = render(concat!(
            "[[[:scp-wiki:component:theme|Sigma-9 Theme]]] ",
            "[[[:scp-wiki : system : Recent Changes]]] ",
            "[[[MAIN/#/page|Hash routing]]] ",
            "[[[example/edit|Edit page]]]",
        ));
        assert!(
            html.contains(r#"href="/scp-wiki:component:theme">Sigma-9 Theme</a>"#),
            "{html}",
        );
        assert!(
            html.contains(
                r#"href="/scp-wiki:system:recent-changes">:scp-wiki : system : Recent Changes</a>"#
            ),
            "{html}",
        );
        assert!(
            html.contains(r#"href="/main#/page">Hash routing</a>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"href="/example-edit">Edit page</a>"#),
            "{html}"
        );
    }

    #[test]
    fn wikidot_rejects_malformed_hash_routes_and_treats_spaced_url_as_slug() {
        let html = render(concat!(
            "[[[home###|Home]]] ",
            "[[[MAIN/##/page#toc1|Hash routing]]] ",
            "[[[ https://example.com/ | Example ]]]",
        ));
        assert!(html.contains("[[[home###|Home]]]"), "{html}");
        assert!(
            html.contains("[[[MAIN/##/page#toc1|Hash routing]]]"),
            "{html}"
        );
        assert!(
            html.contains(r#"href="/https:example-com">Example</a>"#),
            "{html}"
        );
    }

    #[test]
    fn wikidot_accepts_line_break_before_triple_link_label() {
        let html = render("[[[some-page |\nLabel]]]");
        assert!(html.contains(r#"href="/some-page">Label</a>"#), "{html}");
    }

    #[test]
    fn wikidot_star_with_empty_destination_links_to_root() {
        let html = render("[[[|some-page]]] [[[*|some-page]]]");
        assert!(html.contains("[[[|some-page]]]"), "{html}");
        assert!(html.contains(r#"href="/">some-page</a>"#), "{html}");
    }

    #[test]
    fn wikidot_accepts_line_break_before_triple_link_closer() {
        let html = render("[[[some-page\n]]]some-page");
        assert!(
            html.contains(r#"href="/some-page">some-page</a>some-page"#),
            "{html}"
        );
    }
}
