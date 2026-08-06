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
use crate::parsing::collect::{collect_comment_elided_keep, consume_valid_comment};
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
    let source_start = parser.current().span.start;
    assert_step(parser, Token::LeftLink)?;
    try_consume_link(parser, RULE_LINK_TRIPLE, None, source_start)
}

fn link_new_tab<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    let source_start = parser.current().span.start;
    assert_step(parser, Token::LeftLinkStar)?;
    try_consume_link(
        parser,
        RULE_LINK_TRIPLE_NEW_TAB,
        Some(AnchorTarget::NewTab),
        source_start,
    )
}

/// Build a triple-bracket link with the given target.
fn try_consume_link<'r, 't>(
    parser: &mut Parser<'r, 't>,
    rule: Rule,
    target: Option<AnchorTarget>,
    source_start: usize,
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
    let (url, last, authored_url_prefix_is_url) = if parser.settings().layout.legacy() {
        let (url, last) =
            collect_comment_elided_keep(parser, &url_close, url_invalid, None)?;
        let authored_url_prefix_is_url =
            is_url(url.prefix_before_first_comment().trim_start());
        (url.into_cow(), last, authored_url_prefix_is_url)
    } else {
        let (url, last) = collect_text_keep(parser, rule, &url_close, url_invalid, None)?;
        (Cow::Borrowed(url), last, is_url(url.trim_start()))
    };

    // Trim text
    let leading_space = url.trim_start().len() != url.len();
    let url = trim_cow(url);

    // If url is an empty string, parsing should fail, there's nothing here
    if url.is_empty()
        && parser.settings().layout.legacy()
        && target.is_some()
        && last.token == Token::Pipe
    {
        return build_separate(
            parser,
            rule,
            Cow::Borrowed(""),
            false,
            false,
            target,
            source_start,
        );
    }
    if url.is_empty() {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    // Determine what token we ended on, i.e. which [[[ variant it is.
    match last.token {
        // [[[name]]] type links
        Token::RightLink => build_same(
            parser,
            url,
            leading_space,
            authored_url_prefix_is_url,
            target,
        ),

        // [[[url|label]]] type links
        Token::Pipe => build_separate(
            parser,
            rule,
            url,
            leading_space,
            authored_url_prefix_is_url,
            target,
            source_start,
        ),

        // Token was already checked in collect_text(), impossible case
        _ => unreachable!(),
    }
}

/// Helper to build link with the same URL and label.
/// e.g. `[[[name]]]`
fn build_same<'r, 't>(
    parser: &mut Parser<'r, 't>,
    url: Cow<'t, str>,
    leading_space: bool,
    authored_url_prefix_is_url: bool,
    target: Option<AnchorTarget>,
) -> ParseResult<'r, 't, Elements<'t>> {
    // Remove category, if present.
    // If None, then the label is the original URL.
    let label = match url.clone() {
        Cow::Borrowed(url) => Cow::Borrowed(same_link_label(parser, url)),
        Cow::Owned(url) => Cow::Owned(same_link_label(parser, &url).to_owned()),
    };
    let label = if parser.settings().layout.legacy()
        && target.is_some()
        && !is_url(&url)
        && strip_category(&url).is_none()
    {
        Cow::Owned(format!("*{label}"))
    } else {
        label
    };

    // Parse out link location
    let parsed_link =
        parse_link_location(parser, url, leading_space, authored_url_prefix_is_url);
    let Some((link, ltype)) = parsed_link else {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    };

    // Build and return element
    let element = Element::Link {
        ltype,
        target: wikidot_target(parser, target, &link),
        link,
        label: LinkLabel::Slug(label),
    };

    success_elements(element)
}

fn same_link_label<'t>(parser: &Parser<'_, '_>, url: &'t str) -> &'t str {
    match strip_category(url) {
        Some(_)
            if parser.settings().layout.legacy()
                && url.starts_with(':')
                && (url.contains(" :") || url.contains(": ")) =>
        {
            url
        }
        Some(stripped) => stripped,
        None => url,
    }
}

