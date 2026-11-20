/*
 * parsing/rule/impls/block/blocks/form.rs
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

pub const BLOCK_FORM: BlockRule = BlockRule {
    name: "block-form",
    accepts_names: &["form"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines_outside_body: false,
    parse_fn,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &str,
    opener_start: usize,
    flag_star: bool,
    flag_score: bool,
    _in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    assert!(!flag_star, "Form doesn't allow star flag");
    assert!(!flag_score, "Form doesn't allow score flag");
    assert_block_name(&BLOCK_FORM, name);

    let source = parser.full_text().inner();
    let opener = "[[form]]";
    if !source[opener_start..].starts_with(opener) {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    let after = source[opener_start + opener.len()..].trim_start_matches([' ', '\t']);
    if !after.starts_with(['\n', '\r']) {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    let line_start = source[..opener_start]
        .rfind(['\n', '\r'])
        .map_or(0, |index| index + 1);
    if !source[line_start..opener_start]
        .bytes()
        .all(|byte| matches!(byte, b' ' | b'\t'))
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let mut owner = parser.clone();
    let _ = owner.get_body_text(&BLOCK_FORM)?;
    let owner_end = owner.current().span.start.min(source.len());
    let block = source
        .get(opener_start..owner_end)
        .unwrap_or("")
        .trim_end_matches([' ', '\t', '\r', '\n']);

    let mut elements = Vec::new();
    for (index, line) in block.split('\n').enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if index > 0 {
            elements.push(Element::LineBreak);
        }
        if !line.is_empty() {
            elements.push(Element::Form(Cow::Borrowed(line)));
        }
    }

    parser.update(&owner);
    success_elements(Elements::Multiple(elements))
}
