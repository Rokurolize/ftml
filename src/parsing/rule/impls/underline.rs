/*
 * parsing/rule/impls/underline.rs
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

use super::inline_delimiter::assert_unpadded_open;
use super::prelude::*;

pub const RULE_UNDERLINE: Rule = Rule {
    name: "underline",
    position: LineRequirement::Any,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create underline container");
    assert_unpadded_open(parser, Token::Underline)?;
    let close = [ParseCondition::current(Token::Underline)];
    let invalid = [
        ParseCondition::current(Token::ParagraphBreak),
        ParseCondition::token_pair(Token::Underline, Token::Whitespace),
        ParseCondition::token_pair(Token::Whitespace, Token::Underline),
    ];
    let ctype = ContainerType::Underline;
    let collected =
        collect_container(parser, RULE_UNDERLINE, ctype, &close, &invalid, None)?;
    let (elements, errors, paragraph_safe) = collected.into();
    if parser.settings().layout.legacy()
        && matches!(
            &elements,
            Elements::Single(Element::Container(container)) if container.elements().is_empty()
        )
    {
        return ok!(paragraph_safe; Elements::None, errors);
    }
    ok!(paragraph_safe; elements, errors)
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn wikidot_discards_complete_empty_underline_pairs_in_runs() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        for (input, expected) in [
            ("______[[/span]]", "<p>__[[/span]]</p>"),
            ("______x", "<p>__x</p>"),
            ("x________y", "<p>xy</p>"),
            ("______ ______", "<p>__ __</p>"),
        ] {
            let mut source = input.to_owned();
            crate::preprocess(&mut source);
            let tokenization = crate::tokenize(&source);
            let (tree, _errors) =
                crate::parse(&tokenization, &page_info, &settings).into();
            let html = HtmlRender.render(&tree, &page_info, &settings).body;
            assert_eq!(html, expected, "{input:?}");
        }
    }
}