/// Helper to build link with separate URL and label.
/// e.g. `[[[page|label]]]`, or `[[[page|]]]`
fn build_separate<'r, 't>(
    parser: &mut Parser<'r, 't>,
    rule: Rule,
    url: Cow<'t, str>,
    leading_space: bool,
    authored_url_prefix_is_url: bool,
    target: Option<AnchorTarget>,
    source_start: usize,
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
    let label = if legacy {
        match collect_wikidot_separate_label(parser, source_start)? {
            WikidotLabelCandidate::Label(label) => label,
            WikidotLabelCandidate::PreserveSource(source) => {
                return ok!(Element::Text(source));
            }
        }
    } else {
        Cow::Borrowed(collect_text(
            parser,
            rule,
            &label_close,
            label_invalid,
            None,
        )?)
    };

    // Trim label
    let label = match label {
        Cow::Borrowed(label) => Cow::Borrowed(label.trim()),
        Cow::Owned(label) => Cow::Owned(label.trim().to_owned()),
    };

    // Parse out link location
    let parsed_link = parse_link_location(
        parser,
        url.clone(),
        leading_space,
        authored_url_prefix_is_url,
    );
    let Some((link, ltype)) = parsed_link else {
        if legacy && (url.contains("###") || url.contains("/##/")) {
            return ok!(Element::Text(Cow::Owned(format!("[[[{url}|{label}]]]"))));
        }
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    };

    // Wikidot derives an empty external-link label from its normalized URL slug.
    let label = if label.is_empty() && legacy {
        let mut normalized = str!(url.as_ref());
        if normalized.starts_with(':') {
            normalized.remove(0);
        }
        normalize(&mut normalized);
        LinkLabel::Text(Cow::Owned(normalized))
    } else if label.is_empty() {
        LinkLabel::Page
    } else {
        LinkLabel::Text(label)
    };

    // Build link element
    let element = Element::Link {
        ltype,
        target: wikidot_target(parser, target, &link),
        link,
        label,
    };

    // Return result
    success_elements(element)
}

