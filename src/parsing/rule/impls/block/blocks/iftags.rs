/*
 * parsing/rule/impls/block/blocks/iftags.rs
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
use crate::data::PageInfo;
use crate::delayed::DelayedElement;
use crate::parsing::ElementCondition;

pub const BLOCK_IFTAGS: BlockRule = BlockRule {
    name: "block-iftags",
    accepts_names: &["iftags"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: true,
    parse_fn,
};

fn parse_conditions<'r, 't>(
    parser: &Parser<'r, 't>,
    spec: Option<&'t str>,
) -> Result<Vec<ElementCondition<'t>>, ParseError> {
    match spec {
        Some("-") if parser.settings().layout.legacy() => Ok(Vec::new()),
        Some(spec) if !parser.settings().layout.legacy() && spec.trim().is_empty() => {
            Err(parser.make_err(ParseErrorKind::BlockMissingArguments))
        }
        Some(spec) => Ok(ElementCondition::parse(spec)),
        None if parser.settings().layout.legacy() => Ok(Vec::new()),
        None => Err(parser.make_err(ParseErrorKind::BlockMissingArguments)),
    }
}

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Parsing iftags block (name '{name}', in-head {in_head})");
    assert!(!flag_star, "IfTags doesn't allow star flag");
    assert!(!flag_score, "IfTags doesn't allow score flag");
    assert_block_name(&BLOCK_IFTAGS, name);

    // Parse out tag conditions
    let (conditions, body_start) = parser.get_head_value_with_body_start(
        &BLOCK_IFTAGS,
        in_head,
        parse_conditions,
    )?;
    if parser.settings().layout.legacy()
        && !parser.discarding_hidden_body()
        && !parser.has_body_end_block(&BLOCK_IFTAGS)
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    if parser.settings().layout.legacy()
        && parser.discarding_hidden_body()
        && !parser.has_two_body_end_blocks(&BLOCK_IFTAGS)
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    // A closed no-argument gate on Wikidot passes exactly when the page
    // carries at least one tag. The frozen PagePreview parity bindings are
    // tag-less observations and bind the gate as false; on the saved-page
    // route every real bare-gate page on record is tagged and renders its
    // body. Keeping this explicit avoids changing ElementCondition's
    // general empty-set semantics.
    let include_body = if conditions.is_empty() && parser.settings().layout.legacy() {
        !parser.page_info().tags.is_empty()
    } else {
        check_iftags(parser.page_info(), &conditions)
    };
    if !include_body {
        trace!("Conditions failed, skipping hidden body");
        let hidden_start = parser.current().span.start;
        parser.discard_body_elements_with_literal_quote_context(
            &BLOCK_IFTAGS,
            body_start,
        )?;
        let hidden_end = parser.current().span.start;
        let generated = parser.generated_in_range(hidden_start..hidden_end);
        return ok!(true; Element::Delayed(DelayedElement::suppressed(&generated)), Vec::new());
    }

    // Get body content, never with paragraphs
    let body = parser.get_body_elements_with_literal_quote_context(
        &BLOCK_IFTAGS,
        false,
        body_start,
    )?;
    let (elements, errors, paragraph_safe) = body.into();

    trace!(
        "IfTags parsed {} conditions and {} elements",
        conditions.len(),
        elements.len(),
    );

    trace!("Conditions passed, including elements");
    let mut bounded = Vec::with_capacity(elements.len() + 2);
    bounded.push(Element::Delayed(DelayedElement::typography_boundary()));
    bounded.extend(elements);
    bounded.push(Element::Delayed(DelayedElement::typography_boundary()));
    ok!(paragraph_safe; Elements::Multiple(bounded), errors)
}

pub fn check_iftags(info: &PageInfo, conditions: &[ElementCondition]) -> bool {
    trace!("Checking iftags");
    ElementCondition::check(conditions, &info.tags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};
    use std::borrow::Cow;

    #[test]
    fn iftags_no_arguments_follows_page_tags_and_conditions_check_page_tags() {
        let legacy = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tagless = PageInfo {
            tags: Vec::new(),
            ..PageInfo::dummy()
        };
        // PageInfo::dummy() carries tags, like the saved pages on record.
        let tagged = PageInfo::dummy();
        assert!(!tagged.tags.is_empty());

        fn body_rendered(
            source: &str,
            info: &PageInfo,
            settings: &WikitextSettings,
        ) -> bool {
            let tokenization = crate::tokenize(source);
            let (tree, errors) = crate::parse(&tokenization, info, settings).into();
            assert!(errors.is_empty(), "{errors:?}");
            format!("{tree:?}").contains("body")
        }

        // A closed no-argument gate hides its body on a tag-less page (the
        // frozen PagePreview binding context) and renders it on a tagged
        // page (the saved-route evidence).
        for source in ["[[iftags]]body[[/iftags]]", "[[iftags   ]]body[[/iftags]]"] {
            assert!(!body_rendered(source, &tagless, &legacy), "{source}");
            assert!(body_rendered(source, &tagged, &legacy), "{source}");
        }

        // Sign-only specs: "+" parses to a required condition on an empty
        // tag name, which no page satisfies, so the body stays hidden with
        // or without tags. "-" normalizes to no conditions in the legacy
        // layout and follows the no-argument rule above. Both stay hidden
        // on a tag-less page, as the frozen bindings record.
        assert!(!body_rendered(
            "[[iftags +]]body[[/iftags]]",
            &tagless,
            &legacy
        ));
        assert!(!body_rendered(
            "[[iftags +]]body[[/iftags]]",
            &tagged,
            &legacy
        ));
        assert!(!body_rendered(
            "[[iftags -]]body[[/iftags]]",
            &tagless,
            &legacy
        ));
        assert!(body_rendered(
            "[[iftags -]]body[[/iftags]]",
            &tagged,
            &legacy
        ));

        // The Wikijump layout is unchanged: a missing or empty spec is
        // an error rather than an always-true condition.
        let modern = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        for source in ["[[iftags]]body[[/iftags]]", "[[iftags   ]]body[[/iftags]]"] {
            let tokenization = crate::tokenize(source);
            let (_tree, errors) = crate::parse(&tokenization, &tagged, &modern).into();
            assert!(
                errors
                    .iter()
                    .any(|error| error.kind() == ParseErrorKind::BlockMissingArguments),
                "{source}: {errors:?}",
            );
        }

        let info = PageInfo {
            tags: vec![Cow::Borrowed("alpha"), Cow::Borrowed("gamma")],
            ..PageInfo::dummy()
        };
        let included = ElementCondition::parse("+alpha -beta");
        let excluded = ElementCondition::parse("+alpha -gamma");

        assert!(check_iftags(&info, &included));
        assert!(!check_iftags(&info, &excluded));
    }
}
