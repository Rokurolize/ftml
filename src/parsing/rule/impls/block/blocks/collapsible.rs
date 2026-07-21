/*
 * parsing/rule/impls/block/blocks/collapsible.rs
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
use crate::parsing::{ParseError, ParseErrorKind};

pub const BLOCK_COLLAPSIBLE: BlockRule = BlockRule {
    name: "block-collapsible",
    accepts_names: &["collapsible"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: true,
    parse_fn,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Parsing collapsible block (in-head {in_head})");
    assert!(!flag_star, "Collapsible doesn't allow star flag");
    assert!(!flag_score, "Collapsible doesn't allow score flag");
    assert_block_name(&BLOCK_COLLAPSIBLE, name);

    let head = parser.get_head_map_with_body_start(&BLOCK_COLLAPSIBLE, in_head)?;
    let (mut arguments, body_start) = head;

    // Get display arguments
    let show_text = arguments.get("show");
    let hide_text = arguments.get("hide");

    // Get folding arguments
    //
    // We invert this first argument since "folded=no" means "start_open=yes"
    let start_open = !arguments.get_bool(parser, "folded")?.unwrap_or(true);
    let (show_top, show_bottom) = match arguments.get("hideLocation") {
        Some(value) => parse_hide_location(&value, parser)?,
        None => (true, false),
    };

    // Get body content, with paragraphs.
    // Discard paragraph_safe, since collapsibles never are.
    let body =
        parser.get_body_elements_with_context(&BLOCK_COLLAPSIBLE, true, body_start)?;
    let (elements, errors, _) = body.into();

    // Build element and return
    let element = Element::Collapsible {
        elements,
        attributes: arguments.to_attribute_map(parser.settings()),
        start_open,
        show_text,
        hide_text,
        show_top,
        show_bottom,
    };

    ok!(element, errors)
}

fn parse_hide_location(s: &str, parser: &Parser) -> Result<(bool, bool), ParseError> {
    const NAMES: [(&str, (bool, bool)); 5] = [
        ("top", (true, false)),
        ("bottom", (false, true)),
        ("both", (true, true)),
        ("neither", (false, false)),
        ("none", (false, false)),
    ];

    let s = s.trim();
    for &(name, value) in &NAMES {
        if name.eq_ignore_ascii_case(s) {
            return Ok(value);
        }
    }

    warn!("Unknown hideLocation argument '{s}'");
    Err(parser.make_err(ParseErrorKind::BlockMalformedArguments))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender, text::TextRender};
    use crate::settings::{WikitextMode, WikitextSettings};
    use std::time::{Duration, Instant};

    #[test]
    fn collapsible_hide_location_controls_visible_handles() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        let tokenization = crate::tokenize(
            "[[collapsible folded=\"no\" hideLocation=\"both\"]]Body[[/collapsible]]",
        );
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        assert!(errors.is_empty(), "{errors:?}");
        match tree.elements.as_slice() {
            [
                Element::Collapsible {
                    start_open,
                    show_top,
                    show_bottom,
                    ..
                },
            ] => {
                assert!(*start_open);
                assert!(*show_top);
                assert!(*show_bottom);
            }
            other => panic!("expected collapsible element, got {other:?}"),
        }

        let tokenization =
            crate::tokenize("[[collapsible hideLocation=\"side\"]]Body[[/collapsible]]");
        let (_tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        assert!(
            errors
                .iter()
                .any(|error| error.kind() == ParseErrorKind::BlockMalformedArguments)
        );
    }

    fn render(input: &str) -> (String, String, Vec<ParseError>) {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        let text = TextRender.render(&tree, &page_info, &settings);
        (html, text, errors)
    }

    #[test]
    fn quoted_multiline_collapsibles_remain_native_and_bounded() {
        let mut input = String::new();
        for index in 0..24 {
            input.push_str(&format!(
                concat!(
                    "> title-{0}\n",
                    "> [[collapsible show=\"show-{0}\" hide=\"hide-{0}\"]]\n",
                    "> \n",
                    "> body-{0}\n",
                    "> [[/collapsible]]\n",
                    "\n",
                ),
                index,
            ));
        }
        input.push_str("following-page-content\n");

        let started = Instant::now();
        let (html, text, errors) = render(&input);
        let elapsed = started.elapsed();

        assert!(elapsed < Duration::from_secs(5), "elapsed {elapsed:?}");
        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html.matches("class=\"collapsible-block\"").count(), 24);
        assert_eq!(html.matches("<blockquote>").count(), 24);
        assert!(html.contains("show-0"), "{html}");
        assert!(html.contains("hide-23"), "{html}");
        assert!(html.contains("body-0"), "{html}");
        assert!(html.contains("body-23"), "{html}");
        assert!(html.contains("following-page-content"), "{html}");
        assert!(!html.contains("[[collapsible"), "{html}");
        assert!(!html.contains("[[/collapsible]]"), "{html}");
        assert!(text.contains("body-0"), "{text}");
        assert!(text.contains("body-23"), "{text}");
    }

    #[test]
    fn continuous_unclosed_quoted_collapsibles_are_bounded() {
        let mut input = String::new();
        for index in 0..1024 {
            input.push_str(&format!(
                "> [[collapsible show=\"show-{index}\"]]\n> readable-{index}\n",
            ));
        }

        let started = Instant::now();
        let (html, _, _) = render(&input);
        let elapsed = started.elapsed();

        assert!(elapsed < Duration::from_secs(5), "elapsed {elapsed:?}");
        assert_eq!(html.matches("class=\"collapsible-block\"").count(), 0);
        assert!(html.contains("[[collapsible"), "{html}");
        assert!(html.contains("readable-0"), "{html}");
        assert!(html.contains("readable-1023"), "{html}");
    }

    #[test]
    fn quoted_collapsible_keeps_inline_body_on_generic_path() {
        let input = concat!(
            "> [[collapsible show=\"show\" hide=\"hide\"]]inline body\n",
            "[[/collapsible]]\n",
            "following content\n",
        );
        let (html, _, errors) = render(input);

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html.matches("class=\"collapsible-block\"").count(), 1);
        assert!(html.contains("inline body"), "{html}");
        assert!(html.contains("following content"), "{html}");
    }

    #[test]
    fn quoted_multiline_collapsible_allows_immediate_close_and_spaces() {
        let input = concat!(
            "> [[collapsible show=\"show\" hide=\"hide\"]]\n",
            "> [[/collapsible]]   \n",
            "> following quote\n",
        );
        let (html, _, errors) = render(input);

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html.matches("class=\"collapsible-block\"").count(), 1);
        assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");
        assert!(html.contains("following quote"), "{html}");
        assert!(!html.contains("[[/collapsible]]"), "{html}");
    }

    #[test]
    fn quoted_midline_close_stays_literal_until_standalone_close() {
        let input = concat!(
            "> [[collapsible show=\"show\"]]\n",
            "> before [[/collapsible]] stray\n",
            "> body after false close\n",
            "> [[/collapsible]]\n",
            "> following quote\n",
        );
        let (html, text, _errors) = render(input);

        assert_eq!(html.matches("class=\"collapsible-block\"").count(), 1);
        assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");
        assert!(html.contains("[[/collapsible]] stray"), "{html}");
        assert!(html.contains("body after false close"), "{html}");
        assert!(html.contains("following quote"), "{html}");
        assert!(text.contains("[[/collapsible]] stray"), "{text}");
        assert!(text.contains("body after false close"), "{text}");
    }

    #[test]
    fn quoted_close_with_trailing_text_fails_closed_inside_blockquote() {
        let input = concat!(
            "> [[collapsible show=\"show\" hide=\"hide\"]]\n",
            "> body\n",
            "> [[/collapsible]] still quoted\n",
            "following page\n",
        );
        let (html, _, _) = render(input);

        assert_eq!(html.matches("class=\"collapsible-block\"").count(), 0);
        assert!(html.contains("[[collapsible"), "{html}");
        assert!(html.contains("[[/collapsible]] still quoted"), "{html}");
        let quoted = html.find("still quoted").expect("quoted text missing");
        let blockquote_end = html
            .find("</blockquote>")
            .expect("blockquote close missing");
        let following = html.find("following page").expect("following text missing");
        assert!(quoted < blockquote_end, "{html}");
        assert!(blockquote_end < following, "{html}");
    }

    #[test]
    fn quoted_collapsible_does_not_cross_unquoted_boundaries() {
        for input in [
            concat!(
                "> [[collapsible show=\"show\"]]\n",
                "unquoted body\n",
                "[[/collapsible]]\n",
                "following page\n",
            ),
            concat!(
                "> [[collapsible show=\"show\"]]\n",
                "> body\n",
                "\n",
                "> [[/collapsible]]\n",
                "following page\n",
            ),
        ] {
            let (html, _, _) = render(input);
            assert_eq!(html.matches("class=\"collapsible-block\"").count(), 0);
            assert!(html.contains("[[collapsible"), "{html}");
            assert!(html.contains("following page"), "{html}");
        }
    }

    #[test]
    fn nested_span_does_not_hide_an_earlier_collapsible_close() {
        let input = concat!(
            "> [[collapsible show=\"show\"]]\n",
            "> [[span]]\n",
            "> [[/collapsible]]\n",
            "> [[/span]]\n",
            "\n",
            "> escaped page content\n",
            "> [[/collapsible]]\n",
            "following page\n",
        );
        let (html, _, _) = render(input);

        // Live Wikidot closes the collapsible at the first closer even though
        // the span remains active until the following quoted line.
        assert_eq!(
            html.matches("class=\"collapsible-block\"").count(),
            1,
            "{html}"
        );
        assert!(!html.contains("[[collapsible show"), "{html}");
        assert!(html.contains("escaped page content"), "{html}");
        assert!(html.contains("following page"), "{html}");
    }

    #[test]
    fn nested_multiline_inline_collector_prepares_each_quote_prefix() {
        let input = concat!(
            "> [[collapsible show=\"show\"]]\n",
            "> **alpha\n",
            "> beta**\n",
            "> [[/collapsible]]\n",
            "> following quote\n",
        );
        let (html, _, errors) = render(input);

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html.matches("class=\"collapsible-block\"").count(),
            1,
            "{html}"
        );
        assert!(html.contains("<strong>alphabeta</strong>"), "{html}");
        assert!(html.contains("following quote"), "{html}");
        assert!(!html.contains("&gt; beta"), "{html}");
    }

    #[test]
    fn quoted_collapsible_reuses_block_level_paragraph_semantics() {
        let input = concat!(
            "> [[collapsible show=\"show\"]]\n",
            "> text before rule\n",
            "> ----\n",
            "> text after rule\n",
            "> [[/collapsible]]\n",
        );
        let (html, _, errors) = render(input);

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains("<p>text before rule</p><hr>"), "{html}");
        assert!(html.contains("<p>text after rule</p>"), "{html}");
        assert!(!html.contains("text before rule<br>"), "{html}");
    }

    #[test]
    fn quoted_collapsible_preserves_nested_native_quote_depths() {
        let input = concat!(
            "> [[collapsible show=\"show\"]]\n",
            ">> nested one\n",
            ">>> nested two\n",
            "> [[/collapsible]]\n",
            "> following quote\n",
            "following page\n",
        );
        let (html, _, errors) = render(input);

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html.matches("class=\"collapsible-block\"").count(), 1);
        assert_eq!(html.matches("<blockquote>").count(), 3, "{html}");
        assert!(html.contains("nested one"), "{html}");
        assert!(html.contains("nested two"), "{html}");
        assert!(html.contains("following quote"), "{html}");
        assert!(html.contains("following page"), "{html}");
        assert!(!html.contains("[[collapsible"), "{html}");
    }

    #[test]
    fn nested_quoted_collapsibles_restore_outer_quote_cursor() {
        let input = concat!(
            "> [[collapsible show=\"outer\"]]\n",
            ">> [[collapsible show=\"inner\"]]\n",
            ">> inner body\n",
            ">> [[/collapsible]]\n",
            "> outer body\n",
            "> [[/collapsible]]\n",
            "> following quote\n",
        );
        let (html, _, errors) = render(input);

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html.matches("class=\"collapsible-block\"").count(), 2);
        assert_eq!(html.matches("<blockquote>").count(), 2, "{html}");
        assert!(html.contains("inner body"), "{html}");
        assert!(html.contains("outer body"), "{html}");
        assert!(html.contains("following quote"), "{html}");
        assert!(!html.contains("[[collapsible"), "{html}");
    }

    #[test]
    fn nested_spaced_quote_collapsible_uses_absolute_depth() {
        let input = concat!(
            "> [[collapsible show=\"outer\"]]\n",
            "> > [[collapsible show=\"inner\"]]\n",
            "> > inner body\n",
            "> > [[/collapsible]]\n",
            "> outer body\n",
            "> [[/collapsible]]\n",
            "> following quote\n",
        );
        let (html, _, errors) = render(input);

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html.matches("class=\"collapsible-block\"").count(),
            2,
            "{html}"
        );
        assert_eq!(html.matches("<blockquote>").count(), 2, "{html}");
        assert!(html.contains("inner body"), "{html}");
        assert!(html.contains("outer body"), "{html}");
        assert!(html.contains("following quote"), "{html}");
        assert!(!html.contains("[[collapsible"), "{html}");
    }

    #[test]
    fn quoted_collapsible_scopes_close_policy_away_from_inline_blocks() {
        let input = concat!(
            "> [[collapsible show=\"show\"]]\n",
            "> [[span]]inline span[[/span]]\n",
            "> [[/collapsible]]\n",
        );
        let (html, _, errors) = render(input);

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html.matches("class=\"collapsible-block\"").count(), 1);
        assert!(html.contains("<span>inline span</span>"), "{html}");
        assert!(!html.contains("[[span]]"), "{html}");
    }

    #[test]
    fn quoted_collapsible_keeps_nested_raw_text_block_markers_literal() {
        let input = concat!(
            "> [[collapsible show=\"show\"]]\n",
            "> [[code]]\n",
            "> hello code\n",
            "> [[/collapsible]] literal code text\n",
            "> [[/code]]\n",
            "> [[raw]]\n",
            "> **raw text**\n",
            "> [[/raw]]\n",
            "> [[/collapsible]]\n",
            "> following quote\n",
        );
        let (html, _, _errors) = render(input);

        assert_eq!(html.matches("class=\"collapsible-block\"").count(), 1);
        assert_eq!(html.matches("<blockquote>").count(), 1, "{html}");
        assert!(html.contains("hello code"), "{html}");
        assert!(
            html.contains("[[/collapsible]] literal code text"),
            "{html}"
        );
        assert!(html.contains("<strong>raw text</strong>"), "{html}");
        assert!(html.contains("[[code]]"), "{html}");
        assert!(html.contains("[[/code]]"), "{html}");
        assert!(html.contains("[[raw]]"), "{html}");
        assert!(html.contains("[[/raw]]"), "{html}");
        assert!(!html.contains("&gt; hello code"), "{html}");
        assert!(html.contains("following quote"), "{html}");
    }

    #[test]
    fn repeated_unclosed_quoted_code_blocks_are_bounded() {
        let mut input = String::from("> [[collapsible show=\"show\"]]\n");
        for index in 0..512 {
            input.push_str(&format!("> [[code]]\n> readable-code-{index}\n",));
        }
        input.push_str("> [[/collapsible]]\n");

        let started = Instant::now();
        let (html, _, _errors) = render(&input);
        let elapsed = started.elapsed();

        assert!(elapsed < Duration::from_secs(5), "elapsed {elapsed:?}");
        assert_eq!(html.matches("class=\"collapsible-block\"").count(), 1);
        assert!(html.contains("[[code]]"), "{html}");
        assert!(html.contains("readable-code-0"), "{html}");
        assert!(html.contains("readable-code-511"), "{html}");
    }
}