fn collect_wikidot_separate_label<'r, 't>(
    parser: &mut Parser<'r, 't>,
    source_start: usize,
) -> Result<WikidotLabelCandidate<'t>, ParseError>
where
    'r: 't,
{
    use super::raw::RULE_RAW;

    const BRACKET_OWNERS: &[Token] = &[
        Token::LeftBracket,
        Token::LeftBracketAnchor,
        Token::LeftBracketStar,
        Token::RightBracket,
        Token::LeftBlock,
        Token::LeftBlockEnd,
        Token::LeftBlockAnchor,
        Token::LeftBlockStar,
        Token::LeftMath,
        Token::RightBlock,
        Token::RightMath,
        Token::LeftLink,
        Token::LeftLinkStar,
    ];

    let source = parser.full_text().inner();
    let start = parser.current().span.start;
    let mut segment_start = start;
    let mut visible_label = None::<String>;
    let mut preserve_source = false;

    loop {
        match parser.current().token {
            Token::RightLink => {
                let end = parser.current().span.start;
                let source_end = parser.current().span.end;
                parser.step()?;
                if preserve_source {
                    return Ok(WikidotLabelCandidate::PreserveSource(Cow::Borrowed(
                        &source[source_start..source_end],
                    )));
                }
                return match visible_label {
                    Some(mut label) => {
                        label.push_str(&source[segment_start..end]);
                        Ok(WikidotLabelCandidate::Label(Cow::Owned(label)))
                    }
                    None => Ok(WikidotLabelCandidate::Label(Cow::Borrowed(
                        &source[start..end],
                    ))),
                };
            }

            _ if preserve_source => {
                parser.step()?;
            }

            // A valid comment is transparent to the label. Speculatively use
            // the ordinary comment rule so extended closers and generated
            // input retain their existing behavior. An incomplete comment is
            // a bracket owner and therefore rolls back the outer link.
            Token::LeftComment => {
                let comment_start = parser.current().span.start;
                let mut comment_parser = parser.clone();
                if consume_valid_comment(&mut comment_parser).is_err() {
                    return Err(parser.make_err(ParseErrorKind::RuleFailed));
                }

                visible_label
                    .get_or_insert_with(String::new)
                    .push_str(&source[segment_start..comment_start]);
                segment_start = comment_parser.current().span.start;
                parser.update(&comment_parser);

                // The comment's final `]` can also be the first bracket in
                // the outer `]]]` closer, leaving `]]` as the current token.
                if parser.current().token == Token::RightBlock {
                    parser.step()?;
                    return Ok(WikidotLabelCandidate::Label(Cow::Owned(
                        visible_label.unwrap_or_default(),
                    )));
                }
            }

            // Only complete raw syntax owns the outer label. This preserves
            // unmatched `@@` and `@<` as literal label text while allowing a
            // valid raw span to render after the outer transaction rolls back.
            Token::Raw | Token::LeftRaw => {
                let mut raw_parser = parser.clone();
                if RULE_RAW.try_consume(&mut raw_parser).is_ok() {
                    return Err(parser.make_err(ParseErrorKind::RuleFailed));
                }
                parser.step()?;
            }

            Token::ParagraphBreak | Token::InputEnd => {
                let kind = if parser.current().token == Token::InputEnd {
                    ParseErrorKind::EndOfInput
                } else {
                    ParseErrorKind::RuleFailed
                };
                return Err(parser.make_err(kind));
            }

            Token::GeneratedPageLink | Token::GeneratedTagLinks | Token::RuntimeText => {
                return Err(parser.make_err(ParseErrorKind::RuleFailed));
            }

            // Only the live-backed inline block owners may execute after the
            // outer link rolls back. Other block-shaped labels still invalidate
            // the link, but preserve the complete invocation as text so hosted
            // HTML, modules, includes, and future blocks gain no authority.
            Token::LeftBlock => {
                if wikidot_label_block_executes(parser) {
                    return Err(parser.make_err(ParseErrorKind::RuleFailed));
                }
                preserve_source = true;
                parser.step()?;
            }

            token if BRACKET_OWNERS.contains(&token) => {
                return Err(parser.make_err(ParseErrorKind::RuleFailed));
            }

            _ => {
                parser.step()?;
            }
        }
    }
}

