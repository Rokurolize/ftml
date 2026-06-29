/*
 * parsing/rule/impls/block/blocks/span.rs
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

pub const BLOCK_SPAN: BlockRule = BlockRule {
    name: "block-span",
    accepts_names: &["span"],
    accepts_star: false,
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
    debug!("Parsing span block (name '{name}', in-head {in_head})");
    assert!(!flag_star, "Span doesn't allow star flag");
    assert_block_name(&BLOCK_SPAN, name);

    let arguments = parser.get_head_map(&BLOCK_SPAN, in_head)?;

    // Get body content, without paragraphs
    let body = parser.get_body_elements(&BLOCK_SPAN, false)?;
    let (mut elements, errors, paragraph_safe) = body.into();

    if flag_score {
        strip_newlines(&mut elements);
    }

    let element = Element::Container(Container::new(
        ContainerType::Span,
        elements,
        arguments.to_attribute_map(parser.settings()),
    ));

    success_elements_with_paragraph_safety(paragraph_safe, element, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn score_span_strips_line_breaks_from_body() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization =
            crate::tokenize("[[span_ class=\"compact\"]]\nalpha\n[[/span]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let span = match tree.elements.as_slice() {
            [Element::Container(paragraph)] => paragraph
                .elements()
                .iter()
                .find_map(|element| match element {
                    Element::Container(container)
                        if container.ctype() == ContainerType::Span =>
                    {
                        Some(container)
                    }
                    _ => None,
                })
                .expect("paragraph should contain span container"),
            other => panic!("expected paragraph containing span, got {other:?}"),
        };

        assert_eq!(span.elements(), &[text!("alpha")]);
        assert_eq!(
            span.attributes()
                .get()
                .get("class")
                .map(|value| value.as_ref()),
            Some("compact"),
        );
    }
}
