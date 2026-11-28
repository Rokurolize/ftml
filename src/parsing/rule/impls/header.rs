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
use crate::parsing::ParseSuccess;
use crate::parsing::parser::block_pairs::WikidotBlockKind;
use crate::tree::{
    Container, Heading, HeadingReopen, WIKIDOT_HEADING_BARRIER_MARKER,
    WIKIDOT_HEADING_EMPTIED_MARKER,
};
use std::borrow::Cow;
use std::convert::TryInto;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum WikidotHeadingContent {
    Empty,
    FootnoteFirst,
    Other,
}

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

fn wikidot_control_before_unlabeled_footnote(source: &str, start: usize) -> bool {
    let line_start = source[..start.min(source.len())]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let line_end = source[line_start..]
        .find('\n')
        .map_or(source.len(), |index| line_start + index);
    let line = &source[line_start..line_end];
    let lowered = line.to_ascii_lowercase();
    let Some(opener) = lowered.find("[[footnote]]") else {
        return false;
    };
    let prefix = line[..opener]
        .trim_start_matches('+')
        .trim_start_matches([' ', '\t']);
    let mut saw_control = false;
    for character in prefix.chars() {
        if matches!(
            character,
            '\u{0000}' | '\u{0001}' | '\u{0008}' | '\u{001f}' | '\u{0085}'
        ) {
            saw_control = true;
        } else if !character.is_whitespace() {
            return false;
        }
    }
    saw_control
}

fn wikidot_crossed_heading_close_on_line(parser: &Parser<'_, '_>) -> bool {
    let source = parser.full_text().inner();
    let start = parser.current().span.start.min(source.len());
    let end = source[start..]
        .find('\n')
        .map_or(source.len(), |offset| start + offset);
    source[start..end].contains(crate::preproc::WIKIDOT_CROSSED_HEADING_CLOSE_SENTINEL)
}

