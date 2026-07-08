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

    let mut arguments = parser.get_head_map(&BLOCK_BIBLIOGRAPHY, in_head)?;

    let title = arguments.get("title");
    let hide = arguments.get_bool(parser, "hide")?.unwrap_or(false);

    // Get body content. The contents should only be a definition list, but
    // we use the regular elements parser to make it easy on us. If we find
    // anything else, we fail the rule.
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

    for element in elements {
        match element {
            // Append definition list entries
            Element::DefinitionList(items) => {
                for item in items {
                    bibliography.add(item.key_string, item.value_elements);
                }
            }

            // Skip whitespace elements
            _ if element.is_whitespace() => continue,

            // Other elements
            _ => {
                warn!(
                    "Non-definition element in bibliography block: {}",
                    element.name(),
                );

                let kind = ParseErrorKind::BibliographyContainsNonDefinitionList;
                return Err(parser.make_err(kind));
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
    fn bibliography_block_rejects_non_definition_body() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization =
            crate::tokenize("[[bibliography]]\nnot a definition\n[[/bibliography]]");
        let (_tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(
            errors.iter().any(|error| error.kind()
                == ParseErrorKind::BibliographyContainsNonDefinitionList)
        );
    }
}
