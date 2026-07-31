/*
 * parsing/paragraph/mod.rs
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

mod stack;

pub use self::stack::ParagraphStack;
pub(crate) use self::stack::collapsible_has_direct_literal_nested_opener;

use super::consume::consume;
use super::parser::Parser;
use super::parser::QuoteBodyLineStatus;
use super::prelude::*;
use super::rule::Rule;
use super::token::Token;

/// Wrapper type to satisfy the issue with generic closure types.
///
/// Because `None` does not specify the type for `F`, we need to
/// tell the compiler it has a concrete type.
///
/// But since it's just `None`, it's not actually pointing to a function,
/// it's just clarifying what the `_` in `Option<_>` is.
pub const NO_CLOSE_CONDITION: Option<CloseConditionFn> = None;

type CloseConditionFn = fn(&mut Parser) -> Result<bool, ParseError>;

/// Function to iterate over tokens to produce elements in paragraphs.
///
/// Originally in `parse()`, but was moved out to allow paragraph
/// extraction deeper in code, such as in the `try_paragraph`
/// collection helper.
///
/// This does not necessarily produce a paragraph container.
/// It may produce multiple or none. Instead the logic iterates
/// and produces paragraphs or child elements as needed.
pub fn gather_paragraphs<'r, 't, F>(
    parser: &mut Parser<'r, 't>,
    rule: Rule,
    mut close_condition_fn: Option<F>,
) -> ParseResult<'r, 't, Vec<Element<'t>>>
where
    'r: 't,
    F: FnMut(&mut Parser<'r, 't>) -> Result<bool, ParseError>,
{
    // Update parser rule
    parser.set_rule(rule);

    // Create paragraph stack
    let mut stack = if parser.settings().layout.legacy() {
        ParagraphStack::new_wikidot()
    } else {
        ParagraphStack::new()
    };

    let mut finished = false;
    while !finished {
        let quote_line_status = parser.prepare_quote_body_line()?;
        if quote_line_status == QuoteBodyLineStatus::Boundary {
            if parser.quote_boundary_closes_body() {
                stack.pop_line_break();
                finished = true;
                continue;
            }
            return Err(parser.make_err(ParseErrorKind::EndOfInput));
        }
        if parser.settings().layout.legacy()
            && quote_line_status == QuoteBodyLineStatus::Prepared
            && parser.quote_body_line_is_empty_spaced()
        {
            stack.pop_line_break();
            stack.end_paragraph();
            parser.step()?;
            continue;
        }

        match wikidot_empty_raw_line_kind(parser) {
            Some(WikidotEmptyRawLineKind::Complete) => {
                stack.push_wikidot_invisible_raw_line_break();
            }
            Some(WikidotEmptyRawLineKind::Paired) => {
                stack.mark_wikidot_invisible_raw_pair_occupancy();
            }
            None => {}
        }
        let terminal_backslash =
            parser.current().token == Token::LineBreak && parser.current().slice == "\\";
        let invisible_raw_line_break = parser.current().token == Token::LineBreak
            && stack.wikidot_invisible_raw_line_break_pending();
        let invisible_raw_pair_line_break = parser.current().token == Token::LineBreak
            && stack.wikidot_invisible_raw_pair_occupied();
        let continued_block_boundary = parser.current().token == Token::LineBreak
            && parser.current().slice == "\\\n";
        if continued_block_boundary && parser.settings().layout.legacy() {
            stack.mark_wikidot_continued_block_boundary();
        }
        let terminal_spaced_underscore = parser.settings().layout.legacy()
            && parser.current().token == Token::Whitespace
            && parser
                .look_ahead(0)
                .is_some_and(|token| token.token == Token::Underscore)
            && parser
                .look_ahead(1)
                .is_some_and(|token| token.token == Token::InputEnd);
        let comment = parser.current().token == Token::LeftComment;
        let comment_started_line = comment && parser.start_of_line();
        let empty_quote_control =
            parser.current().token == Token::Quote && parser.start_of_line();
        let consumed = match parser.current().token {
            Token::InputEnd => {
                if close_condition_fn.is_some() {
                    if parser.settings().layout.legacy() && rule.name() == "block-div" {
                        finished = true;
                        continue;
                    }
                    // There was a close condition, but it was not satisfied
                    // before the end of input.
                    //
                    // Pass an error up the chain

                    warn!("Hit the end of input, producing an error");
                    return Err(parser.make_err(ParseErrorKind::EndOfInput));
                } else {
                    // Avoid an unnecessary Element::Null and just exit
                    // If there's no close condition, then this is not an error

                    warn!("Hit the end of input, terminating token iteration");
                    finished = true;
                    None
                }
            }

            // If we've hit a paragraph break, then finish the current paragraph
            Token::ParagraphBreak => {
                if stack.wikidot_invisible_raw_line_break_pending() {
                    stack.clear_wikidot_invisible_raw_line_break_pending();
                }
                if stack.wikidot_invisible_raw_pair_occupied() {
                    stack.clear_wikidot_invisible_raw_pair_occupancy();
                }
                // Paragraph break -- end the paragraph and start a new one!
                stack.end_paragraph_at_break();

                // We must manually bump up this pointer because
                // we 'continue' here, skipping the usual pointer update.
                parser.step()?;
                None
            }

            // Determine if we're ending the paragraph here,
            // or continuing with another element
            _ => {
                let close_condition_met = match close_condition_fn.as_mut() {
                    Some(close_condition_fn) => close_condition_fn(parser)?,
                    None => false,
                };

                if close_condition_met {
                    finished = true;
                    None
                } else {
                    if parser.current().token == Token::DiscardedControl {
                        stack.mark_discarded_control();
                    }
                    // Otherwise, produce consumption from this token pointer
                    match consume(parser) {
                        Ok(consumed) => Some(consumed),
                        Err(error)
                            if parser.discarding_hidden_body()
                                && parser.at_hidden_body_boundary() =>
                        {
                            let close_condition = close_condition_fn
                                .as_mut()
                                .expect("body parser must have a close condition");
                            let close_condition_met = close_condition(parser)?;

                            finished =
                                finish_hidden_boundary(close_condition_met, error)?;
                            None
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
        };

        if let Some(consumed) = consumed {
            let (elements, mut errors, paragraph_safe) = consumed.into();
            let literal_list_item = stack.current_empty()
                && errors
                    .iter()
                    .any(|error| error.kind() == ParseErrorKind::ListItemOutsideList);
            let literal_iftags_line = parser.settings().layout.legacy()
                && errors.iter().any(|error| {
                    error.kind() == ParseErrorKind::RuleFailed
                        && error.rule() == "block-iftags"
                });
            let literal_div_line = parser.settings().layout.legacy()
                && errors.iter().any(|error| {
                    error.kind() == ParseErrorKind::RuleFailed
                        && error.rule() == "block-div"
                });

            // Add new elements to the list
            if invisible_raw_line_break
                && (elements.is_empty()
                    || elements == Elements::Single(Element::LineBreak))
            {
                // The complete empty raw line already contributed this break.
            } else if invisible_raw_pair_line_break && elements.is_empty() {
                stack.push_element(Element::LineBreak, true);
            } else if empty_quote_control && !paragraph_safe && elements.is_empty() {
                stack.end_paragraph();
            } else {
                push_elements(&mut stack, elements, paragraph_safe);
            }
            if invisible_raw_line_break {
                stack.clear_wikidot_invisible_raw_line_break_pending();
            }
            if invisible_raw_pair_line_break {
                stack.clear_wikidot_invisible_raw_pair_occupancy();
            }
            if comment_started_line
                && parser.settings().layout.legacy()
                && parser.current().token == Token::Whitespace
            {
                stack.push_paragraph_safe_elements(vec![text!(" ")]);
                parser.step()?;
            }
            if literal_list_item {
                stack.mark_wikidot_literal_list_item();
            }
            if literal_iftags_line {
                stack.mark_wikidot_literal_iftags_line();
            }
            if literal_div_line {
                stack.mark_wikidot_literal_div_line();
            }
            if terminal_backslash && parser.settings().layout.legacy() {
                if parser.full_text().inner().ends_with("\u{fffd}\\") {
                    stack.pop_line_break();
                }
                stack.mark_wikidot_terminal_backslash();
            }
            if terminal_spaced_underscore {
                stack.mark_wikidot_terminal_backslash();
            }
            if comment_started_line
                && parser.settings().layout.legacy()
                && !stack.current_empty()
                && matches!(
                    parser.current().token,
                    Token::LineBreak | Token::ParagraphBreak | Token::InputEnd
                )
            {
                stack.pop_line_break();
            }

            // Process errors
            stack.push_errors(&mut errors);
        }
    }

    stack.into_result()
}

#[derive(Clone, Copy)]
enum WikidotEmptyRawLineKind {
    Complete,
    Paired,
}

fn wikidot_empty_raw_line_kind(
    parser: &Parser<'_, '_>,
) -> Option<WikidotEmptyRawLineKind> {
    let boundary = |token: Option<&ExtractedToken<'_>>| {
        token.is_some_and(|token| {
            matches!(
                token.token,
                Token::LineBreak | Token::ParagraphBreak | Token::InputEnd
            )
        })
    };
    if !(parser.settings().layout.legacy()
        && parser.start_of_line()
        && parser.current().token == Token::Raw
        && parser
            .look_ahead(0)
            .is_some_and(|token| token.token == Token::Raw))
    {
        return None;
    }
    if boundary(parser.look_ahead(1)) {
        return Some(WikidotEmptyRawLineKind::Complete);
    }
    (parser
        .look_ahead(1)
        .is_some_and(|token| token.token == Token::Whitespace && token.slice == " ")
        && parser
            .look_ahead(2)
            .is_some_and(|token| token.token == Token::Raw)
        && parser
            .look_ahead(3)
            .is_some_and(|token| token.token == Token::Raw)
        && boundary(parser.look_ahead(4)))
    .then_some(WikidotEmptyRawLineKind::Paired)
}

fn finish_hidden_boundary(
    close_condition_met: bool,
    error: ParseError,
) -> Result<bool, ParseError> {
    if close_condition_met {
        Ok(true)
    } else {
        Err(error)
    }
}

fn push_elements<'t>(
    stack: &mut ParagraphStack<'t>,
    elements: Elements<'t>,
    paragraph_safe: bool,
) {
    match elements {
        Elements::None if paragraph_safe => {}
        Elements::None => stack.mark_current_unwrapped(),
        Elements::Single(element) => push_element(stack, element, paragraph_safe),
        Elements::Multiple(elements) if paragraph_safe => {
            stack.push_paragraph_safe_elements(elements);
        }
        Elements::Multiple(elements) => {
            if elements
                .iter()
                .any(|element| matches!(element, Element::TabView(_)))
            {
                stack.mark_wikidot_tabview_boundary();
            }
            for element in elements {
                push_element(stack, element, paragraph_safe);
            }
        }
    }
}

fn push_element<'t>(
    stack: &mut ParagraphStack<'t>,
    element: Element<'t>,
    paragraph_safe: bool,
) {
    // Don't add a line break if the paragraph is otherwise empty
    if !(paragraph_safe
        && stack.current_empty()
        && element == Element::LineBreak
        && !stack.wikidot_line_break_follows_block())
    {
        if paragraph_safe
            && element == Element::LineBreak
            && stack.wikidot_line_break_follows_block()
        {
            stack.mark_next_unwrapped_after_block();
        }
        stack.push_element(element, paragraph_safe);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};

    fn render_wikidot(source: &str) -> String {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let mut source = source.to_owned();
        crate::preprocess_for_layout(&mut source, settings.layout);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        assert!(errors.is_empty(), "{errors:?}");
        HtmlRender.render(&tree, &page_info, &settings).body
    }

    #[test]
    fn non_paragraph_safe_multiple_elements_do_not_reserve_current_paragraph() {
        let mut stack = ParagraphStack::new();
        push_elements(
            &mut stack,
            Elements::Multiple(vec![Element::HorizontalRule, Element::HorizontalRule]),
            false,
        );

        assert_eq!(stack.current_capacity(), 0);
    }

    #[test]
    fn empty_non_paragraph_safe_result_unwraps_pending_content() {
        let mut stack = ParagraphStack::new();
        stack.push_element(text!("alpha"), true);
        stack.push_element(Element::LineBreak, true);

        push_elements(&mut stack, Elements::None, false);

        assert_eq!(
            stack.into_elements(),
            vec![text!("alpha"), Element::LineBreak],
        );
    }

    #[test]
    fn empty_quote_control_does_not_leave_a_line_break() {
        assert_eq!(render_wikidot("a\n>b"), "<p>a</p>");
    }

    #[test]
    fn wikidot_html_block_starts_a_new_paragraph() {
        assert_eq!(
            render_wikidot(
                "before\n[[html]]\n<b>first</b>\n[[/html]]\nmiddle\n[[html]]\n<b>second</b>\n[[/html]]\nafter",
            ),
            concat!(
                "<p>before</p>",
                "<p><iframe src=\"https://example.com/\" allowtransparency=\"true\" frameborder=\"0\" class=\"html-block-iframe\"></iframe><br>\n",
                "middle</p>",
                "<p><iframe src=\"https://example.com/\" allowtransparency=\"true\" frameborder=\"0\" class=\"html-block-iframe\"></iframe><br>\n",
                "after</p>",
            ),
        );
    }

    #[test]
    fn hidden_boundary_propagates_a_mismatched_child_close_error() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("plain");
        let parser = Parser::new(&tokenization, &page_info, &settings);
        let finished = finish_hidden_boundary(
            true,
            parser.make_err(ParseErrorKind::BlockExpectedEnd),
        )
        .unwrap();
        let error = parser.make_err(ParseErrorKind::BlockExpectedEnd);
        let propagated = finish_hidden_boundary(false, error).unwrap_err();

        assert!(finished);
        assert_eq!(propagated.kind(), ParseErrorKind::BlockExpectedEnd);
    }

    #[test]
    fn discarded_controls_are_invisible_without_losing_physical_line_occupancy() {
        assert_eq!(render_wikidot("\0\t \t\t\nΩ"), "<p><br>\nΩ</p>",);
        assert_eq!(
            render_wikidot("\u{0007}\talpha \u{000b} beta"),
            "<p>alpha beta</p>",
        );
    }

    #[test]
    fn discarded_control_remains_a_structural_syntax_barrier() {
        assert_eq!(render_wikidot("\0> quote"), "<p>&gt; quote</p>",);
    }

    #[test]
    fn discarded_control_does_not_preserve_leading_line_whitespace() {
        assert_eq!(
            render_wikidot("a\r\u{0001}\t\u{fffd}\0b"),
            "<p>a<br>\nb</p>",
        );
    }

    #[test]
    fn wikidot_preserves_an_underscore_at_the_start_of_a_line() {
        assert_eq!(render_wikidot("_\na"), "<p>_<br>\na</p>");
        assert_eq!(render_wikidot(" _\na"), "<p><br>\na</p>");
    }

    #[test]
    fn wikidot_terminal_spaced_underscore_emits_two_line_breaks() {
        assert_eq!(render_wikidot("a\\ _"), "\n\na\\<br>\n<br>\n");
    }

    #[test]
    fn wikidot_terminal_backslash_becomes_an_unwrapped_line_break() {
        assert_eq!(render_wikidot("\\"), "");
        assert_eq!(render_wikidot("alpha\\"), "\n\nalpha<br>\n");
        assert_eq!(
            render_wikidot("first\nsecond\\"),
            "\n\nfirst<br>\nsecond<br>\n",
        );
        assert_eq!(
            render_wikidot("paragraph\n\nlast\\"),
            "<p>paragraph</p>\nlast<br>\n",
        );
        assert_eq!(render_wikidot("a\u{fffd}\\"), "\n\na1");
    }

    #[test]
    fn wikidot_comment_only_line_does_not_leave_a_line_break() {
        assert_eq!(
            render_wikidot("alpha\n[!-- hidden --]\nomega"),
            "<p>alpha<br>\nomega</p>",
        );
        assert_eq!(
            render_wikidot("alpha\n[!-- hidden --]\n\nomega"),
            "<p>alpha</p><p>omega</p>",
        );
        assert_eq!(
            render_wikidot("alpha\n[!-- hidden --] visible\nomega"),
            "<p>alpha<br>\n visible<br>\nomega</p>",
        );
        assert_eq!(
            render_wikidot("before [!-- hidden --] after"),
            "<p>before after</p>",
        );
        assert_eq!(
            render_wikidot(
                "[!-- --]OMEGA_TRUE[!-- --]\n[!-- OMEGA_FALSE[!-- --]\n[!-- 2 OMEGA_COMMENT --]\nOMEGA_AFTER",
            ),
            "<p>OMEGA_TRUE<br>\nOMEGA_AFTER</p>",
        );
        assert_eq!(
            render_wikidot("before\n\n[!-- --]\nbranch body\n[!----]\n\n\nmiddle"),
            "<p>before</p><p>branch body</p><p>middle</p>",
        );
    }

    #[test]
    fn wikidot_comment_after_advanced_list_preserves_the_list_break() {
        assert_eq!(
            render_wikidot(
                "[[ol]][[li]]before[[/li]][[/ol]]\n\n[!-- hidden --]\n[[ul]][[li]]after[[/li]][[/ul]]",
            ),
            "<ol>\n<li>before</li>\n</ol><br>\n<ul>\n<li>after</li>\n</ul><br>\n",
        );
    }

    #[test]
    fn line_after_block_is_unwrapped_only_in_wikidot_layout() {
        let page_info = PageInfo::dummy();
        let input = "[[div]]\nblock\n[[/div]]\nafter";
        let render = |layout| {
            let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
            let tokenization = crate::tokenize(input);
            let (tree, errors) =
                crate::parse(&tokenization, &page_info, &settings).into();
            assert!(errors.is_empty(), "{layout:?}: {errors:#?}");
            HtmlRender.render(&tree, &page_info, &settings).body
        };

        let wikidot = render(Layout::Wikidot);
        let wikijump = render(Layout::Wikijump);
        assert!(wikidot.contains("</div><br>\nafter"), "{wikidot}");
        assert!(!wikidot.contains("<p>after</p>"), "{wikidot}");
        assert!(wikijump.contains("</div><p>after</p>"), "{wikijump}");
    }
}
