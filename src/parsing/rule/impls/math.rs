/*
 * parsing/rule/impls/math.rs
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

use super::block::{RULE_BLOCK, get_block_rule_with_name_for_layout};
use super::link_triple::{RULE_LINK_TRIPLE, RULE_LINK_TRIPLE_NEW_TAB};
use super::prelude::*;
use super::raw::RULE_RAW;
use crate::parsing::collect::consume_valid_comment;
use crate::parsing::parser::closer_scan::count_scan_token_visit;
use crate::parsing::{discard_wikidot_controls, trim_wikidot_ascii_cow};
use crate::tree::PartialElement;
use std::borrow::Cow;

pub const RULE_MATH: Rule = Rule {
    name: "math",
    position: LineRequirement::Any,
    try_consume_fn,
};

#[derive(Debug)]
struct WikidotMathSource<'t> {
    latex_source: Cow<'t, str>,
    residual_closer: &'t str,
    closed_outer_span: bool,
    closed_outer_size: bool,
    closed_outer_anchor: bool,
}

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create inline math equation");
    assert_step(parser, Token::LeftMath)?;
    let WikidotMathSource {
        latex_source,
        residual_closer,
        closed_outer_span,
        closed_outer_size,
        closed_outer_anchor,
    } = if parser.settings().layout.legacy() {
        collect_wikidot_math_source(parser)?
    } else {
        let close = [ParseCondition::current(Token::RightMath)];
        let invalid = [
            ParseCondition::current(Token::ParagraphBreak),
            ParseCondition::current(Token::LineBreak),
        ];
        WikidotMathSource {
            latex_source: Cow::Borrowed(
                collect_text(parser, RULE_MATH, &close, &invalid, None)?.trim(),
            ),
            residual_closer: "",
            closed_outer_span: false,
            closed_outer_size: false,
            closed_outer_anchor: false,
        }
    };

    let element = Element::MathInline { latex_source };
    if residual_closer.is_empty()
        && !closed_outer_span
        && !closed_outer_size
        && !closed_outer_anchor
    {
        success_elements(element)
    } else {
        let mut out = vec![element];
        if !residual_closer.is_empty() {
            out.push(text!(residual_closer));
        }
        if closed_outer_span {
            out.push(Element::Partial(PartialElement::InlineSpanClose(cow!(
                "[[/span]]"
            ))));
        }
        if closed_outer_size {
            out.push(Element::Partial(PartialElement::InlineSizeClose(cow!(
                "[[/size]]"
            ))));
        }
        if closed_outer_anchor {
            out.push(Element::Partial(PartialElement::InlineAnchorClose(cow!(
                "[[/a]]"
            ))));
        }
        success_elements(out)
    }
}

fn collect_wikidot_math_source<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Result<WikidotMathSource<'t>, ParseError>
where
    'r: 't,
{
    let scan_start = parser.current_token_index();
    // A scan from an earlier `[[$` opener that stepped to `InputEnd` proved
    // that no `RightMath` exists at or after it, and that the walk was a plain
    // single-token walk (it never consumed a comment or a raw region, which are
    // the only ways this loop jumps over tokens). Every later opener therefore
    // walks the same way to the same `InputEnd` and returns the identical
    // `EndOfInput` error; see `parsing/parser/closer_scan.rs`.
    //
    // A discarding hidden body commits the failed attempt when it lands on the
    // body's end-block boundary, and the full walk lands there on `InputEnd`
    // while this shortcut would stay on the opener, so those scans keep
    // running for real.
    if !parser.discarding_hidden_body() && parser.math_close_absent(scan_start) {
        return Err(parser.make_end_of_input_err());
    }

    let source = parser.full_text().inner();
    let start = parser.current().span.start;
    let mut segment_start = start;
    let mut comment_elided = None::<String>;
    let mut suppress_formula = false;
    let mut span_open = false;
    let mut size_open = false;
    let mut a_open = false;
    let in_span = parser.in_wikidot_span_body();
    let in_size = parser.in_wikidot_size_body();
    let in_anchor = parser.in_wikidot_anchor_body();
    let mut closed_outer_span = false;
    let mut closed_outer_size = false;
    let mut closed_outer_anchor = false;
    // Only needed to decide whether this walk may be memoized.
    let mut saw_jump = false;

    loop {
        count_scan_token_visit();
        match parser.current().token {
            Token::RightMath => {
                let end = parser.current().span.start;
                parser.step()?;
                if !suppress_formula
                    && (span_open || size_open || a_open)
                    && wikidot_math_tail_closes_open_inline(
                        parser, span_open, size_open, a_open,
                    )
                {
                    suppress_formula = true;
                }
                if suppress_formula {
                    return Ok(WikidotMathSource {
                        latex_source: Cow::Borrowed(""),
                        residual_closer: "",
                        closed_outer_span,
                        closed_outer_size,
                        closed_outer_anchor,
                    });
                }
                let latex_source = match comment_elided {
                    Some(mut formula) => {
                        formula.push_str(&source[segment_start..end]);
                        trim_wikidot_ascii_cow(discard_wikidot_controls(Cow::Owned(
                            formula,
                        )))
                    }
                    None => trim_wikidot_ascii_cow(Cow::Borrowed(&source[start..end])),
                };
                return Ok(WikidotMathSource {
                    latex_source,
                    residual_closer: "",
                    closed_outer_span,
                    closed_outer_size,
                    closed_outer_anchor,
                });
            }

            Token::ParagraphBreak => {
                return Err(parser.make_err(ParseErrorKind::RuleFailed));
            }
            Token::InputEnd => {
                if !saw_jump {
                    parser.record_math_close_absent(scan_start);
                }
                return Err(parser.make_err(ParseErrorKind::EndOfInput));
            }

            Token::RuntimeText | Token::GeneratedPageLink | Token::GeneratedTagLinks => {
                return Err(parser.make_err(ParseErrorKind::RuleFailed));
            }

            Token::DiscardedControl => {
                let control_start = parser.current().span.start;
                comment_elided
                    .get_or_insert_with(String::new)
                    .push_str(&source[segment_start..control_start]);
                parser.step()?;
                segment_start = parser.current().span.start;
            }

            Token::LeftComment => {
                saw_jump = true;
                let comment_start = parser.current().span.start;
                let mut comment = parser.clone();
                if consume_valid_comment(&mut comment).is_err() {
                    parser.step()?;
                    continue;
                }

                comment_elided
                    .get_or_insert_with(String::new)
                    .push_str(&source[segment_start..comment_start]);
                parser.update(&comment);
                segment_start = parser.current().span.start;

                let formula = comment_elided
                    .as_mut()
                    .expect("valid comments initialize the elided formula");
                if let Some((visible_close_bytes, current_close_bytes)) =
                    comment_joined_math_close(parser, formula)
                {
                    formula.truncate(formula.len() - visible_close_bytes);
                    let residual_closer = &parser.current().slice[current_close_bytes..];
                    parser.step()?;
                    let latex_source = if suppress_formula {
                        Cow::Borrowed("")
                    } else {
                        let cleaned =
                            discard_wikidot_controls(Cow::Borrowed(formula.as_str()));
                        Cow::Owned(
                            crate::parsing::trim_wikidot_ascii(&cleaned).to_owned(),
                        )
                    };
                    return Ok(WikidotMathSource {
                        latex_source,
                        residual_closer,
                        closed_outer_span,
                        closed_outer_size,
                        closed_outer_anchor,
                    });
                }
            }

            Token::LineBreak => {
                suppress_formula = true;
                parser.step()?;
            }

            Token::LeftLink | Token::LeftLinkStar => {
                let rule = if parser.current().token == Token::LeftLinkStar {
                    RULE_LINK_TRIPLE_NEW_TAB
                } else {
                    RULE_LINK_TRIPLE
                };
                let mut link = parser.clone();
                if rule.try_consume(&mut link).is_ok() {
                    suppress_formula = true;
                }
                parser.step()?;
            }

            Token::LeftBlock => {
                if wikidot_image_block_opens(parser) {
                    suppress_formula = true;
                    parser.mark_wikidot_math_image_occupied();
                } else if let Some((name, actionable)) = wikidot_math_inline_block_open(parser)
                    && actionable
                {
                    span_open |= name.eq_ignore_ascii_case("span");
                    size_open |= name.eq_ignore_ascii_case("size");
                    a_open |= name.eq_ignore_ascii_case("a");
                }
                parser.step()?;
            }

            Token::LeftBlockEnd => {
                let mut close = parser.clone();
                if let Ok((name, _)) = close.get_wikidot_end_block_with_residual() {
                    if !span_open && in_span && name.eq_ignore_ascii_case("span") {
                        suppress_formula = true;
                        closed_outer_span = true;
                    } else if !size_open && in_size && name.eq_ignore_ascii_case("size") {
                        suppress_formula = true;
                        closed_outer_size = true;
                    } else if !a_open && in_anchor && name.eq_ignore_ascii_case("a") {
                        suppress_formula = true;
                        closed_outer_anchor = true;
                    } else if (span_open && name.eq_ignore_ascii_case("span"))
                        || (size_open && name.eq_ignore_ascii_case("size"))
                        || (a_open && name.eq_ignore_ascii_case("a"))
                    {
                        suppress_formula = true;
                    }
                }
                parser.step()?;
            }

            Token::RightBlock
                if comment_elided.is_none()
                    && parser.current().slice == "]]"
                    && parser.current().span.start > start
                    && source.as_bytes()[parser.current().span.start - 1] == b'$' =>
            {
                let end = parser.current().span.start;
                parser.step()?;
                let latex_source = if suppress_formula {
                    Cow::Borrowed("")
                } else {
                    trim_wikidot_ascii_cow(Cow::Borrowed(&source[start..end - 1]))
                };
                return Ok(WikidotMathSource {
                    latex_source,
                    residual_closer: "",
                    closed_outer_span,
                    closed_outer_size,
                    closed_outer_anchor,
                });
            }

            Token::Raw => {
                saw_jump = true;
                let mut raw = parser.clone();
                if RULE_RAW.try_consume(&mut raw).is_err() {
                    parser.step()?;
                    continue;
                }

                let raw_end = raw.current().span.start;
                let mut owner = parser.clone();
                while owner.current().span.start < raw_end {
                    if owner.current().token == Token::RightMath {
                        return Err(parser.make_err(ParseErrorKind::RuleFailed));
                    }
                    if owner.current_generated().is_some()
                        || matches!(
                            owner.current().token,
                            Token::RuntimeText
                                | Token::GeneratedPageLink
                                | Token::GeneratedTagLinks
                        )
                    {
                        return Err(parser.make_err(ParseErrorKind::RuleFailed));
                    }
                    owner.step()?;
                }

                suppress_formula = true;
                parser.update(&raw);
            }

            _ => {
                parser.step()?;
            }
        }
    }
}

fn wikidot_math_inline_block_open<'r, 't>(
    parser: &Parser<'r, 't>,
) -> Option<(String, bool)>
where
    'r: 't,
{
    let mut opener = parser.clone();
    let (name, in_head, segment) = opener.get_block_name(false).ok()?;
    let name_owned = name.into_owned();
    let borrowed = matches!(
        name_owned.as_str(),
        "span" | "size" | "a" | "SPAN" | "SIZE" | "A"
    );
    if in_head {
        loop {
            match opener.current().token {
                Token::RightBlock => {
                    let _ = opener.step();
                    break;
                }
                Token::RightLink if opener.current().slice == "]]]" => {
                    let _ = opener.step();
                    break;
                }
                Token::RightBracket
                | Token::RightLink
                | Token::LineBreak
                | Token::ParagraphBreak
                | Token::InputEnd => return None,
                _ => {
                    opener.step().ok()?;
                }
            }
        }
    }
    let actionable = opener.head_word_is_actionable(&name_owned, borrowed, segment);
    Some((name_owned, actionable))
}

fn wikidot_math_tail_closes_open_inline<'r, 't>(
    parser: &Parser<'r, 't>,
    span_open: bool,
    size_open: bool,
    a_open: bool,
) -> bool
where
    'r: 't,
{
    let mut scan = parser.clone();
    loop {
        match scan.current().token {
            Token::LeftBlockEnd => {
                let mut close = scan.clone();
                if let Ok((name, _)) = close.get_wikidot_end_block_with_residual() {
                    if (span_open && name.eq_ignore_ascii_case("span"))
                        || (size_open && name.eq_ignore_ascii_case("size"))
                        || (a_open && name.eq_ignore_ascii_case("a"))
                    {
                        return true;
                    }
                    scan.update(&close);
                    continue;
                }
            }
            Token::ParagraphBreak | Token::InputEnd => return false,
            _ => {}
        }
        if scan.step().is_err() {
            return false;
        }
    }
}

fn wikidot_image_block_opens<'r, 't>(parser: &Parser<'r, 't>) -> bool
where
    'r: 't,
{
    let Some(name) = parser.block_head_name_here() else {
        return false;
    };
    if !matches!(
        get_block_rule_with_name_for_layout(&name, parser.settings().layout),
        Some(rule) if rule.name() == "block-image"
    ) {
        return false;
    }
    let mut block = parser.clone();
    let Ok(parsed) = RULE_BLOCK.try_consume(&mut block) else {
        return false;
    };
    parsed
        .item
        .as_ref()
        .iter()
        .any(|element| matches!(element, Element::Image { .. }))
}

fn comment_joined_math_close<'r, 't>(
    parser: &Parser<'r, 't>,
    visible: &str,
) -> Option<(usize, usize)>
where
    'r: 't,
{
    if parser.current_generated().is_some()
        || !matches!(
            parser.current().token,
            Token::RightBracket | Token::RightBlock | Token::RightLink
        )
    {
        return None;
    }

    let current = parser.current().slice.as_bytes();
    if visible.ends_with("$]") && current.starts_with(b"]") {
        Some((2, 1))
    } else if visible.ends_with('$') && current.starts_with(b"]]") {
        Some((1, 2))
    } else {
        None
    }
}

pub(super) fn wikidot_math_candidate_is_complete<'r, 't>(parser: &Parser<'r, 't>) -> bool
where
    'r: 't,
{
    if parser.current().token != Token::LeftMath {
        return false;
    }

    let mut scan = parser.clone();
    if scan.step().is_err() {
        return false;
    }
    collect_wikidot_math_source(&mut scan).is_ok()
}
