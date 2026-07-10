/*
 * parsing/rule/impls/block/blocks/math.rs
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

pub const BLOCK_MATH: BlockRule = BlockRule {
    name: "block-math",
    accepts_names: &["math"],
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
    debug!("Parsing math block (name '{name}', in-head {in_head})");
    assert!(!flag_star, "User doesn't allow star flag");
    assert!(!flag_score, "User doesn't allow score flag");
    assert_block_name(&BLOCK_MATH, name);

    let name = parser.get_head_value(&BLOCK_MATH, in_head, |_, value| {
        Ok(value.map(|s| std::borrow::Cow::Borrowed(s.trim())))
    })?;

    let latex_source = match parser.get_body_text(&BLOCK_MATH)? {
        Cow::Borrowed(source) => Cow::Borrowed(source.trim()),
        Cow::Owned(source) => Cow::Owned(source.trim().to_owned()),
    };
    if latex_source.is_empty() {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let element = Element::Math { name, latex_source };

    success_elements(element)
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn quoted_math_block_trims_owned_source() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(concat!(
            "> [[collapsible]]\n",
            "> [[math]]\n",
            ">   x + y   \n",
            "> [[/math]]\n",
            "> [[/collapsible]]\n",
        ));
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        assert!(format!("{tree:?}").contains("x + y"));
    }
}
