/*
 * parsing/rule/impls/block/blocks/html.rs
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
use crate::parsing::rule::impls::block::parser::BlockBodyStart;
use std::borrow::Cow;

pub const BLOCK_HTML: BlockRule = BlockRule {
    name: "block-html",
    accepts_names: &["html"],
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
    debug!("Parsing HTML block (in-head {in_head})");
    assert!(!flag_star, "HTML doesn't allow star flag");
    assert!(!flag_score, "HTML doesn't allow score flag");
    assert_block_name(&BLOCK_HTML, name);

    if !parser.settings().enable_html_blocks {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    if parser.settings().layout.legacy() && !parser.discarding_hidden_body() && in_head {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    let (arguments, body_start) =
        parser.get_head_map_with_body_start(&BLOCK_HTML, in_head)?;
    let body_content_start = parser.current().span.start;
    let html = parser.get_body_text(&BLOCK_HTML)?;
    let stored_html = if parser.settings().layout.legacy() {
        let leading_newline =
            body_start == BlockBodyStart::NextPhysicalLine && !html.starts_with('\n');
        let trailing_newline = matches!(html, Cow::Borrowed(_))
            && parser.full_text().inner()[body_content_start + html.len()..]
                .starts_with('\n');
        if leading_newline || trailing_newline {
            let mut stored = String::with_capacity(
                html.len() + usize::from(leading_newline) + usize::from(trailing_newline),
            );
            if leading_newline {
                stored.push('\n');
            }
            stored.push_str(&html);
            if trailing_newline {
                stored.push('\n');
            }
            Cow::Owned(stored)
        } else {
            html.clone()
        }
    } else {
        html.clone()
    };
    let element = Element::Html {
        contents: html,
        attributes: arguments.to_attribute_map(parser.settings()),
    };
    parser.push_html_block(stored_html);
    ok!(parser.settings().layout.legacy(); element)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn html_block_tracks_body_and_element_contents_in_wikijump() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        let tokenization = crate::tokenize("[[html]]\n<strong>raw</strong>\n[[/html]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let [
            Element::Html {
                contents,
                attributes,
            },
        ] = tree.elements.as_slice()
        else {
            panic!("expected one HTML block element, got {:?}", tree.elements);
        };
        let [tracked_html] = tree.html_blocks.as_slice() else {
            panic!(
                "expected one tracked HTML block, got {:?}",
                tree.html_blocks
            );
        };

        assert_eq!(contents, "<strong>raw</strong>");
        assert!(attributes.get().is_empty());
        assert_eq!(tracked_html, "<strong>raw</strong>");
    }

    #[test]
    fn wikidot_html_block_tracks_body_for_hosted_iframe() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization =
            crate::tokenize("[[html]]\n<strong>isolated</strong>\n[[/html]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(tree.html_blocks, ["\n<strong>isolated</strong>\n"]);

        let html = crate::render::html::HtmlRender
            .render(&tree, &page_info, &settings)
            .body;
        assert_eq!(
            html,
            r#"<p><iframe src="https://example.com/" allowtransparency="true" frameborder="0" class="html-block-iframe"></iframe></p>"#,
        );
    }

    #[test]
    fn wikidot_inline_html_block_does_not_gain_boundary_newlines() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[html]]<strong>inline</strong>[[/html]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(tree.html_blocks, ["<strong>inline</strong>"]);
    }

    #[test]
    fn wikidot_html_block_with_arguments_remains_literal() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[html garbage]]x[[/html]]");
        let (tree, _errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(tree.html_blocks.is_empty(), "{tree:#?}");
        assert!(!format!("{tree:?}").contains("Html {"), "{tree:#?}");
    }

    #[test]
    fn wikidot_html_blocks_can_be_kept_literal_by_the_caller() {
        let page_info = PageInfo::dummy();
        let mut settings =
            WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        settings.enable_html_blocks = false;
        let tokenization =
            crate::tokenize("[[html]]\n<strong>isolated</strong>\n[[/html]]");
        let (tree, _errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(tree.html_blocks.is_empty(), "{tree:#?}");
        assert_eq!(
            crate::render::html::HtmlRender
                .render(&tree, &page_info, &settings)
                .body,
            "<p>[[html]]<br>\n&lt;strong&gt;isolated&lt;/strong&gt;<br>\n[[/html]]</p>",
        );
    }

    #[test]
    fn disabled_wikidot_html_blocks_keep_preview_shapes_literal() {
        let page_info = PageInfo::dummy();
        let mut settings =
            WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        settings.enable_html_blocks = false;

        for (source, expected) in [
            (
                "[[html]]<b>X</b>[[/html]]",
                "<p>[[html]]&lt;b&gt;X&lt;/b&gt;[[/html]]</p>",
            ),
            (
                "BEFORE|[[html]]\n<b>X</b>\n[[/html]]|AFTER",
                "<p>BEFORE|[[html]]<br>\n&lt;b&gt;X&lt;/b&gt;<br>\n[[/html]]|AFTER</p>",
            ),
            (
                " [[html]]\n<b>X</b>\n[[/html]]",
                "<p>[[html]]<br>\n&lt;b&gt;X&lt;/b&gt;<br>\n[[/html]]</p>",
            ),
            (
                "> [[html]]\n> <b>X</b>\n> [[/html]]",
                "<blockquote><p>[[html]]<br>\n&lt;b&gt;X&lt;/b&gt;<br>\n[[/html]]</p></blockquote>",
            ),
            (
                "[[html class=\"probe\"]]\n<b>X</b>\n[[/html]]",
                "<p>[[html class=&quot;probe&quot;]]<br>\n&lt;b&gt;X&lt;/b&gt;<br>\n[[/html]]</p>",
            ),
            (
                "[[html]]\n<b>unclosed</b>",
                "<p>[[html]]<br>\n&lt;b&gt;unclosed&lt;/b&gt;</p>",
            ),
        ] {
            let tokenization = crate::tokenize(source);
            let (tree, _errors) =
                crate::parse(&tokenization, &page_info, &settings).into();
            let html = crate::render::html::HtmlRender
                .render(&tree, &page_info, &settings)
                .body;

            assert!(tree.html_blocks.is_empty(), "{source:?}: {tree:#?}");
            assert_eq!(html, expected, "{source:?}");
        }
    }
}