fn consume_header_once<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create header container");

    let heading_token = step_expected(parser, Token::Heading)?;
    let heading = heading_from_token(heading_token);

    step_expected(parser, Token::Whitespace)?;
    while parser.current().token == Token::DiscardedControl {
        parser.step()?;
    }
    let content_start = parser.current().span.start;

    let close = [
        ParseCondition::current(Token::InputEnd),
        ParseCondition::current(Token::LineBreak),
        ParseCondition::current(Token::ParagraphBreak),
    ];
    let ctype = ContainerType::Header(heading);
    let restore_line = if parser.settings().layout.legacy() {
        let previous = (
            parser.in_wikidot_single_line_content(),
            parser.in_wikidot_heading_line(),
        );
        parser.set_in_wikidot_single_line_content(true);
        parser.set_in_wikidot_heading_line(true);
        Some(previous)
    } else {
        None
    };
    let crossed_close_on_line = parser.settings().layout.legacy()
        && wikidot_crossed_heading_close_on_line(parser);
    let collected = if parser.settings().layout.legacy()
        && parser.in_native_blockquote_line()
        && parser.current().token == Token::Other
        && parser.current().slice == "`"
        && crate::parsing::rule::impls::backtick_starts_rawold(parser)
    {
        let output = crate::parsing::rule::impls::RULE_RAWOLD.try_consume(parser)?;
        let (item, errors, _) = output.into();
        let body = match item {
            Elements::Single(element) => vec![element],
            Elements::Multiple(elements) => elements,
            Elements::None => Vec::new(),
        };
        let heading = Elements::Single(Element::Container(Container::new(
            ctype,
            body,
            AttributeMap::new(),
        )));
        Ok(ParseSuccess::new((heading, None), errors, false))
    } else {
        collect_wikidot_heading_with_barriers(parser, ctype, heading, &close)
    };
    if let Some((single_line, heading_line)) = restore_line {
        parser.set_in_wikidot_single_line_content(single_line);
        parser.set_in_wikidot_heading_line(heading_line);
    }
    let ((elements, toc_elements), mut all_errors, _) = collected?.into();
    let mut elements = elements;

    let content = match &elements {
        Elements::Single(Element::Container(container)) => {
            wikidot_heading_content(container.elements())
        }
        _ => unreachable!("header collection always returns one container"),
    };

    let control_owned_footnote = parser.settings().layout.legacy()
        && wikidot_control_before_unlabeled_footnote(
            parser.full_text().inner(),
            content_start,
        );
    if parser.settings().layout.legacy()
        && content == WikidotHeadingContent::FootnoteFirst
    {
        if !control_owned_footnote {
            return ok!(
                wikidot_literal_heading(heading_token.slice, elements),
                all_errors,
            );
        }
        if let Elements::Single(Element::Container(container)) = &mut elements {
            container.elements_mut().retain(|element| {
                !matches!(
                    element,
                    Element::Text(text) | Element::Raw(text)
                        if text.chars().all(char::is_whitespace)
                )
            });
        }
    }

    if parser.settings().layout.legacy()
        && content == WikidotHeadingContent::Empty
        && parser.current().token == Token::LeftBlock
        && parser
            .look_ahead(0)
            .is_some_and(|token| token.slice.eq_ignore_ascii_case("footnote"))
    {
        let mut candidate = parser.clone_with_rule(RULE_HEADER);
        if let Ok(footnote) = super::RULE_BLOCK.try_consume(&mut candidate)
            && let Elements::Single(Element::Footnote(index)) = footnote.item
        {
            if control_owned_footnote
                && let Elements::Single(Element::Container(container)) = &mut elements
            {
                container.elements_mut().retain(|element| {
                    !matches!(
                        element,
                        Element::Text(text) | Element::Raw(text)
                            if text.chars().all(char::is_whitespace)
                    )
                });
                container.elements_mut().push(Element::Footnote(index));
                all_errors.extend(footnote.errors);
                parser.update(&candidate);
            } else {
                let mut literal = wikidot_literal_heading(heading_token.slice, elements);
                literal.push(Element::Footnote(index));
                all_errors.extend(footnote.errors);
                parser.update(&candidate);
                return ok!(literal, all_errors);
            }
        }
    }

    if heading.has_toc {
        match &toc_elements {
            Some(toc) => parser.push_table_of_contents_entry(heading.level, toc),
            None => {
                if let Elements::Single(Element::Container(container)) = &elements {
                    parser.push_table_of_contents_entry(
                        heading.level,
                        container.elements(),
                    );
                }
            }
        }
    }

    if parser.settings().layout.legacy()
        && content == WikidotHeadingContent::Other
        && parser.current().token == Token::LeftBlock
        && parser
            .look_ahead(0)
            .is_some_and(|token| token.slice.eq_ignore_ascii_case("footnote"))
    {
        let mut candidate = parser.clone_with_rule(RULE_HEADER);
        if let Ok(footnote) = super::RULE_BLOCK.try_consume(&mut candidate)
            && let Elements::Single(Element::Footnote(index)) = footnote.item
            && let Elements::Single(Element::Container(container)) = &mut elements
        {
            container.elements_mut().push(Element::Footnote(index));
            all_errors.extend(footnote.errors);
            parser.update(&candidate);
        }
    }

    if parser.settings().layout.legacy() {
        elements = hoist_heading_explicit_lists(elements);
    }
    if crossed_close_on_line
        && let Elements::Single(Element::Container(container)) = &mut elements
        && container.elements().is_empty()
    {
        container
            .attributes_mut()
            .insert(WIKIDOT_HEADING_BARRIER_MARKER, cow!(""));
    }

    ok!(false; elements, all_errors)
}

type CollectedHeading<'t> = (Elements<'t>, Option<Vec<Element<'t>>>);

fn collect_wikidot_heading_with_barriers<'r, 't>(
    parser: &mut Parser<'r, 't>,
    ctype: ContainerType,
    heading: Heading,
    close: &[ParseCondition],
) -> ParseResult<'r, 't, CollectedHeading<'t>> {
    let replacement_merge = {
        let rest = &parser.full_text().inner()[parser.current().span.start..];
        match (rest.find('\u{0001}'), rest.find('\n')) {
            (Some(sentinel), Some(newline)) => sentinel < newline,
            _ => false,
        }
    };
    if parser.settings().layout.legacy()
        && !parser.discarding_hidden_body()
        && replacement_merge
    {
        return collect_wikidot_replacement_merged_heading(parser, ctype, close);
    }
    if parser.settings().layout.legacy() && !parser.discarding_hidden_body() {
        let from = parser.current().span.start;
        let collapsible_head = parser.wikidot_heading_collapsible_head(from);
        let closers = if collapsible_head.is_some() {
            Vec::new()
        } else {
            parser.wikidot_heading_flat_closers(from)
        };
        if collapsible_head.is_some() || !closers.is_empty() {
            return collect_split_wikidot_heading(
                parser,
                ctype,
                heading,
                close,
                collapsible_head,
                &closers,
            );
        }
        if collapsible_head.is_none() {
            let list_closers = parser.wikidot_heading_list_closers(from);
            if !list_closers.is_empty() {
                let collected =
                    collect_deleting_list_closers(parser, heading, close, &list_closers)?;
                let ((elements, errors), _, _) = collected.into();
                let content = Elements::Single(Element::Container(Container::new(
                    ctype,
                    elements,
                    AttributeMap::new(),
                )));
                return Ok(crate::parsing::ParseSuccess::new(
                    (content, None),
                    errors,
                    false,
                ));
            }
        }
    }

    let collected = collect_container(parser, RULE_HEADER, ctype, close, &[], None)?;
    let (elements, errors, paragraph_safe) = collected.into();
    Ok(ParseSuccess::new((elements, None), errors, paragraph_safe))
}

