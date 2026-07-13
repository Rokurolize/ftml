/*
 * parsing/rule/impls/blockquote.rs
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
use crate::parsing::paragraph::ParagraphStack;
use crate::parsing::parser::QuoteBodyLineStatus;
use crate::parsing::{DepthItem, DepthList, process_depths};
use crate::tree::{AttributeMap, Container, ContainerType};

const MAX_BLOCKQUOTE_DEPTH: usize = 30;

#[derive(Debug)]
struct NativeQuoteRow<'t> {
    elements: Vec<Element<'t>>,
    paragraph_safe: bool,
    empty_spaced: bool,
}

pub const RULE_BLOCKQUOTE: Rule = Rule {
    name: "blockquote",
    position: LineRequirement::StartOfLine,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    // Context variables
    let mut depths = Vec::new();
    let mut errors = Vec::new();
    let mut consumed_pruned_row = false;

    // Produce a depth list with elements
    while parser.prepare_quote_body_line()? != QuoteBodyLineStatus::Boundary
        && parser.current().token == Token::Quote
    {
        let current = parser.current();

        // 1 or more ">"s in one token. Return ASCII length.
        let physical_depth = current.slice.len();
        let (depth, absolute_depth) = parser.native_blockquote_depths(physical_depth);
        debug_assert!(depth > 0, "residual quote depth must be positive");
        parser.step()?;
        // Wikidot distinguishes an empty `> ` row, which separates quoted
        // paragraphs, from an empty `>` row, which has no rendering effect.
        let spaced_after_marker = parser.current().token == Token::Whitespace;
        parser.get_optional_space()?; // allow whitespace after ">"
        if parser.current().token != Token::Quote {
            // Wikidot only counts contiguous quote markers toward native depth.
            // A marker after horizontal space is literal quoted content.
            parser.mark_virtual_start_of_line();
        }

        // Check that the depth isn't obscenely deep, to avoid DOS attacks via stack overflow.
        if absolute_depth > MAX_BLOCKQUOTE_DEPTH {
            return Err(parser.make_err(ParseErrorKind::BlockquoteDepthExceeded));
        }

        // Parse elements until we hit the end of the line
        let close_conditions = [
            ParseCondition::current(Token::LineBreak),
            ParseCondition::current(Token::ParagraphBreak),
            ParseCondition::current(Token::InputEnd),
        ];
        let close = &close_conditions;
        let mut paragraph_safe = true;
        let original_depth = parser.native_blockquote_depth();
        let (physical_line_end, ends_quote_run) = std::iter::once(parser.current())
            .chain(parser.remaining().iter())
            .find(|token| {
                matches!(
                    token.token,
                    Token::LineBreak | Token::ParagraphBreak | Token::InputEnd
                )
            })
            .map(|token| (token.span.end, token.token == Token::ParagraphBreak))
            .expect("tokenization always ends with input-end");
        parser.set_native_blockquote_depth(Some(absolute_depth));
        let result = collect_native_blockquote_line(parser, close);
        parser.set_native_blockquote_depth(original_depth);
        let errors_before = errors.len();
        let mut elements = result?.chain(&mut errors, &mut paragraph_safe);

        // An invisible multiline child can consume the quote row containing
        // its opener and finish beyond that physical line. Do not turn such a
        // row into a visible blank line solely because blockquotes normally
        // append a break to every row.
        let row_is_empty = elements.is_empty() && errors.len() == errors_before;
        let consumed_past_line =
            row_is_empty && parser.current().span.start > physical_line_end;
        let empty_spaced_row = row_is_empty && spaced_after_marker;
        if consumed_past_line || (row_is_empty && !spaced_after_marker) {
            consumed_pruned_row = true;
            if ends_quote_run {
                break;
            }
            continue;
        }

        // Add a line break for the end of the line
        if !empty_spaced_row {
            elements.push(Element::LineBreak);
        }

        // Append blockquote line
        //
        // Depth lists expect zero-based list depths, but tokens are one-based.
        // So, we subtract one.
        //
        // This will not overflow because Token::Quote requires at least one ">".
        depths.push((
            depth - 1,
            (),
            NativeQuoteRow {
                elements,
                paragraph_safe,
                empty_spaced: empty_spaced_row,
            },
        ));

        // An unquoted blank line terminates the current native quote run.
        // A following quote at the same depth starts a sibling blockquote.
        if ends_quote_run {
            break;
        }
    }

    // This blockquote has no rows, so the rule fails
    if depths.is_empty() {
        if consumed_pruned_row {
            return ok!(false; Elements::None, errors);
        }
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let depth_lists = process_depths((), depths);
    let elements: Vec<Element> = depth_lists
        .into_iter()
        .filter_map(|(_, depth_list)| build_blockquote_element(depth_list))
        .collect();

    ok!(false; elements, errors)
}

/// Collect exactly one physical native-quote line.
///
/// Several start-of-line rules consume their trailing line-break token. The
/// generic collector would then continue into the next physical line and can
/// swallow an enclosing block's closer, causing combinatorial block retries.
fn collect_native_blockquote_line<'r, 't>(
    parser: &mut Parser<'r, 't>,
    close: &[ParseCondition],
) -> ParseResult<'r, 't, Vec<Element<'t>>> {
    let line_end_token = std::iter::once(parser.current())
        .chain(parser.remaining().iter())
        .find(|token| {
            token.token == Token::LineBreak
                || token.token == Token::ParagraphBreak
                || token.token == Token::InputEnd
        })
        .expect("tokenization always ends with input-end");
    let line_end = line_end_token.span.end;
    let mut elements = Vec::new();
    let mut errors = Vec::new();
    let mut paragraph_safe = true;

    loop {
        if parser.evaluate_any(close) {
            if parser.current().token != Token::InputEnd {
                parser.step()?;
            }
            return ok!(paragraph_safe; elements, errors);
        }

        let consumed = consume(parser)?.chain(&mut errors, &mut paragraph_safe);
        elements.extend(consumed);

        // A child rule may already have consumed this line's break. Stop at
        // the first token after it instead of parsing the enclosing next line.
        if parser.current().span.start >= line_end {
            return ok!(paragraph_safe; elements, errors);
        }
    }
}

fn build_blockquote_element(list: DepthList<(), NativeQuoteRow>) -> Option<Element> {
    let mut stack = ParagraphStack::new();

    // Convert depth list into a list of elements
    for item in list {
        match item {
            DepthItem::Item(row) => {
                if row.empty_spaced {
                    stack.pop_line_break();
                    stack.end_paragraph();
                    continue;
                }
                for element in row.elements {
                    stack.push_element(element, row.paragraph_safe);
                }
            }
            DepthItem::List(_, list) => {
                if let Some(blockquote) = build_blockquote_element(list) {
                    stack.pop_line_break();
                    stack.push_element(blockquote, false);
                }
            }
        }
    }

    stack.pop_line_break();
    let elements = stack.into_elements();
    if elements.is_empty() {
        return None;
    }

    Some(Element::Container(Container::new(
        ContainerType::Blockquote,
        elements,
        AttributeMap::new(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender, text::TextRender};
    use crate::settings::{WikitextMode, WikitextSettings};
    use std::sync::Once;

    #[derive(Debug)]
    struct TestLogger;

    impl log::Log for TestLogger {
        fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            let _ = record.args().to_string();
        }

        fn flush(&self) {}
    }

    static TEST_LOGGER: TestLogger = TestLogger;
    static INIT_LOGGER: Once = Once::new();

    fn enable_test_logging() {
        INIT_LOGGER.call_once(|| {
            let _ = log::set_logger(&TEST_LOGGER);
            log::set_max_level(log::LevelFilter::Trace);
        });
    }

    #[test]
    fn native_blockquote_rejects_excessive_depth() {
        enable_test_logging();

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = format!("{} too deep", ">".repeat(MAX_BLOCKQUOTE_DEPTH + 1));
        let tokenization = crate::tokenize(&input);
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("quote token should follow input start");
        parser.set_rule(RULE_BLOCKQUOTE);

        let error = RULE_BLOCKQUOTE
            .try_consume(&mut parser)
            .expect_err("excessive blockquote depth should fail");
        assert_eq!(error.kind(), ParseErrorKind::BlockquoteDepthExceeded);
    }

    #[test]
    fn native_blockquote_rejects_non_quote_start() {
        enable_test_logging();

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("plain");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("identifier token should follow input start");
        parser.set_rule(RULE_BLOCKQUOTE);

        let error = RULE_BLOCKQUOTE
            .try_consume(&mut parser)
            .expect_err("non-quote input should not produce a blockquote");
        assert_eq!(error.kind(), ParseErrorKind::RuleFailed);
    }

    #[test]
    fn native_blockquote_prunes_empty_rows_at_every_depth() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let mut input = "> \n>\n>> \n".to_owned();
        crate::preprocess(&mut input);
        let tokenization = crate::tokenize(&input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.trim().is_empty(), "{html}");
    }

    #[test]
    fn native_blockquote_prunes_rows_consumed_by_an_invisible_child() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let mut input = concat!(
            "> [[collapsible show=\"show\" hide=\"hide\"]]\n",
            "> [[iftags +missing]]\n",
            "> [[div]]\n",
            "> OMEGA_HIDDEN\n",
            "> [[/iftags]]\n",
            "> OMEGA_VISIBLE_INSIDE\n",
            "> [[/collapsible]]\n",
            "OMEGA_AFTER",
        )
        .to_owned();
        crate::preprocess(&mut input);
        let tokenization = crate::tokenize(&input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(!html.contains("OMEGA_HIDDEN"), "{html}");
        assert!(html.contains("OMEGA_VISIBLE_INSIDE"), "{html}");
        assert!(html.contains("OMEGA_AFTER"), "{html}");
    }

    #[test]
    fn native_blockquote_content_respects_virtual_line_start_for_headings() {
        enable_test_logging();

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = concat!(
            "> + {{WARNING}}\n",
            ">\n",
            "> {{Body}}\n",
            ">\n",
            "> ++ {{LEVEL 5 AUTHORIZATION REQUIRED}}\n",
        );
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");

        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("<h1"));
        assert!(html.contains("WARNING"));
        assert!(html.contains("<h2"));
        assert!(html.contains("LEVEL 5 AUTHORIZATION REQUIRED"));
        assert!(!html.contains("+ <tt>WARNING</tt>"));
        assert!(!html.contains("++ <tt>LEVEL 5 AUTHORIZATION REQUIRED</tt>"));

        let text = TextRender.render(&tree, &page_info, &settings);
        assert!(text.contains("WARNING"));
        assert!(text.contains("LEVEL 5 AUTHORIZATION REQUIRED"));
        assert!(!text.contains("+ WARNING"));
        assert!(!text.contains("++ LEVEL 5 AUTHORIZATION REQUIRED"));
    }

    #[test]
    fn native_blockquote_depth_counts_only_contiguous_markers() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = concat!(
            ">> ALPHA_CONTIGUOUS_DEPTH_TWO\n",
            "> > ALPHA_SPACED_LITERAL\n",
            "> >ALPHA_TIGHT_SPACED_LITERAL\n",
            "> ALPHA_BEFORE\n",
            "> >ALPHA_ACTIVE_LITERAL\n",
            "> ALPHA_AFTER\n",
        );
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html.matches("<blockquote>").count(), 2, "{html}");
        assert!(html.contains("ALPHA_CONTIGUOUS_DEPTH_TWO"), "{html}");
        assert!(html.contains("&gt; ALPHA_SPACED_LITERAL"), "{html}");
        assert!(html.contains("&gt;ALPHA_TIGHT_SPACED_LITERAL"), "{html}");
        assert!(html.contains("&gt;ALPHA_ACTIVE_LITERAL"), "{html}");
        assert!(html.contains("ALPHA_AFTER"), "{html}");
    }

    #[test]
    fn native_blockquote_horizontal_rule_does_not_consume_outer_block_close() {
        enable_test_logging();

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = concat!(
            "[[collapsible]]\n",
            "> Derivative of:\n",
            "> ------\n",
            "> Author\n",
            "[[/collapsible]]\n",
            "After\n",
        );
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");

        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        assert!(
            html.contains(r#"<details class="wj-collapsible""#),
            "{html}"
        );
        assert!(html.contains("<blockquote>"), "{html}");
        assert!(html.contains("<hr>"), "{html}");
        assert!(html.contains("Author"), "{html}");
        assert!(html.contains("After"), "{html}");
        assert!(!html.contains("[[collapsible"), "{html}");
        assert!(!html.contains("[[/collapsible]]"), "{html}");
    }

    #[test]
    fn native_blockquote_line_rules_do_not_trigger_combinatorial_div_retries() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        // Reduced from EN:anomalous-entity-engagement-division-hub
        // (source SHA-256 19ceb79035996a7c70df15cbcfc49ebe4833147aef4343b43ae77786f43e5f04),
        // which repeats div-wrapped native quote lines throughout its tabview.
        let cases = [
            ("= Centered corpus quote", "text-align: center;"),
            ("+ Quoted heading", "<h1"),
            ("* Quoted list item", "<ul>"),
            ("[[toc]]", "id=\"wj-toc\""),
            ("----", "<hr>"),
        ];

        for (quoted_line, expected) in cases {
            let mut input = String::new();
            for _ in 0..25 {
                input.push_str("[[div class=\"table1\"]]\n> ");
                input.push_str(quoted_line);
                input.push_str("\n[[/div]]\n");
            }
            input.push_str("Following sentinel\n");

            let tokenization = crate::tokenize(&input);
            let (tree, errors) =
                crate::parse(&tokenization, &page_info, &settings).into();
            let html = HtmlRender.render(&tree, &page_info, &settings).body;

            assert!(errors.is_empty(), "{quoted_line}: {errors:#?}");
            assert_eq!(
                html.matches("class=\"table1\"").count(),
                25,
                "{quoted_line}: {html}",
            );
            assert_eq!(
                html.matches("<blockquote>").count(),
                25,
                "{quoted_line}: {html}",
            );
            assert!(html.contains(expected), "{quoted_line}: {html}");
            assert!(html.contains("Following sentinel"), "{quoted_line}: {html}");
        }
    }
}
