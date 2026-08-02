use super::prelude::*;
use crate::delayed::{DelayedElement, GeneratedKind};

pub const RULE_DELAYED: Rule = Rule {
    name: "delayed",
    position: LineRequirement::Any,
    try_consume_fn,
};

pub const RULE_DELAYED_CONDITIONAL: Rule = Rule {
    name: "delayed-conditional",
    position: LineRequirement::Any,
    try_consume_fn: try_consume_conditional,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    let Some(generated) = parser.current_generated().cloned() else {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    };
    parser.step()?;
    ok!(Element::Delayed(DelayedElement::active(
        generated.id,
        generated.kind
    )))
}

fn try_consume_conditional<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    assert_step(parser, Token::LeftBlockAnchor)?;
    if parser.current().token != Token::Identifier
        || !parser.current().slice.eq_ignore_ascii_case("if")
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    parser.step()?;
    parser.get_token(Token::Whitespace, ParseErrorKind::RuleFailed)?;

    let condition_start = parser.current().span.start;
    while !matches!(
        parser.current().token,
        Token::Pipe
            | Token::RightBlock
            | Token::LineBreak
            | Token::ParagraphBreak
            | Token::InputEnd
    ) {
        if parser.current_generated().is_some() {
            return Err(parser.make_err(ParseErrorKind::RuleFailed));
        }
        parser.step()?;
    }
    if parser.current().token != Token::Pipe {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    let condition =
        parser.full_text().inner()[condition_start..parser.current().span.start].trim();
    if !condition.eq_ignore_ascii_case("true") {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    parser.step()?;

    let true_start = parser.current().span.start;
    let mut generated = None;
    while !matches!(
        parser.current().token,
        Token::Pipe
            | Token::RightBlock
            | Token::LineBreak
            | Token::ParagraphBreak
            | Token::InputEnd
    ) {
        if let Some(slot) = parser.current_generated().cloned()
            && generated.replace(slot).is_some()
        {
            return Err(parser.make_err(ParseErrorKind::RuleFailed));
        }
        parser.step()?;
    }
    if parser.current().token != Token::Pipe {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    let true_end = parser.current().span.start;
    let generated =
        generated.ok_or_else(|| parser.make_err(ParseErrorKind::RuleFailed))?;
    let source = parser.full_text().inner();
    if !source[true_start..generated.source_range.start]
        .trim()
        .is_empty()
        || !source[generated.source_range.end..true_end]
            .trim()
            .is_empty()
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    parser.step()?;

    let false_start = parser.current().span.start;
    while !matches!(
        parser.current().token,
        Token::RightBlock | Token::LineBreak | Token::ParagraphBreak | Token::InputEnd
    ) {
        if parser.current_generated().is_some() {
            return Err(parser.make_err(ParseErrorKind::RuleFailed));
        }
        parser.step()?;
    }
    if parser.current().token != Token::RightBlock {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    let false_branch = source[false_start..parser.current().span.start].trim();
    parser.step()?;

    let element = match generated.kind {
        GeneratedKind::PageLink => {
            DelayedElement::page_conditional_recovery(generated.id, false_branch)
        }
        GeneratedKind::TagLinks => DelayedElement::active(generated.id, generated.kind),
    };
    ok!(Element::Delayed(element))
}
