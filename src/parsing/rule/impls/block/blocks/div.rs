/*
 * parsing/rule/impls/block/blocks/div.rs
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
use crate::delayed::DelayedElement;
use crate::parsing::collect::consume_valid_comment;
use crate::parsing::rule::impls::block::parser::BlockBodyStart;
use crate::settings::WikitextMode;
use crate::tree::AcceptsPartial;
use std::borrow::Cow;

pub const BLOCK_DIV: BlockRule = BlockRule {
    name: "block-div",
    accepts_names: &["div"],
    accepts_star: false,
    accepts_score: true,
    accepts_newlines: true,
    parse_fn,
};

fn wikidot_div_head_started_physical_line(
    parser: &Parser<'_, '_>,
    body_start: BlockBodyStart,
) -> bool {
    let source = parser.full_text().inner();
    let head_end = parser.current().span.start;
    let head_line_end = if body_start == BlockBodyStart::NextPhysicalLine {
        source[..head_end].trim_end_matches(['\r', '\n']).len()
    } else {
        head_end
    };
    let line_start = source[..head_line_end]
        .rfind('\n')
        .map_or(0, |offset| offset + 1);
    source[line_start..head_line_end]
        .trim_start_matches([' ', '\t'])
        .get(..5)
        .is_some_and(|head| head.eq_ignore_ascii_case("[[div"))
}

fn wikidot_div_follows_inline_structural_close(
    parser: &Parser<'_, '_>,
    body_start: BlockBodyStart,
) -> bool {
    if parser.accepts_partial() != AcceptsPartial::ListItem {
        return false;
    }
    let source = parser.full_text().inner();
    let head_end = parser.current().span.start;
    let literal_end = if body_start == BlockBodyStart::NextPhysicalLine {
        source[..head_end].trim_end_matches(['\r', '\n']).len()
    } else {
        head_end
    };
    let line_start = source[..literal_end]
        .rfind('\n')
        .map_or(0, |offset| offset + 1);
    let opener_start = source[line_start..literal_end]
        .rfind("[[")
        .map_or(line_start, |offset| line_start + offset);
    let prefix = source[line_start..opener_start].trim_end();
    prefix
        .get(prefix.len().saturating_sub(8)..)
        .is_some_and(|close| close.eq_ignore_ascii_case("[[/div]]"))
        || prefix
            .get(prefix.len().saturating_sub(7)..)
            .is_some_and(|close| close.eq_ignore_ascii_case("[[/ul]]"))
}

// Keep post-body normalization out of the recursively nested div parser's
// stack frame. The deep parser deliberately supports 1024 nested owners on a
// bounded worker stack.
#[inline(never)]
fn normalize_wikidot_div_elements(elements: &mut Vec<Element<'_>>, flag_score: bool) {
    if flag_score {
        let item_count = elements.len();
        let mut unwrapped = Vec::with_capacity(item_count.saturating_mul(2));
        for (index, element) in elements.drain(..).enumerate() {
            if index > 0 {
                unwrapped.push(text!("\n"));
            }
            match element {
                Element::Container(container)
                    if container.ctype() == ContainerType::Paragraph
                        && (index == 0 || index + 1 == item_count) =>
                {
                    unwrapped.extend(Vec::<Element>::from(container));
                }
                element => unwrapped.push(element),
            }
        }
        *elements = unwrapped;
        while matches!(
            elements.last(),
            Some(Element::LineBreak | Element::LineBreaks(_))
        ) {
            elements.pop();
        }
        let mut previous_was_scored_div = false;
        elements.retain(|element| {
            if previous_was_scored_div
                && (matches!(element, Element::LineBreak | Element::LineBreaks(_))
                    || matches!(element, Element::Text(text) if text == "\n"))
            {
                return false;
            }
            previous_was_scored_div = matches!(element, Element::Container(container)
                if container.ctype() == ContainerType::Div
                    && !container.elements().iter().any(|child| matches!(child,
                        Element::Container(paragraph)
                            if paragraph.ctype() == ContainerType::Paragraph)));
            true
        });
        let mut cleaned = Vec::with_capacity(elements.len());
        for element in elements.drain(..) {
            let redundant_newline_after_nested_div =
                matches!(&element, Element::Text(text) if text == "\n")
                    && matches!(cleaned.last(), Some(Element::LineBreak | Element::LineBreaks(_)))
                    && cleaned[..cleaned.len().saturating_sub(1)]
                        .iter()
                        .rev()
                        .find(|previous| {
                            !matches!(previous, Element::Text(text) if text == "\n")
                        })
                        .is_some_and(|previous| {
                            matches!(previous, Element::Container(container)
                                if container.ctype() == ContainerType::Div)
                        });
            if !redundant_newline_after_nested_div {
                cleaned.push(element);
            }
        }
        *elements = cleaned;
        return;
    }

    if matches!(elements.last(), Some(Element::LineBreak)) {
        elements.pop();
    }
    for index in 1..elements.len() {
        if !matches!(elements[index], Element::LineBreak | Element::LineBreaks(_))
            || !matches!(&elements[index - 1], Element::Container(container)
                if container.ctype() == ContainerType::Div)
        {
            continue;
        }
        let inline_scored_div_follows = matches!(elements.get(index + 1), Some(Element::Text(text))
                if text.to_ascii_lowercase().starts_with("[[div_]]"))
            || matches!(
                elements.get(index + 1..index + 5),
                Some([
                    Element::Text(open),
                    Element::Text(name),
                    Element::Text(score),
                    Element::Text(close),
                ]) if open == "[["
                    && name.eq_ignore_ascii_case("div")
                    && score == "_"
                    && close == "]]"
            );
        if inline_scored_div_follows {
            elements[index] = text!("\n");
        }
    }
}

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Parsing div block (name '{name}', in-head {in_head}, score {flag_score})");
    assert!(!flag_star, "Div doesn't allow star flag");
    assert_block_name(&BLOCK_DIV, name);
    let source = parser.full_text().inner();
    let name_start = (name.as_ptr() as usize)
        .checked_sub(source.as_ptr() as usize)
        .expect("parsed div name belongs to the source");
    let owner_start = source[..name_start]
        .rfind("[[")
        .expect("parsed div name follows its opener");

    let head = parser.get_head_map_with_body_start_wikidot(&BLOCK_DIV, in_head)?;
    let (arguments, mut body_start) = head;
    if parser.settings().layout.legacy() && arguments.has_empty_key() {
        return recover_wikidot_empty_key_candidate(parser, &BLOCK_DIV, owner_start);
    }
    if parser.settings().layout.legacy()
        && flag_score
        && !parser.wikidot_alias_has_compatible_close(&BLOCK_DIV, owner_start)
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    if parser.settings().layout.legacy()
        && parser.in_wikidot_simple_table_cell()
        && body_start == BlockBodyStart::Inline
        && let Some(delimiter) = wikidot_inline_div_table_delimiter(parser)
    {
        let literal_end = delimiter.current().span.start;
        parser.update(&delimiter);
        return ok!(true; text!(&source[owner_start..literal_end]));
    }
    if parser.settings().layout.legacy()
        && parser.settings().mode != WikitextMode::List
        && !parser.in_wikidot_div_body()
        && !parser.in_native_blockquote_line()
        && body_start == BlockBodyStart::Inline
        && parser.has_body_end_block_on_line(&BLOCK_DIV)
    {
        let _ = parser.get_body_text(&BLOCK_DIV)?;
        let owner_end = parser.current().span.start;
        return ok!(true; text!(&source[owner_start..owner_end]));
    }
    let head_started_physical_line =
        wikidot_div_head_started_physical_line(parser, body_start);
    let follows_inline_structural_close =
        wikidot_div_follows_inline_structural_close(parser, body_start);
    if parser.settings().layout.legacy()
        && parser.in_wikidot_div_body()
        && !head_started_physical_line
        && !follows_inline_structural_close
    {
        let source = parser.full_text().inner();
        let head_end = parser.current().span.start;
        let literal_end = if body_start == BlockBodyStart::NextPhysicalLine {
            source[..head_end].trim_end_matches(['\r', '\n']).len()
        } else {
            head_end
        };
        let line_start = source[..literal_end]
            .rfind('\n')
            .map_or(0, |offset| offset + 1);
        let opener_start = source[line_start..literal_end]
            .rfind("[[")
            .map_or(line_start, |offset| line_start + offset);
        let literal = text!(&source[opener_start..literal_end]);
        return if body_start == BlockBodyStart::NextPhysicalLine {
            ok!(Elements::Multiple(vec![literal, Element::LineBreak]))
        } else {
            ok!(literal)
        };
    }
    if parser.settings().layout.legacy()
        && parser.in_wikidot_div_body()
        && flag_score
        && body_start == BlockBodyStart::Inline
        && parser.current().token == Token::LeftBlockEnd
    {
        let mut close = parser.clone();
        if close
            .get_end_block()
            .is_ok_and(|name| name.eq_ignore_ascii_case("div"))
        {
            parser.update(&close);
            return ok!(false; Elements::None);
        }
    }
    if parser.settings().layout.legacy()
        && body_start == BlockBodyStart::Inline
        && parser.in_wikidot_div_body()
        && head_started_physical_line
        && !parser.in_native_blockquote_line()
        && !flag_score
    {
        while !matches!(
            parser.current().token,
            Token::LineBreak | Token::ParagraphBreak | Token::InputEnd
        ) {
            parser.step()?;
        }
        if parser.current().token == Token::LineBreak {
            parser.step()?;
        }
        body_start = BlockBodyStart::NextPhysicalLine;
    }
    if parser.settings().layout.legacy()
        && !parser.in_wikidot_div_body()
        && !parser.has_body_end_block(&BLOCK_DIV)
    {
        return Err(parser.make_err(ParseErrorKind::BlockExpectedEnd));
    }
    if parser.settings().layout.legacy()
        && !head_started_physical_line
        && !follows_inline_structural_close
        && parser.body_has_generated(&BLOCK_DIV)
    {
        let _ = parser.get_body_text(&BLOCK_DIV)?;
        let owner_end = parser.current().span.start;
        let generated = parser.generated_in_range(owner_start..owner_end);
        return success_elements(Element::Delayed(DelayedElement::shell(
            source,
            owner_start..owner_end,
            &generated,
        )));
    }

    // "div" means we wrap in paragraphs, like normal
    // "div_" means we don't wrap it
    let wrap_paragraphs = !flag_score;
    // Get body content, based on whether we want paragraphs or not.
    // Discard paragraph_safe, since divs never are.
    if parser.settings().layout.legacy() {
        parser.enter_wikidot_div_body();
        if flag_score {
            parser.enter_wikidot_scored_div_body();
        }
    }
    let parse_as_paragraphs =
        wrap_paragraphs || parser.settings().layout.legacy() && flag_score;
    let body = parser.get_body_elements_with_context(
        &BLOCK_DIV,
        parse_as_paragraphs,
        body_start,
    );
    if parser.settings().layout.legacy() {
        if flag_score {
            parser.leave_wikidot_scored_div_body();
        }
        parser.leave_wikidot_div_body();
    }
    let (mut elements, errors, _) = body?.into();
    if parser.settings().layout.legacy() {
        normalize_wikidot_div_elements(&mut elements, flag_score);
    }

    if parser.settings().layout.legacy() && arguments.is_empty() && elements.is_empty() {
        return ok!(false; Elements::None, errors);
    }

    // Build element and return
    let mut attributes = arguments.to_attribute_map(parser.settings());
    if parser.settings().layout.legacy()
        && let Some(class) = attributes.remove("class")
    {
        let class = class.trim_end_matches([' ', '\t']).to_owned();
        attributes.insert("class", Cow::Owned(class));
    }
    let element =
        Element::Container(Container::new(ContainerType::Div, elements, attributes));

    ok!(element, errors)
}

fn wikidot_inline_div_table_delimiter<'r, 't>(
    parser: &Parser<'r, 't>,
) -> Option<Parser<'r, 't>>
where
    'r: 't,
{
    let mut scan = parser.clone();
    let mut raw = false;
    let mut alternate_raw = false;
    let mut triple_link_depth = 0usize;

    loop {
        if matches!(
            scan.current().token,
            Token::LineBreak | Token::ParagraphBreak | Token::InputEnd
        ) {
            return None;
        }

        if !raw
            && !alternate_raw
            && triple_link_depth == 0
            && scan.current().token == Token::LeftComment
        {
            let mut comment = scan.clone();
            if consume_valid_comment(&mut comment).is_ok() {
                scan.update(&comment);
                continue;
            }
        }

        match scan.current().token {
            Token::Raw => raw = !raw,
            Token::LeftRaw if !raw => alternate_raw = true,
            Token::RightRaw if alternate_raw => alternate_raw = false,
            Token::LeftLink | Token::LeftLinkStar if !raw && !alternate_raw => {
                triple_link_depth += 1;
            }
            Token::RightLink if triple_link_depth > 0 => {
                triple_link_depth -= 1;
            }
            Token::TableColumn
            | Token::TableColumnTitle
            | Token::TableColumnCenter
            | Token::TableColumnRight
                if !raw && !alternate_raw && triple_link_depth == 0 =>
            {
                return Some(scan);
            }
            _ => {}
        }

        scan.step().ok()?;
    }
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::parsing::ParseErrorKind;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};

    fn render(input: &str, layout: Layout) -> String {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        assert!(errors.is_empty(), "{errors:#?}");
        HtmlRender.render(&tree, &page_info, &settings).body
    }

    #[test]
    fn wikidot_div_trims_trailing_class_whitespace_only_in_legacy_layout() {
        let input = "[[div class=\"box \"]]\nbody\n[[/div]]";

        assert_eq!(
            render(input, Layout::Wikidot),
            "<div class=\"box\"><p>body</p></div>",
        );
        assert_eq!(
            render(input, Layout::Wikijump),
            "<div class=\"box \"><p>body</p></div>",
        );
    }

    #[test]
    fn wikidot_continuation_before_a_div_preserves_its_block_boundary() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let mut source = concat!(
            "[[div id=\"outer\"]]\n",
            "[[a href=\"x\"]]x[[/a]]\\\n",
            "[[div_ class=\"image\"]]\n",
            "x\n",
            "[[/div]]\n",
            "[[/div]]",
        )
        .to_owned();
        crate::preprocess_for_layout(&mut source, settings.layout);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            "<div id=\"u-outer\"><a href=\"x\">x</a>\n<div class=\"image\">x</div></div>",
        );
    }

    #[test]
    fn wikidot_continuation_before_a_div_like_name_remains_inline() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let mut source = concat!(
            "[[a href=\"x\"]]x[[/a]]\\\n",
            "[[division]]\n",
            "x\n",
            "[[/div]]",
        )
        .to_owned();
        crate::preprocess_for_layout(&mut source, settings.layout);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(!errors.is_empty());
        assert_eq!(
            html,
            "<p><a href=\"x\">x</a>[[division]]<br>\nx<br>\n[[/div]]</p>",
        );
    }

    #[test]
    fn wikidot_exact_div_names_discard_malformed_arguments() {
        for (name, normal, continued) in [
            (
                "div",
                "<p>before</p><div><p>x</p></div><br>\nafter",
                "<a href=\"x\">x</a>\n<div><p>x</p></div>",
            ),
            (
                "div_",
                "<p>before</p><div>x</div><br>\nafter",
                "<a href=\"x\">x</a>\n<div>x</div>",
            ),
        ] {
            let normal_source =
                format!("before\n[[{name} @=\"value\"]]\nx\n[[/div]]\nafter",);
            assert_eq!(render_preprocessed_wikidot(&normal_source), normal);

            let continued_source = format!(
                "[[a href=\"x\"]]x[[/a]]\\\n[[{name} @=\"value\"]]\nx\n[[/div]]",
            );
            assert_eq!(render_preprocessed_wikidot(&continued_source), continued,);
        }

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        let mut source = "before\n[[div @=\"value\"]]\nx\n[[/div]]\nafter".to_owned();
        crate::preprocess_for_layout(&mut source, settings.layout);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        assert!(
            errors
                .iter()
                .any(|error| { error.kind() == ParseErrorKind::BlockMalformedArguments })
        );
        assert!(html.contains("[[div @=&quot;value&quot;]]"), "{html}");
        assert!(!html.contains("<div>"), "{html}");
    }

    fn render_preprocessed_wikidot(source: &str) -> String {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let mut source = source.to_owned();
        crate::preprocess_for_layout(&mut source, settings.layout);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        assert!(errors.is_empty(), "{errors:#?}");
        HtmlRender.render(&tree, &page_info, &settings).body
    }

    #[test]
    fn wikidot_div_drops_the_terminal_break_after_a_nested_list() {
        let input = concat!(
            "[[div id=\"fruit\" class=\"box\"]]\n",
            "  [[ul]]\n",
            "    [[li]] 1  [[/li]]\n",
            "    [[li]] 2  [[/li]]\n",
            "  [[/ul]]\n",
            "[[/div]]",
        );

        assert_eq!(
            render(input, Layout::Wikidot),
            "<div class=\"box\" id=\"u-fruit\"><ul>\n<li>1</li>\n<li>2</li>\n</ul></div>",
        );
        assert_eq!(
            render(input, Layout::Wikijump),
            "<div class=\"box\" id=\"fruit\"><ul><li>1 </li><li>2 </li></ul></div>",
        );
    }

    #[test]
    fn quoted_multiline_div_with_quoted_close_remains_native_and_bounded() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = concat!(
            "> [[div style=\"font-weight: bold;\"]]\n",
            "> First quoted line.\n",
            "> \n",
            "> Second quoted line.\n",
            "> [[/div]]\n",
        );
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            "<blockquote><div style=\"font-weight: bold;\"><p>First quoted line.</p><p>Second quoted line.</p></div></blockquote>",
        );
    }

    #[test]
    fn quoted_inline_div_remains_literal_like_wikidot() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = "> [[div class=\"notice\"]]Quoted body.[[/div]]\n";
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(!errors.is_empty());
        assert_eq!(
            html,
            "<blockquote><p>[[div class=&quot;notice&quot;]]Quoted body.[[/div]]</p></blockquote>",
        );
    }

    #[test]
    fn inline_div_remains_literal_in_wikidot_layout() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = "[[div id=\"credit-view\"]]X[[/div]]";
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html, "<p>[[div id=&quot;credit-view&quot;]]X[[/div]]</p>",);
    }

    #[test]
    fn prose_adjacent_inline_div_remains_one_literal_paragraph() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = "BEFORE|[[div]]X[[/div]]|AFTER";
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html, "<p>BEFORE|[[div]]X[[/div]]|AFTER</p>");
    }

    #[test]
    fn wikidot_divs_follow_inline_list_and_div_closers() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        for row_count in [1, 2] {
            let mut input = String::new();
            for _ in 0..row_count {
                input.push_str(
                    "[[div_ class=\"colmod-block\"]]\n\
                     [[ul]][[li class=\"folded\"]][[ul]]_[[/ul]][[div class=\"colmod-link-top\"]]\n\
                     [[div_ class=\"foldable-list-container\"]]\n\
                     link[[/div]][[/div]][[div class=\"colmod-content\"]]\n",
                );
            }
            for _ in 0..row_count {
                input.push_str(
                    "[[/div]][[div]]\n\
                     [[div_ class=\"foldable-list-container\"]]\n\
                     link[[/div]][[/div]][[/li]][[/ul]][[/div]]\n",
                );
            }

            let tokenization = crate::tokenize(&input);
            let (tree, errors) =
                crate::parse(&tokenization, &page_info, &settings).into();
            let html = HtmlRender.render(&tree, &page_info, &settings).body;

            assert!(errors.is_empty(), "{errors:#?}");
            assert_eq!(html.matches("colmod-link-top").count(), row_count);
            assert_eq!(html.matches("colmod-content").count(), row_count);
            assert_eq!(
                html.matches("foldable-list-container").count(),
                row_count * 2,
            );
            assert!(!html.contains("[[div"), "{html}");
            assert!(!html.contains("[[/div]]"), "{html}");
        }
    }

    #[test]
    fn unclosed_normal_div_lines_remain_in_a_paragraph() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = "[[div]]\n[[div]]\n[[div]]";
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(
            errors.iter().any(|error| {
                error.rule() == "block-div"
                    && error.kind() == ParseErrorKind::BlockExpectedEnd
            }),
            "{errors:#?}",
        );
        assert_eq!(html, "<p>[[div]]<br>\n[[div]]<br>\n[[div]]</p>");
    }

    #[test]
    fn quoted_scored_div_closes_without_absorbing_following_page() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = concat!(
            "> [[div_ class=\"notice\"]]\n",
            "> body\n",
            "> [[/div]]\n",
            "following page\n",
        );
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains("<div class=\"notice\">body</div>"), "{html}");
        assert!(html.contains("following page"), "{html}");
        assert!(!html.contains("[[/div]]"), "{html}");
        let div_end = html.find("</div>").expect("div close missing");
        let following = html.find("following page").expect("following text missing");
        assert!(div_end < following, "{html}");
    }

    #[test]
    fn scored_div_does_not_accept_scored_close_in_wikidot_layout() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[div_]]\nbody\n[[/div_]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(!errors.is_empty());
        assert_eq!(html, "<p>[[div_]]<br>\nbody<br>\n[[/div_]]</p>");
    }

    #[test]
    fn wikidot_scored_div_discards_formatting_breaks_around_nested_divs() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let source = concat!(
            "[[div_ class=\"outer\"]]\n",
            "    \t[[div_ class=\"left\"]]\n",
            "    \t\t[[span]]alpha[[/span]] [[span]]beta[[/span]]\n",
            "    \t[[/div]]\n",
            "    \t[[div_ class=\"empty\"]]\n",
            "    \t[[/div]]\n",
            "[[/div]]",
        );
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            "<div class=\"outer\"><div class=\"left\"><span>alpha</span> <span>beta</span></div><div class=\"empty\"></div></div>",
        );
    }

    #[test]
    fn wikidot_nested_div_line_and_scored_paragraph_boundaries_match_live_dom() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let source = concat!(
            "[[div]]\n",
            "cherry\n",
            "[[div id=\"tasty\"]]mango[[/div]]\n",
            "melon\n",
            "[[/div]]\n",
            "[[div_]][[/div]]\n",
            "[[div_ class=\"blockquote\"]]\n",
            "Apple\n",
            "[[/div]]\n",
            "[[div_]]Banana[[/div]]\n",
            "[[div_]]\n",
            "alpha\n\nbeta\n\ngamma\n",
            "[[/div]]",
        );
        let tokenization = crate::tokenize(source);
        let (tree, _) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert_eq!(
            html,
            "<div><p>cherry</p><div id=\"u-tasty\"><p>melon</p></div><div class=\"blockquote\">Apple</div>\n[[div_]]Banana</div><div>alpha\n<p>beta</p>\ngamma</div>",
        );
    }
}
