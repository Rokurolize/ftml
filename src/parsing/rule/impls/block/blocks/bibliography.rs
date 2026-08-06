/*
 * parsing/rule/impls/block/blocks/bibliography.rs
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
use crate::tree::Bibliography;

pub const BLOCK_BIBLIOGRAPHY: BlockRule = BlockRule {
    name: "block-bibliography",
    accepts_names: &["bibliography"],
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
    debug!("Parsing bibliography block {name}, in-head {in_head}, score {flag_score}");
    assert!(!flag_star, "Bibliography doesn't allow star flag");
    assert!(!flag_score, "Bibliography doesn't allow score flag");
    assert_block_name(&BLOCK_BIBLIOGRAPHY, name);
    if parser.settings().layout.legacy() && name != "bibliography" {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let mut arguments = parser.get_head_map_wikidot(&BLOCK_BIBLIOGRAPHY, in_head)?;

    let title = arguments.get("title");
    let hide = arguments.get_bool(parser, "hide")?.unwrap_or(false);

    // Wikidot leaves a bibliography opener without a matching closer
    // literal at the end of a page. The legacy parser otherwise treats EOF
    // as an implicit close and invents an empty bibliography container.
    if parser.settings().layout.legacy() && parser.current().token == Token::InputEnd {
        let source = parser.full_text().inner();
        let opener_start = source[..parser.current().span.start]
            .rfind("[[")
            .unwrap_or(parser.current().span.start);
        return ok!(Element::Text(cow!(&source[opener_start..])));
    }

    // Get body content. Wikidot accepts non-definition content and renders it
    // directly inside the bibliography container.
    //
    // We also discard paragraph_safe, since it's not relevant, and this element
    // never is (uses <div>).
    let body = parser.get_body_elements(&BLOCK_BIBLIOGRAPHY, false)?;
    let (elements, errors, _) = body.into();

    // Build up the bibliography
    //
    // Look through to find definition lists, ignoring "space" type elements,
    // and adding definition list values to the bibliography as we find them.
    let mut bibliography = Bibliography::new();
    let mut in_residual = false;

    for element in elements {
        match element {
            // Append definition list entries
            Element::DefinitionList(items) => {
                for item in items {
                    bibliography.add(item.key_string, item.value_elements);
                }
                in_residual = false;
            }

            // Skip structural whitespace between definition lists.
            _ if element.is_whitespace() && !in_residual => continue,

            // Other elements are preserved without an item number.
            _ => {
                bibliography.add_residual(element);
                in_residual = true;
            }
        }
    }

    // Add bibliography object to parser for unified tracking, like footnotes.
    let index = parser.push_bibliography(bibliography);

    ok!(Element::BibliographyBlock { index, title, hide }, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn bibliography_block_collects_definition_items_and_options() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(
            "[[bibliography title=\"Works\" hide=\"true\"]]\n: alpha : Alpha reference\n[[/bibliography]]",
        );
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty());
        match tree.elements.as_slice() {
            [Element::BibliographyBlock { index, title, hide }] => {
                assert_eq!(*index, 0);
                assert_eq!(title.as_deref(), Some("Works"));
                assert!(*hide);
            }
            other => panic!("expected bibliography block, got {other:?}"),
        }

        let (reference_index, reference_elements) = tree
            .bibliographies
            .get_reference("alpha")
            .expect("bibliography reference should be stored");
        assert_eq!(reference_index, 1);
        assert_eq!(
            reference_elements,
            [text!("Alpha"), text!(" "), text!("reference")]
        );
    }

    #[test]
    fn bibliography_block_preserves_non_definition_body() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization =
            crate::tokenize("[[bibliography]]\nnot a definition\n[[/bibliography]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:#?}");
        let bibliography = tree.bibliographies.get_bibliography(0);
        assert_eq!(bibliography.slice().len(), 1, "{:#?}", bibliography.slice(),);
        assert!(Bibliography::is_residual(&bibliography.slice()[0].0));
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        assert_eq!(
            html,
            "<div class=\"bibitems\"><div class=\"title\">Bibliography</div>\nnot a definition</div>",
        );
    }

    #[test]
    fn wikidot_bibliography_renders_legacy_dom_and_self_citations() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let mut source = concat!(
            "[[bibliography title=\"Works\" hide=\"true\"]]\n",
            ": alpha : ((bibcite alpha))\n",
            "[[/bibliography]]\n",
            "((bibcite alpha))",
        )
        .to_owned();
        crate::preprocess(&mut source);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains(r#"<div class="bibitems">"#), "{html}");
        assert!(html.contains(r#"<div class="title">Works</div>"#), "{html}");
        assert!(
            html.contains(r#"<div class="bibitem" id="bibitem-1">1. "#),
            "{html}",
        );
        assert_eq!(html.matches(r#"class="bibcite""#).count(), 2, "{html}");
        assert!(!html.contains("wj-bibliography"), "{html}");
        assert!(!html.contains("error-inline"), "{html}");
    }

    #[test]
    fn wikidot_unclosed_bibliography_opener_remains_literal() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[bibliography]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html, "<p>[[bibliography]]</p>");
    }

    #[test]
    fn wikidot_unclosed_bibliography_after_footnote_remains_literal() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(
            "A claim[[footnote]]with a note[[/footnote]].\n\n[[bibliography]]",
        );
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            "<p>A claim<sup class=\"footnoteref\"><a id=\"footnoteref-1\" href=\"javascript:;\" class=\"footnoteref\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnote-1&#39;)\">1</a></sup>.</p><p>[[bibliography]]</p><div class=\"footnotes-footer\"><div class=\"title\">Footnotes</div><div class=\"footnote-footer\" id=\"footnote-1\"><a href=\"javascript:;\" onclick=\"WIKIDOT.page.utils.scrollToReference(&#39;footnoteref-1&#39;)\">1</a>. with a note</div></div>",
        );
    }
}
