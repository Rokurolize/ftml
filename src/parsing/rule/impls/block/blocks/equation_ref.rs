/*
 * parsing/rule/impls/block/blocks/equation_ref.rs
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
use std::borrow::Cow;

pub const BLOCK_EQUATION_REF: BlockRule = BlockRule {
    name: "block-equation-ref",
    accepts_names: &["equation", "eref", "eqref"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines_outside_body: false,
    parse_fn,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &str,
    _opener_start: usize,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Parsing equation reference block (name '{name}', in-head {in_head})");
    assert!(!flag_star, "Equation reference doesn't allow start flag");
    assert!(!flag_score, "Equation reference doesn't allow score flag");
    assert_block_name(&BLOCK_EQUATION_REF, name);
    if parser.settings().layout.legacy() && name != "eref" {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let block = &BLOCK_EQUATION_REF;
    let checkpoint = parser.clone();
    let name = match parser.get_head_value_resolved(
        block,
        in_head,
        require_trimmed_block_argument_resolved,
    ) {
        Ok(name) => name,
        Err(error) => {
            if !parser.settings().layout.legacy() || !in_head {
                return Err(error);
            }
            parser.update(&checkpoint);
            match wikidot_freelink_head_value(parser) {
                Some(name) => name,
                None => return Err(error),
            }
        }
    };

    success_elements(Element::EquationReference(name))
}

fn wikidot_freelink_head_value<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Option<Cow<'t, str>>
where
    'r: 't,
{
    let source = parser.full_text().inner();
    let start = parser.current().span.start;
    let mut attempt = parser.clone();
    let mut depth = 0usize;
    let close_end;
    loop {
        let token = attempt.current();
        match token.token {
            Token::LeftLink => {
                depth += 1;
                attempt.step().ok()?;
            }
            Token::RightLink => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                attempt.step().ok()?;
            }
            Token::RightBlock => {
                if depth != 0 {
                    return None;
                }
                close_end = token.span.start;
                attempt.step().ok()?;
                break;
            }
            Token::RightBracket => {
                if depth != 0 {
                    return None;
                }
                let first_end = token.span.end;
                attempt.step().ok()?;
                let next = attempt.current();
                if next.token != Token::RightBracket || next.span.start != first_end {
                    return None;
                }
                close_end = token.span.start;
                attempt.step().ok()?;
                break;
            }
            Token::LineBreak | Token::ParagraphBreak | Token::InputEnd => return None,
            _ => {
                attempt.step().ok()?;
            }
        }
    }
    let value = &source[start..close_end];
    if !value.contains("[[[") {
        return None;
    }
    let name =
        require_trimmed_block_argument_resolved(&attempt, Some(Cow::Borrowed(value)))
            .ok()?;
    parser.update(&attempt);
    Some(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn equation_reference_parses_name_and_requires_argument() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);

        let tokenization = crate::tokenize("[[eqref theorem-one]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        assert!(errors.is_empty(), "{errors:?}");
        assert!(matches!(
            tree.elements.as_slice(),
            [Element::Container(paragraph)]
                if paragraph.elements().iter().any(|element| matches!(
                    element,
                    Element::EquationReference(name) if name.as_ref() == "theorem-one"
                ))
        ));

        let tokenization = crate::tokenize("[[eref]]");
        let (_tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        assert!(
            errors
                .iter()
                .any(|error| error.kind() == ParseErrorKind::BlockMissingArguments)
        );
    }
}