enum WikidotLabelCandidate<'t> {
    Label(Cow<'t, str>),
    PreserveSource(Cow<'t, str>),
}

fn wikidot_label_block_executes<'r, 't>(parser: &Parser<'r, 't>) -> bool
where
    'r: 't,
{
    let mut block = parser.clone();
    let Ok((name, _)) = block.get_block_name(false) else {
        return false;
    };

    ["span", "size", "image", "footnote", "iframe"]
        .iter()
        .any(|owner| name.eq_ignore_ascii_case(owner))
}

fn wikidot_target(
    parser: &Parser<'_, '_>,
    target: Option<AnchorTarget>,
    link: &LinkLocation<'_>,
) -> Option<AnchorTarget> {
    match (parser.settings().layout.legacy(), link) {
        (true, LinkLocation::Url(_)) | (false, _) => target,
        (true, LinkLocation::Page(_)) => None,
    }
}

fn parse_link_location<'r, 't>(
    parser: &Parser<'r, 't>,
    url: Cow<'t, str>,
    leading_space: bool,
    authored_url_prefix_is_url: bool,
) -> Option<(LinkLocation<'t>, LinkType)> {
    if !parser.settings().layout.legacy() {
        return LinkLocation::parse_with_interwiki(url, parser.settings());
    }

    if url.contains("###") || url.contains("/##/") {
        return None;
    }

    if url.is_empty() {
        return Some((LinkLocation::Page(PageRef::page_only("")), LinkType::Page));
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

    if leading_space && is_url(&url) && authored_url_prefix_is_url {
        let page = url.replace("://", ":");
        let page = page.trim_end_matches('/');
        return Some((LinkLocation::Page(PageRef::page_only(page)), LinkType::Page));
    }

    if let Some((page, extra)) = url.split_once("/#/") {
        let mut page_ref = PageRef::page_only(page);
        page_ref.extra = Some(format!("#/{extra}"));
        return Some((LinkLocation::Page(page_ref), LinkType::Page));
    }

    if url.contains('/')
        && !url.starts_with('/')
        && (!is_url(&url) || !authored_url_prefix_is_url)
    {
        return Some((
            LinkLocation::Page(PageRef::page_only(url.replace('/', "-"))),
            LinkType::Page,
        ));
    }

    if is_url(&url) && !authored_url_prefix_is_url {
        return Some((LinkLocation::Page(PageRef::page_only(&url)), LinkType::Page));
    }

    LinkLocation::parse_with_interwiki(url, parser.settings())
}

fn trim_cow(value: Cow<'_, str>) -> Cow<'_, str> {
    match value {
        Cow::Borrowed(value) => Cow::Borrowed(value.trim()),
        Cow::Owned(value) => Cow::Owned(value.trim().to_owned()),
    }
}

/// Strip off the category for use in URL triple-bracket links.
///
/// The label for a URL link is its URL, but without its category.
/// For instance, `theme: Sigma-9` becomes just `Sigma-9`.
///
/// It returns `Some(_)` if a slice was performed, and `None` if
/// the string would have been returned as-is.
fn strip_category(url: &str) -> Option<&str> {
    // A URL scheme colon is not a Wikidot page-category separator. Live
    // Wikidot keeps the complete URL as the default label for both ordinary
    // and new-tab unlabeled external triple links.
    if is_url(url) {
        return None;
    }
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
    fn wikidot_star_external_triple_links_open_a_new_tab() {
        // Live anonymous PagePreviewModule provenance:
        // listpages-synchronized-final-20260730/actionable-23-references-20260731.jsonl,
        // case jp:neko-sagashi:L17:B190.
        let html = render(concat!(
            "[[[*http://ja.scp-wiki.net/scp-040-jp |ねこでした。]]] ",
            "[[[http://ja.scp-wiki.net/scp-040-jp |ordinary]]]",
        ));

        assert!(
            html.contains(
                r#"<a href="http://ja.scp-wiki.net/scp-040-jp" target="_blank" rel="noopener noreferrer">ねこでした。</a>"#,
            ),
            "{html}",
        );
        assert!(
            html.contains(r#"<a href="http://ja.scp-wiki.net/scp-040-jp">ordinary</a>"#,),
            "{html}",
        );
    }

    #[test]
    fn wikidot_unlabeled_external_triple_links_keep_the_scheme_in_the_label() {
        let html = render(concat!(
            "[[[http://sandbox-for-codex.wikidot.com/example]]] ",
            "[[[*http://sandbox-for-codex.wikidot.com/new-tab]]]",
        ));

        assert!(
            html.contains(concat!(
                r#"<a href="http://sandbox-for-codex.wikidot.com/example">"#,
                "http://sandbox-for-codex.wikidot.com/example</a>",
            )),
            "{html}",
        );
        assert!(
            html.contains(concat!(
                r#"<a href="http://sandbox-for-codex.wikidot.com/new-tab" target="_blank" rel="noopener noreferrer">"#,
                "http://sandbox-for-codex.wikidot.com/new-tab</a>",
            )),
            "{html}",
        );
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
    fn wikidot_only_treats_leading_space_before_url_as_slug() {
        let html = render(concat!(
            "[[[https://example.com | Trailing]]] ",
            "[[[ https://example.com|Leading]]] ",
            "[[[ https://example.com | Both]]]",
        ));
        assert!(
            html.contains(r#"href="https://example.com">Trailing</a>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"href="/https:example-com">Leading</a>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"href="/https:example-com">Both</a>"#),
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
        assert!(
            html.contains(r#"class="newpage" href="/">some-page</a>"#),
            "{html}"
        );
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
