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
    let mut stack = ParagraphStack::new();

    let mut finished = false;
    while !finished {
        if parser.prepare_quote_body_line()? == QuoteBodyLineStatus::Boundary {
            return Err(parser.make_err(ParseErrorKind::EndOfInput));
        }

        let consumed = match parser.current().token {
            Token::InputEnd => {
                if close_condition_fn.is_some() {
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
                // Paragraph break -- end the paragraph and start a new one!
                stack.end_paragraph();

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

                            if close_condition_met {
                                finished = true;
                                None
                            } else {
                                return Err(error);
                            }
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
        };

        if let Some(consumed) = consumed {
            let (elements, mut errors, paragraph_safe) = consumed.into();

            // Add new elements to the list
            push_elements(&mut stack, elements, paragraph_safe);

            // Process errors
            stack.push_errors(&mut errors);
        }
    }

    stack.into_result()
}

fn push_elements<'t>(
    stack: &mut ParagraphStack<'t>,
    elements: Elements<'t>,
    paragraph_safe: bool,
) {
    match elements {
        Elements::None => {}
        Elements::Single(element) => push_element(stack, element, paragraph_safe),
        Elements::Multiple(elements) if paragraph_safe => {
            stack.push_paragraph_safe_elements(elements);
        }
        Elements::Multiple(elements) => {
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
    if !(stack.current_empty() && element == Element::LineBreak) {
        stack.push_element(element, paragraph_safe);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
