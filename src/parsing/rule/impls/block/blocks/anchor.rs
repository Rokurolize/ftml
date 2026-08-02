/*
 * parsing/rule/impls/block/blocks/anchor.rs
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
use crate::parsing::strip_newlines;
use crate::tree::AnchorTarget;

pub const BLOCK_ANCHOR: BlockRule = BlockRule {
    name: "block-anchor",
    accepts_names: &["a", "anchor"],
    accepts_star: true,
    accepts_score: true,
    accepts_newlines: false,
    parse_fn,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Parsing anchor block (name '{name}', in-head {in_head}, star {flag_star})");
    assert_block_name(&BLOCK_ANCHOR, name);
    if parser.settings().layout.legacy() && flag_star {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let arguments = parser.get_head_map(&BLOCK_ANCHOR, in_head)?;
    let attributes = if parser.settings().layout.legacy() {
        arguments.to_wikidot_anchor_attribute_map(parser.settings())
    } else {
        arguments.to_attribute_map(parser.settings())
    };

    // Get anchor target depending on special
    let target = if flag_star {
        Some(AnchorTarget::NewTab)
    } else {
        None
    };

    // Get body content, without paragraphs
    let body = parser.get_body_elements(&BLOCK_ANCHOR, false)?;
    let (mut elements, errors, paragraph_safe) = body.into();
    if parser.settings().layout.legacy()
        && let Some(Element::Text(text)) = elements.first_mut()
        && text.starts_with(' ')
    {
        text.to_mut().remove(0);
        if text.is_empty() {
            elements.remove(0);
        }
    }

    if flag_score {
        strip_newlines(&mut elements);
        if parser.settings().layout.legacy()
            && matches!(
                parser.current().token,
                Token::LineBreak | Token::ParagraphBreak
            )
        {
            parser.step()?;
        }
    }

    let element = Element::Anchor {
        elements,
        attributes,
        target,
    };

    success_elements_with_paragraph_safety(
        paragraph_safe || (flag_score && parser.settings().layout.legacy()),
        element,
        errors,
    )
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::render::html::HtmlRender;
    use crate::settings::{WikitextMode, WikitextSettings};

    fn render_with_layout(source: &str, layout: Layout) -> String {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
        let tokenization = crate::tokenize(source);
        let (tree, _errors) = crate::parse(&tokenization, &page_info, &settings).into();
        HtmlRender.render(&tree, &page_info, &settings).body
    }

    fn render(source: &str) -> String {
        render_with_layout(source, Layout::Wikidot)
    }

    #[test]
    fn wikidot_anchor_preserves_relative_href() {
        assert_eq!(
            render(r#"[[a href="some-page"]]click[[/a]]"#),
            r#"<p><a href="some-page">click</a></p>"#,
        );
    }

    #[test]
    fn wikidot_anchor_discards_title_attribute() {
        let source = r#"[[a href="https://example.com/" title="Label"]]Label[[/a]]"#;
        assert_eq!(
            render(source),
            r#"<p><a href="https://example.com/">Label</a></p>"#,
        );
        assert_eq!(
            render_with_layout(source, Layout::Wikijump),
            r#"<p><a class="wj-anchor" href="https://example.com/" title="Label">Label</a></p>"#,
        );
    }

    #[test]
    fn wikidot_star_anchor_stays_literal() {
        let html = render(r#"[[*a href="some-page"]]click[[/a]]"#);
        assert!(
            html.contains("[[*a href=&quot;some-page&quot;]]click[[/a]]"),
            "{html}"
        );
    }

    #[test]
    fn wikidot_anchor_keeps_dangerous_href_sanitized() {
        for href in [
            "javascript:alert(1)",
            "data:text/html,test",
            "vbscript:msgbox(1)",
        ] {
            let html = render(&format!(r#"[[a href="{href}"]]click[[/a]]"#));
            assert!(!html.contains(&format!(r#"href="{href}""#)), "{html}");
            assert!(
                html.contains(r##"href="#invalid-url""##)
                    || html.contains(&format!(r#"href="/{href}""#)),
                "{html}",
            );
        }
    }

    #[test]
    fn wikidot_score_anchor_consumes_following_newlines() {
        let html = render(concat!(
            "EMPTY [[a_]][[/a]]\n",
            "BASIC [[a_ href=\"some-page\"]]\nclick\n[[/a]]\n",
            "ATTRS [[a_ href=\"https://example.com\"]]my link[[/a]]\n\n",
            "**BOTH**",
        ));
        assert_eq!(
            html,
            concat!(
                "<p>EMPTY <a href></a>",
                "BASIC <a href=\"some-page\">click</a>",
                "ATTRS <a href=\"https://example.com\">my link</a>",
                "<strong>BOTH</strong></p>",
            ),
        );
    }
}