fn collect_wikidot_replacement_merged_heading<'r, 't>(
    parser: &mut Parser<'r, 't>,
    ctype: ContainerType,
    close: &[ParseCondition],
) -> ParseResult<'r, 't, CollectedHeading<'t>> {
    let first = collect_container(parser, RULE_HEADER, ctype, close, &[], None)?;
    let (first_elements, mut errors, paragraph_safe) = first.into();
    let Elements::Single(Element::Container(container)) = first_elements else {
        unreachable!("header collection always returns one container");
    };
    let mut elements = Vec::<Element>::from(container);
    let second = collect_container(parser, RULE_HEADER, ctype, close, &[], None)?;
    let (second_elements, second_errors, _) = second.into();
    errors.extend(second_errors);
    if let Elements::Single(Element::Container(container)) = second_elements {
        elements.extend(Vec::<Element>::from(container));
    }
    elements.push(Element::LineBreak);
    elements.push(Element::LineBreak);
    let content = Elements::Single(Element::Container(Container::new(
        ctype,
        elements,
        AttributeMap::new(),
    )));
    Ok(ParseSuccess::new((content, None), errors, paragraph_safe))
}

/// Collect heading text across `[[/li]]` stops on the heading line: the
/// closer is deleted mid-text and the heading continues (j1/j2/j6/x1 live
/// probes: `h[[/li]]` renders as `h`, a same-line ` y` joins the heading,
/// following item lines stay bare). No barrier marker, no split.
fn collect_deleting_list_closers<'r, 't>(
    parser: &mut Parser<'r, 't>,
    heading: Heading,
    close: &[ParseCondition],
    closers: &[(usize, bool)],
) -> ParseResult<'r, 't, (Vec<Element<'t>>, Vec<crate::parsing::ParseError>)> {
    let mut elements: Vec<Element<'t>> = Vec::new();
    let mut errors = Vec::new();
    let mut paragraph_safe = true;
    for &(byte, is_ol_or_ul) in closers {
        let mut conditions: Vec<ParseCondition> = close.to_vec();
        conditions.push(ParseCondition::AtSourceByte(byte));
        let collected =
            collect_consume_before(parser, RULE_HEADER, &conditions, &[], None)?;
        let ((chunk, _last), mut step_errors, chunk_safe) = collected.into();
        errors.append(&mut step_errors);
        paragraph_safe &= chunk_safe;
        elements.extend(chunk);
        if is_ol_or_ul {
            // c4/j4: the heading hard-stops before `[[/ul]]`/`[[/ol]]` (left
            // unconsumed); the item/list machinery ends around it. Skip the
            // post-loop tail collection: it would read the closer as text.
            let _ = heading;
            return Ok(crate::parsing::ParseSuccess::new(
                (elements, errors),
                Vec::new(),
                paragraph_safe,
            ));
        }
        let start = parser.current().span.start;
        if start >= byte {
            // Delete the closer (zero-width in the heading like it is in
            // live: h1 `h[[/li]] y` renders `h y`) and keep collecting.
            let mut probe = parser.clone();
            if probe.get_end_block().is_ok() {
                parser.update(&probe);
                parser.set_wikidot_consumed_li_close(byte);
            }
        }
    }
    let collected = collect_consume_before(parser, RULE_HEADER, close, &[], None)?;
    let ((chunk, _last), mut step_errors, chunk_safe) = collected.into();
    errors.append(&mut step_errors);
    elements.extend(chunk);
    paragraph_safe &= chunk_safe;
    if !matches!(
        parser.current().token,
        Token::InputEnd | Token::BulletItem | Token::NumberedItem
    ) {
        parser.step()?;
    }
    let _ = heading;
    crate::delayed::trim_trailing_ascii_space(&mut elements);
    Ok(crate::parsing::ParseSuccess::new(
        (elements, errors),
        Vec::new(),
        paragraph_safe,
    ))
}

