/*
 * parsing/rule/impls/block/blocks/size.rs
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
use crate::tree::AttributeMap;
use std::borrow::Cow;

pub const BLOCK_SIZE: BlockRule = BlockRule {
    name: "block-size",
    accepts_names: &["size"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: false,
    parse_fn,
};

fn parse_size_argument<'r, 't>(
    parser: &Parser<'r, 't>,
    value: Option<&str>,
) -> Result<String, ParseError> {
    match value {
        Some(size) => Ok(format!("font-size: {};", safe_size_value(size))),
        None => Err(parser.make_err(ParseErrorKind::BlockMissingArguments)),
    }
}

fn safe_size_value(size: &str) -> &str {
    let size = size.trim();
    if !size.is_empty() && is_safe_size_value(size) {
        size
    } else {
        "inherit"
    }
}

fn is_safe_size_value(size: &str) -> bool {
    size.chars().all(|ch| {
        !ch.is_control()
            && !matches!(ch, ';' | '{' | '}' | '<' | '>' | '"' | '\'' | '\\' | '&')
    })
}

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Parsing size block (name '{name}', in-head {in_head})");
    assert!(!flag_star, "Size doesn't allow star flag");
    assert!(!flag_score, "Size doesn't allow score flag");
    assert_block_name(&BLOCK_SIZE, name);

    let size = parser.get_head_value(&BLOCK_SIZE, in_head, parse_size_argument)?;

    // Get body content, without paragraphs
    let body = parser.get_body_elements(&BLOCK_SIZE, false)?;
    let (elements, errors, paragraph_safe) = body.into();

    let mut attributes = AttributeMap::new();
    attributes.insert("style", Cow::Owned(size));

    let container = Container::new(ContainerType::Size, elements, attributes);
    let element = Element::Container(container);

    success_elements_with_paragraph_safety(paragraph_safe, element, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn size_block_requires_size_argument() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[size]]text[[/size]]");
        let (_tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(
            errors
                .iter()
                .any(|error| error.kind() == ParseErrorKind::BlockMissingArguments)
        );
    }

    #[test]
    fn size_block_wraps_body_with_style_attribute() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[size 80%]]small[[/size]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let [Element::Container(paragraph)] = tree.elements.as_slice() else {
            panic!("expected paragraph, got {:?}", tree.elements);
        };
        let [Element::Container(size)] = paragraph.elements() else {
            panic!("expected size container, got {:?}", paragraph.elements());
        };

        assert_eq!(size.ctype(), ContainerType::Size);
        assert_eq!(
            size.attributes()
                .get()
                .get("style")
                .map(|value| value.as_ref()),
            Some("font-size: 80%;"),
        );
        assert_eq!(size.elements(), &[text!("small")]);
    }
}
