/*
 * parsing/rule/impls/header.rs
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
use crate::tree::Heading;
use std::convert::TryInto;

pub const RULE_HEADER: Rule = Rule {
    name: "header",
    position: LineRequirement::StartOfLine,
    try_consume_fn,
};

fn step_expected<'r, 't>(
    parser: &mut Parser<'r, 't>,
    token: Token,
) -> Result<&'r ExtractedToken<'t>, ParseError> {
    let current = parser.current();
    if current.token != token {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    parser.step()?;
    Ok(current)
}

fn heading_from_token(token: &ExtractedToken<'_>) -> Heading {
    token
        .slice
        .try_into()
        .expect("Received invalid heading length token slice")
}

fn consume_header_once<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create header container");

    // Get header depth
    let heading_token = step_expected(parser, Token::Heading)?;
    let heading = heading_from_token(heading_token);

    // Step over whitespace
    step_expected(parser, Token::Whitespace)?;

    let close = [
        ParseCondition::current(Token::InputEnd),
        ParseCondition::current(Token::LineBreak),
        ParseCondition::current(Token::ParagraphBreak),
    ];
    let ctype = ContainerType::Header(heading);
    let collected = collect_container(parser, RULE_HEADER, ctype, &close, &[], None)?;
    let (elements, all_errors, _) = collected.into();

    // If this heading wants a table of contents (TOC) entry, then add one
    if heading.has_toc
        && let Elements::Single(Element::Container(ref container)) = elements
    {
        parser.push_table_of_contents_entry(heading.level, container.elements());
    }

    // Build final Elements object
    ok!(false; elements, all_errors)
}

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    let success = consume_header_once(parser)?;
    let (elements, mut all_errors, _) = success.into();
    let mut all_elements: Vec<_> = elements.into_iter().collect();

    loop {
        let parser_state = parser.get_mutable_state();
        let mut sub_parser = parser.clone_with_rule(RULE_HEADER);

        let success = match consume_header_once(&mut sub_parser) {
            Ok(success) => success,
            Err(_) => {
                parser.reset_mutable_state(parser_state);
                return ok!(false; all_elements, all_errors);
            }
        };

        parser.update(&sub_parser);

        let (elements, mut errors, _) = success.into();
        all_elements.extend(elements);
        all_errors.append(&mut errors);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};
    use crate::tree::{ContainerType, Element, HeadingLevel};

    #[test]
    fn header_rule_collects_adjacent_headings_and_toc_flags() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("+ One\n++* Two\nplain");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(tree.table_of_contents.len(), 1);
        assert_eq!(tree.elements.len(), 3);

        let Element::Container(first) = &tree.elements[0] else {
            panic!("expected first heading, got {:?}", tree.elements[0]);
        };
        assert_eq!(
            first.ctype(),
            ContainerType::Header(Heading {
                level: HeadingLevel::One,
                has_toc: true,
            }),
        );

        let Element::Container(second) = &tree.elements[1] else {
            panic!("expected second heading, got {:?}", tree.elements[1]);
        };
        assert_eq!(
            second.ctype(),
            ContainerType::Header(Heading {
                level: HeadingLevel::Two,
                has_toc: false,
            }),
        );
    }
}