fn collect_split_wikidot_heading<'r, 't>(
    parser: &mut Parser<'r, 't>,
    ctype: ContainerType,
    heading: Heading,
    close: &[ParseCondition],
    collapsible_head: Option<usize>,
    closers: &[(usize, usize, WikidotBlockKind)],
) -> ParseResult<'r, 't, CollectedHeading<'t>> {
    let source = parser.full_text().inner();
    let mut stops: Vec<usize> = closers.iter().map(|(byte, ..)| *byte).collect();
    if let Some(head) = collapsible_head {
        stops.push(head);
    }

    let mut label: Vec<Element<'t>> = Vec::new();
    let mut tail: Vec<Element<'t>> = Vec::new();
    let mut toc: Vec<Element<'t>> = Vec::new();
    let mut errors = Vec::new();
    let mut label_done = false;

    for (index, &byte) in stops.iter().enumerate() {
        let mut conditions: Vec<ParseCondition> = close.to_vec();
        conditions.push(ParseCondition::AtSourceByte(byte));
        parser.set_wikidot_heading_barrier(Some(byte));
        let collected =
            collect_consume_before(parser, RULE_HEADER, &conditions, &[], None);
        parser.set_wikidot_heading_barrier(None);
        let success = collected?;
        let ((mut elements, _last), mut step_errors, _) = success.into();
        errors.append(&mut step_errors);

        if label_done {
            toc.extend(elements.iter().cloned());
            tail.append(&mut elements);
        } else {
            toc.extend(elements.iter().cloned());
            label = elements;
            label_done = true;
        }

        if Some(byte) == collapsible_head {
            break;
        }
        if parser.current().span.start < byte {
            break;
        }

        let kind = closers[index].2;
        let start = parser.current().span.start;
        let mut probe = parser.clone();
        if probe.get_end_block().is_err() {
            break;
        }
        parser.update(&probe);
        let end = parser.current().span.start;
        let raw: Cow<'t, str> = Cow::Borrowed(&source[start..end]);
        toc.push(Element::Text(raw));
        if kind == WikidotBlockKind::Collapsible
            && parser.current().token == Token::Whitespace
            && parser.current().slice == " "
        {
            parser.step()?;
        }
    }

    if collapsible_head.is_none() {
        let success = collect_consume_before(parser, RULE_HEADER, close, &[], None)?;
        let ((mut elements, _last), mut step_errors, _) = success.into();
        errors.append(&mut step_errors);
        toc.extend(elements.iter().cloned());
        tail.append(&mut elements);
        if !matches!(
            parser.current().token,
            Token::InputEnd | Token::BulletItem | Token::NumberedItem
        ) {
            parser.step()?;
        }
    }

    if let Some(head) = collapsible_head {
        crate::delayed::trim_trailing_ascii_space(&mut label);
        let label_empty = label.iter().all(
            |element| matches!(element, Element::Text(text) if text.trim().is_empty()),
        );
        let id = heading
            .has_toc
            .then(|| Cow::Owned(format!("toc{}", parser.table_of_contents_len())));
        parser.set_wikidot_heading_reopen(Some((
            head,
            HeadingReopen {
                level: heading.level,
                id,
                label_empty,
            },
        )));
    }

    let mut content: Vec<Element<'t>> = Vec::with_capacity(tail.len() + 1);
    content.push(Element::Container(Container::new(
        ContainerType::HeadingLabel,
        label,
        AttributeMap::new(),
    )));
    content.extend(tail);

    let mut attributes = AttributeMap::new();
    attributes.insert(WIKIDOT_HEADING_BARRIER_MARKER, cow!(""));
    let element = Element::Container(Container::new(ctype, content, attributes));
    let mut success =
        ParseSuccess::new((Elements::Single(element), Some(toc)), errors, false);
    success.split_close_leftover = None;
    Ok(success)
}

