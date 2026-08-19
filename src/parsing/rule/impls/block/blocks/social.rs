/*
 * parsing/rule/impls/block/blocks/social.rs
 *
 * ftml - Library to parse Wikidot text
 * Copyright (C) 2019-2026 Wikijump Team
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 */

use super::prelude::*;
use crate::tree::SocialButtons;

pub const BLOCK_SOCIAL: BlockRule = BlockRule {
    name: "block-social",
    accepts_names: &["social"],
    accepts_star: false,
    accepts_score: false,
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
    debug!("Parsing social block (name '{name}', in-head {in_head})");
    assert!(!flag_star, "Social doesn't allow star flag");
    assert!(!flag_score, "Social doesn't allow score flag");
    assert_block_name(&BLOCK_SOCIAL, name);
    parser.check_page_syntax()?;
    if !parser.settings().layout.legacy() {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    if !in_head {
        return success_elements(Element::SocialButtons(SocialButtons::parse("")));
    }

    let start = parser.current().span.start;
    loop {
        let current = parser.current();
        let trailing_bracket = match current.token {
            Token::RightBlock => false,
            Token::RightLink if current.slice == "]]]" => true,
            Token::LineBreak | Token::ParagraphBreak => {
                return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments));
            }
            Token::InputEnd => return Err(parser.make_err(ParseErrorKind::EndOfInput)),
            _ => {
                parser.step()?;
                continue;
            }
        };

        let head = &parser.full_text().inner()[start..current.span.start];
        let social = Element::SocialButtons(SocialButtons::parse(head));
        parser.step()?;
        return if trailing_bracket {
            ok!(Elements::Multiple(vec![social, text!("]")]))
        } else {
            success_elements(social)
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};
    use crate::tree::{Element, SocialSelection, SocialService};

    fn parse_social(input: &str) -> Element<'static> {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokens = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokens, &page_info, &settings).into();
        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(tree.elements.len(), 1, "{:#?}", tree.elements);
        match &tree.elements[0] {
            Element::SocialButtons(_) => tree.elements[0].to_owned(),
            Element::Container(container) => {
                let [Element::SocialButtons(social)] = container.elements() else {
                    panic!(
                        "social paragraph did not contain exactly one typed request: {container:#?}"
                    );
                };
                Element::SocialButtons(social.clone())
            }
            element => panic!("social block did not stay typed: {element:#?}"),
        }
    }

    #[test]
    fn wikidot_social_selection_matches_live_token_boundaries() {
        let Element::SocialButtons(default) = parse_social("[[social]]") else {
            panic!("social block did not stay typed");
        };
        assert_eq!(default.selection(), None);

        let Element::SocialButtons(selected) =
            parse_social("[[social reddit, not-a-service,facebook]]")
        else {
            panic!("social block did not stay typed");
        };
        assert_eq!(
            selected.selection(),
            Some(
                &[
                    SocialSelection::Service(SocialService::Reddit),
                    SocialSelection::Service(SocialService::Facebook),
                ][..]
            )
        );

        let Element::SocialButtons(invalid) = parse_social("[[social Reddit,FACEBOOK]]")
        else {
            panic!("social block did not stay typed");
        };
        assert_eq!(
            invalid.selection(),
            Some(&[SocialSelection::Empty, SocialSelection::Empty][..])
        );
    }
}
