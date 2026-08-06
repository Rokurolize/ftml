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
    let wikidot_candidate =
        parser.settings().layout.legacy() && !parser.discarding_hidden_body();
    let nested_wikidot_opener =
        wikidot_candidate && parser.in_wikidot_bibliography_body();
    let source = parser.full_text().inner();
    let name_start = (name.as_ptr() as usize)
        .checked_sub(source.as_ptr() as usize)
        .expect("parsed bibliography name belongs to the source");
    let opener_start = source[..name_start]
        .rfind("[[")
        .expect("parsed bibliography name follows its opener");

    let (mut arguments, _) =
        parser.get_head_map_with_body_start_wikidot(&BLOCK_BIBLIOGRAPHY, in_head)?;
    let opener_end = opener_start
        + source[opener_start..]
            .find("]]")
            .expect("parsed bibliography opener has closing brackets")
        + "]]".len();

    // A canonical bibliography nested directly in another bibliography is
    // literal. Wikidot also folds the one physical line break after that
    // marker into a space before the outer body continues.
    if nested_wikidot_opener {
        let opener = &source[opener_start..opener_end];
        if opener != "[[bibliography]]" {
            return Err(parser.make_err(ParseErrorKind::RuleFailed));
        }

        let mut elements = vec![Element::Text(cow!(opener))];
        if &source[opener_end..parser.current().span.start] == "\n" {
            elements.push(text!(" "));
        }
        return success_elements(elements);
    }

    let title = arguments.get("title");
    let hide = arguments.get_bool(parser, "hide")?.unwrap_or(false);

    // Wikidot commits a bibliography only after finding its closing marker.
    // Validate before body parsing so an EOF candidate cannot commit delayed
    // metadata or an invented bibliography container.
    if wikidot_candidate && !parser.has_body_end_block(&BLOCK_BIBLIOGRAPHY) {
        let body_start = parser.current().span.start;
        parser.enter_wikidot_bibliography_body();
        let body = parser.get_body_elements(&BLOCK_BIBLIOGRAPHY, false);
        parser.leave_wikidot_bibliography_body();
        let body = body?;
        let (mut body_elements, errors, _) = body.into();
        let mut literal = vec![Element::Text(cow!(&source[opener_start..opener_end]))];
        if &source[opener_end..body_start] == "\n" {
            literal.push(Element::LineBreak);
        }
        literal.append(&mut body_elements);
        return ok!(literal, errors);
    }

    // Get body content. Wikidot accepts non-definition content and renders it
    // directly inside the bibliography container.
    //
    // We also discard paragraph_safe, since it's not relevant, and this element
    // never is (uses <div>).
    let body_start = parser.current().span.start;
    if wikidot_candidate {
        parser.enter_wikidot_bibliography_body();
    }
    let body = parser.get_body_elements(&BLOCK_BIBLIOGRAPHY, false);
    if wikidot_candidate {
        parser.leave_wikidot_bibliography_body();
    }
    let body = body?;
    if wikidot_candidate && wikidot_bibliography_crosses_bold_owner(parser, body_start) {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
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

fn wikidot_bibliography_crosses_bold_owner(
    parser: &Parser<'_, '_>,
    body_start: usize,
) -> bool {
    let source = parser.full_text().inner();
    let owner_end = parser.current().span.start;
    let Some(closer_start) = source[body_start..owner_end].rfind("[[/") else {
        return false;
    };
    let body = &source[body_start..body_start + closer_start];
    if wikidot_unclosed_block_depth(body, "bold") == 0 {
        return false;
    }

    let suffix = source[owner_end..].trim_start_matches([' ', '\t', '\r', '\n', '\0']);
    let Some(close) = suffix.strip_prefix("[[/") else {
        return false;
    };
    let Some(end) = close.find("]]") else {
        return false;
    };
    close[..end].trim().eq_ignore_ascii_case("bold")
}

fn wikidot_unclosed_block_depth(source: &str, block_name: &str) -> usize {
    let mut depth = 0_usize;
    let mut remaining = source;

    while let Some(start) = remaining.find("[[") {
        remaining = &remaining[start + 2..];
        let Some(end) = remaining.find("]]") else {
            break;
        };
        let marker = remaining[..end].trim();
        let marker_name = marker.split_ascii_whitespace().next().unwrap_or_default();
        if marker_name.eq_ignore_ascii_case(block_name) {
            depth += 1;
        } else if marker_name
            .strip_prefix('/')
            .is_some_and(|name| name.eq_ignore_ascii_case(block_name))
        {
            depth = depth.saturating_sub(1);
        }
        remaining = &remaining[end + 2..];
    }

    depth
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