/// Live hoists an explicit list out of a heading line: the heading keeps only
/// the content before the list, and the list (with whatever followed it on the
/// line) renders after the heading shell.
fn hoist_heading_explicit_lists<'t>(elements: Elements<'t>) -> Elements<'t> {
    match elements {
        Elements::Single(Element::Container(mut container)) => {
            let list_at = container.elements().iter().position(|element| {
                matches!(
                    element,
                    Element::List {
                        wikidot_explicit_block: true,
                        ..
                    }
                )
            });
            let Some(list_at) = list_at else {
                return Element::Container(container).into();
            };
            let hoisted = container.elements_mut().split_off(list_at);
            trim_heading_prefix_space(&mut container);
            if container.elements().is_empty() {
                container
                    .attributes_mut()
                    .insert(WIKIDOT_HEADING_EMPTIED_MARKER, cow!(""));
            }
            let mut output = Vec::with_capacity(hoisted.len() + 1);
            output.push(Element::Container(container));
            output.extend(hoisted);
            Elements::Multiple(output)
        }
        other => other,
    }
}

fn trim_heading_prefix_space(container: &mut crate::tree::Container<'_>) {
    let Some(Element::Text(text)) = container.elements_mut().last_mut() else {
        return;
    };
    let trimmed = text.trim_end_matches([' ', '\t']);
    if trimmed.is_empty() {
        container.elements_mut().pop();
    } else if trimmed.len() != text.len() {
        *text = Cow::Owned(trimmed.to_owned());
    }
}

fn wikidot_literal_heading<'t>(
    marker: &'t str,
    elements: Elements<'t>,
) -> Vec<Element<'t>> {
    let Elements::Single(Element::Container(container)) = elements else {
        unreachable!("header collection always returns one container");
    };
    let mut literal = Vec::with_capacity(container.elements().len() + 1);
    literal.push(text!(marker));
    literal.extend(Vec::from(container));
    literal
}

fn wikidot_heading_content(elements: &[Element<'_>]) -> WikidotHeadingContent {
    for element in elements {
        let content = match element {
            Element::Text(text) | Element::Raw(text)
                if text.chars().all(char::is_whitespace) =>
            {
                WikidotHeadingContent::Empty
            }
            Element::Container(container) => {
                wikidot_heading_content(container.elements())
            }
            Element::Anchor { elements, .. } | Element::Color { elements, .. } => {
                wikidot_heading_content(elements)
            }
            Element::Footnote(_) => WikidotHeadingContent::FootnoteFirst,
            Element::LineBreak => WikidotHeadingContent::Empty,
            Element::Partial(partial) if partial.is_inline_format_control() => {
                WikidotHeadingContent::Empty
            }
            _ => WikidotHeadingContent::Other,
        };
        if content != WikidotHeadingContent::Empty {
            return content;
        }
    }
    WikidotHeadingContent::Empty
}

fn indented_after_quote_marker(parser: &Parser<'_, '_>) -> bool {
    let source = parser.full_text().inner();
    let start = parser.current().span.start;
    let line_start = source[..start].rfind('\n').map_or(0, |offset| offset + 1);
    let prefix = &source[line_start..start];
    let Some(marker) = prefix.rfind('>') else {
        return false;
    };
    if !prefix[..marker]
        .chars()
        .all(|c| matches!(c, '>' | ' ' | '\t'))
    {
        return false;
    }
    let tail = &prefix[marker + 1..];
    tail.len() >= 2 && tail.chars().all(|c| matches!(c, ' ' | '\t'))
}

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    if parser.wikidot_footnote_zone_blocks(parser.current().span.start) {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    if parser.settings().layout.legacy()
        && parser.in_native_blockquote_line()
        && indented_after_quote_marker(parser)
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let success = consume_header_once(parser)?;
    if success.paragraph_safe {
        return Ok(success);
    }
    let (elements, mut all_errors, _) = success.into();
    let mut all_elements: Vec<_> = elements.into_iter().collect();

    if parser.native_blockquote_depth().is_some() {
        return ok!(false; all_elements, all_errors);
    }

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

        if success.paragraph_safe {
            parser.reset_mutable_state(parser_state);
            return ok!(false; all_elements, all_errors);
        }

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

    #[test]
    fn wikidot_heading_absorbs_footnote_from_next_line() {
        use crate::render::{Render, html::HtmlRender};

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization =
            crate::tokenize("+ Secret heading\n[[footnote]]secret[[/footnote]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        assert!(
            html.contains("Secret heading<sup class=\"footnoteref\""),
            "{html}",
        );
        assert!(!html.contains("</h1><p><sup"), "{html}");
    }
}
