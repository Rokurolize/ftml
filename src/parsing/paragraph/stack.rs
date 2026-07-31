/*
 * parsing/paragraph/stack.rs
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

use crate::parsing::prelude::*;
use crate::tree::{Alignment, AttributeMap, Container, ContainerType, TableType};
use std::mem;

pub(crate) fn collapsible_has_direct_literal_nested_opener(
    element: &Element<'_>,
) -> bool {
    let Element::Collapsible { elements, .. } = element else {
        return false;
    };
    elements.iter().any(|element| {
        let Element::Container(container) = element else {
            return false;
        };
        if container.ctype() != ContainerType::Paragraph {
            return false;
        }
        let literal: String = container
            .elements()
            .iter()
            .filter_map(|element| match element {
                Element::Text(text) | Element::Raw(text) => Some(text.as_ref()),
                _ => None,
            })
            .collect();
        literal.contains("[[collapsible")
    })
}

#[derive(Debug, Default)]
pub struct ParagraphStack<'t> {
    wikidot: bool,

    /// Elements being accumulated in the current paragraph.
    current: Vec<Element<'t>>,

    /// Whether Wikidot renders the current physical paragraph without a
    /// paragraph wrapper because it contains a naked image block.
    current_unwrapped: bool,

    /// Whether the current physical line contains an invisible C0 control.
    ///
    /// Wikidot drops these controls from output, but a control-only line is
    /// still occupied and therefore preserves its following line break.
    current_has_discarded_control: bool,

    pending_unwrapped_separator: bool,
    unwrapped_after_block_line: bool,
    wikidot_literal_iftags_line: bool,
    wikidot_literal_div_line: bool,
    trim_unwrapped_trailing_line_break: bool,
    suppress_next_line_break: bool,

    /// Previous elements created, to be outputted in the final [`SyntaxTree`].
    finished: Vec<Element<'t>>,

    /// Gathered errors from paragraph parsing.
    errors: Vec<ParseError>,
}

impl<'t> ParagraphStack<'t> {
    #[inline]
    pub fn new() -> Self {
        ParagraphStack::default()
    }

    #[inline]
    pub fn new_wikidot() -> Self {
        ParagraphStack {
            wikidot: true,
            ..ParagraphStack::default()
        }
    }

    #[inline]
    pub fn current_empty(&self) -> bool {
        self.current.is_empty() && !self.current_has_discarded_control
    }

    #[inline]
    pub(crate) fn wikidot_line_break_follows_block(&self) -> bool {
        self.wikidot
            && self.current_empty()
            && self.finished.last().is_some_and(|element| {
                matches!(
                    element,
                    Element::Container(container)
                        if matches!(
                            container.ctype(),
                            ContainerType::Div
                                | ContainerType::Blockquote
                                | ContainerType::Align(_)
                        )
                ) || matches!(element, Element::Code(_) | Element::Collapsible { .. })
            })
    }

    #[cfg(test)]
    pub fn current_capacity(&self) -> usize {
        self.current.capacity()
    }

    #[inline]
    pub fn mark_current_unwrapped(&mut self) {
        if !self.current.is_empty() {
            self.current_unwrapped = true;
        }
    }

    #[inline]
    pub fn mark_wikidot_terminal_backslash(&mut self) {
        if self.wikidot && !self.current.is_empty() && !self.current_unwrapped {
            let separator = if self.finished.is_empty() {
                "\n\n"
            } else {
                "\n"
            };
            self.current.insert(0, text!(separator));
            self.current_unwrapped = true;
        }
    }

    /// End the preceding inline run without a paragraph wrapper while keeping
    /// the source newline as insignificant HTML whitespace.
    #[inline]
    pub fn mark_wikidot_continued_block_boundary(&mut self) {
        if self.wikidot && !self.current.is_empty() {
            self.current.push(text!("\n"));
            self.current_unwrapped = true;
        }
    }

    #[inline]
    pub(crate) fn mark_next_unwrapped(&mut self) {
        self.current_unwrapped = true;
    }

    #[inline]
    pub(crate) fn mark_next_unwrapped_after_block(&mut self) {
        self.current_unwrapped = true;
        self.unwrapped_after_block_line = true;
    }

    #[inline]
    pub(crate) fn mark_wikidot_literal_div_line(&mut self) {
        if self.wikidot && !self.wikidot_literal_iftags_line {
            if !self.finished.is_empty() && !self.current.is_empty() {
                self.current.insert(0, text!("\n"));
            }
            self.current_unwrapped = true;
            self.wikidot_literal_div_line = true;
        }
    }

    #[inline]
    pub(crate) fn mark_wikidot_literal_iftags_line(&mut self) {
        if self.wikidot {
            self.wikidot_literal_iftags_line = true;
        }
    }

    #[inline]
    pub fn mark_wikidot_literal_list_item(&mut self) {
        if self.wikidot {
            self.mark_current_unwrapped();
        }
    }

    #[inline]
    pub fn mark_wikidot_tabview_boundary(&mut self) {
        if !self.wikidot || self.current.is_empty() {
            return;
        }
        self.trim_trailing_ascii_space();
        if !matches!(self.current.last(), Some(Element::LineBreak)) {
            self.current.push(Element::LineBreak);
        }
        self.current_unwrapped = true;
    }

    #[inline]
    pub fn mark_discarded_control(&mut self) {
        if self.wikidot {
            self.current_has_discarded_control = true;
        }
    }

    #[inline]
    pub fn push_element(&mut self, element: Element<'t>, paragraph_safe: bool) {
        if self.suppress_next_line_break {
            self.suppress_next_line_break = false;
            if element == Element::LineBreak {
                return;
            }
        }
        let aligned_image = matches!(
            element,
            Element::Image {
                alignment: Some(_),
                ..
            }
        ) || matches!(&element, Element::Delayed(delayed) if matches!(delayed.image_alignment(), Some(Some(_))));
        let unaligned_image = matches!(
            element,
            Element::Image {
                alignment: None,
                ..
            }
        ) || matches!(&element, Element::Delayed(delayed) if delayed.image_alignment() == Some(None));
        let wikidot_html_block = self.wikidot && matches!(element, Element::Html { .. });
        let wikidot_section_marker = self.wikidot
            && matches!(
                &element,
                Element::Container(container)
                    if container.ctype() == ContainerType::Div
                        && container
                            .attributes()
                            .get()
                            .get("class")
                            .is_some_and(|value| value.as_ref() == "content-separator")
            );
        let wikidot_simple_table = self.wikidot
            && matches!(
                &element,
                Element::Table(table) if table.table_type == TableType::Simple
            );
        let wikidot_center_alignment = self.wikidot
            && matches!(
                &element,
                Element::Container(container)
                    if container.ctype() == ContainerType::Align(Alignment::Center)
            );

        if self.wikidot
            && matches!(element, Element::DefinitionList(_))
            && !self.current.is_empty()
        {
            self.trim_trailing_ascii_space();
            self.current.push(Element::LineBreak);
            self.current_unwrapped = true;
        }

        if wikidot_simple_table && !self.current.is_empty() {
            self.trim_trailing_ascii_space();
            self.current.push(Element::LineBreak);
            self.current_unwrapped = true;
        }

        if wikidot_center_alignment
            && !self.current.is_empty()
            && !matches!(self.current.last(), Some(Element::LineBreak))
        {
            self.trim_trailing_ascii_space();
            self.current.push(Element::LineBreak);
        }

        if self.wikidot
            && matches!(
                element,
                Element::Container(ref container)
                    if matches!(container.ctype(), ContainerType::Align(_))
            )
            && self.current.is_empty()
            && matches!(
                self.finished.last(),
                Some(Element::Container(container))
                    if matches!(container.ctype(), ContainerType::Align(_))
            )
        {
            self.finished.push(Element::LineBreak);
        }

        if matches!(element, Element::TabView(_)) {
            self.mark_wikidot_tabview_boundary();
        }

        if wikidot_html_block {
            self.end_paragraph();
            self.current.push(element);
        } else if aligned_image {
            self.end_paragraph();
            self.finished.push(element);
        } else if unaligned_image {
            // Any unaligned image suppresses the paragraph wrapper for its
            // contiguous physical paragraph on Wikidot.
            if self.wikidot
                && self.finished.is_empty()
                && !self.current.is_empty()
                && !matches!(self.current.first(), Some(Element::Text(text)) if text.starts_with("\n\n"))
            {
                self.current.insert(0, text!("\n\n"));
            }
            if self.pending_unwrapped_separator
                && matches!(self.finished.last(), Some(Element::Text(text)) if text == " ")
            {
                self.finished.pop();
            }
            self.pending_unwrapped_separator = false;
            self.current.push(element);
            self.current_unwrapped = true;
        } else if paragraph_safe {
            // Add it to the current (or new) paragraph. Nothing special.
            if self.wikidot
                && matches!(element, Element::Text(ref text) if text == " ")
                && (self.current.is_empty()
                    || matches!(
                        self.current.last(),
                        Some(Element::Text(text)) if text == " "
                    )
                    || matches!(self.current.last(), Some(Element::LineBreak)))
            {
                return;
            }
            if element == Element::LineBreak {
                self.trim_trailing_ascii_space();
            }
            self.current.push(element);
        } else {
            // This has to be its own "finished" element, outside of any
            // paragraph wrapper. So finish up what we have, then add this element.
            let invisible_block_line = self.wikidot && element == Element::LineBreak;
            let nested_literal_collapsible =
                self.wikidot && collapsible_has_direct_literal_nested_opener(&element);
            if wikidot_section_marker
                && self.current_unwrapped
                && !self.current.is_empty()
                && !matches!(self.current.last(), Some(Element::LineBreak))
            {
                self.current.push(text!("\n"));
            }
            if invisible_block_line {
                self.pop_line_break();
            }
            self.end_paragraph();
            self.finished.push(element);
            if invisible_block_line {
                self.current_unwrapped = true;
                self.trim_unwrapped_trailing_line_break = true;
            } else if nested_literal_collapsible {
                self.current.push(Element::LineBreak);
                self.current_unwrapped = true;
                self.suppress_next_line_break = true;
            }
        }
    }

    pub fn push_paragraph_safe_elements(&mut self, mut elements: Vec<Element<'t>>) {
        if self.current.is_empty() {
            if let Some(index) = elements
                .iter()
                .position(|element| *element != Element::LineBreak)
            {
                if index != 0 {
                    elements.drain(..index);
                }
                self.current = elements;
            }
        } else {
            self.current.append(&mut elements);
        }
    }

    #[inline]
    pub fn push_errors(&mut self, errors: &mut Vec<ParseError>) {
        self.errors.append(errors);
    }

    /// Remove the trailing line break if one exists.
    ///
    /// Exclusively for native blockquote logic, since
    /// it needs to build blockquotes but also strip
    /// excess line breaks.
    ///
    /// This should only be between lines in the blockquote.
    #[inline]
    pub fn pop_line_break(&mut self) {
        if let Some(Element::LineBreak) = self.current.last() {
            self.current.pop();
        } else if self.wikidot
            && self.current.is_empty()
            && matches!(self.finished.last(), Some(Element::LineBreak))
        {
            self.finished.pop();
        }
    }

    pub(crate) fn ensure_wikidot_trailing_line_break(&mut self) {
        if !self.wikidot {
            return;
        }
        if !self.current.is_empty() {
            if !matches!(self.current.last(), Some(Element::LineBreak)) {
                self.current.push(Element::LineBreak);
            }
            self.trim_unwrapped_trailing_line_break = false;
        } else if matches!(self.finished.last(), Some(Element::Text(_))) {
            self.finished.push(Element::LineBreak);
        }
    }

    /// Creates a paragraph element out of this instance's current elements.
    pub fn build_paragraph(&mut self) -> Option<Element<'t>> {
        self.trim_trailing_ascii_space();
        // Don't create empty paragraphs
        if self.current.is_empty() {
            return None;
        }

        // Pull out gathered elements, then make a new paragraph container
        let elements = mem::take(&mut self.current);
        let attributes = AttributeMap::new();
        let container = Container::new(ContainerType::Paragraph, elements, attributes);
        let element = Element::Container(container);
        Some(element)
    }

    fn trim_trailing_ascii_space(&mut self) {
        if matches!(self.current.last(), Some(Element::Text(text)) if text == " ") {
            self.current.pop();
        }
    }

    /// Set the finished field in this struct to the paragraph element.
    pub fn end_paragraph(&mut self) {
        if self.current_unwrapped {
            if self.trim_unwrapped_trailing_line_break
                && matches!(self.current.last(), Some(Element::LineBreak))
            {
                self.current.pop();
            }
            self.finished.append(&mut self.current);
            self.current_unwrapped = false;
        } else if let Some(paragraph) = self.build_paragraph() {
            self.finished.push(paragraph);
        }
        self.current_has_discarded_control = false;
        self.pending_unwrapped_separator = false;
        self.unwrapped_after_block_line = false;
        self.wikidot_literal_iftags_line = false;
        self.wikidot_literal_div_line = false;
        self.trim_unwrapped_trailing_line_break = false;
        self.suppress_next_line_break = false;
    }

    pub fn end_paragraph_at_break(&mut self) {
        let unwrapped = self.current_unwrapped && !self.current.is_empty();
        let literal_div_line = self.wikidot_literal_div_line;
        let unwrapped_after_block_line = self.unwrapped_after_block_line;
        self.end_paragraph();
        if unwrapped {
            self.finished
                .push(if literal_div_line || unwrapped_after_block_line {
                    text!("\n")
                } else {
                    text!(" ")
                });
            self.pending_unwrapped_separator = true;
        }
    }

    /// Convert all paragraph context into a `ParseResult.`
    ///
    /// This returns all collected elements, errors, and returns the final
    /// paragraph safety value.
    pub fn into_result<'r>(mut self) -> ParseResult<'r, 't, Vec<Element<'t>>> {
        // Finish current paragraph, if any
        self.end_paragraph();

        // Deconstruct stack
        let elements = self.finished;
        let errors = self.errors;

        // If this has any paragraphs in it, or other incompatible elements,
        // it's not fit to be wrapped in <p>.
        //
        // Otherwise it's just a listing of internal elements.
        // This is definitely not the common case here, this mostly will happen
        // if the element list is empty.
        let paragraph_safe = elements.iter().all(|element| element.paragraph_safe());

        // Return finished element list
        ok!(paragraph_safe; elements, errors)
    }

    /// Converts all paragraph context into a set of `Element`s.
    ///
    /// You should only use this if you know for sure there are no errors,
    /// and either have an alternate means of determining paragraph safety, or
    /// statically know what that value would be.
    pub fn into_elements(mut self) -> Vec<Element<'t>> {
        // Finish current paragraph, if any
        self.end_paragraph();

        // Check that there are no errors
        debug_assert!(self.errors.is_empty(), "ParagraphStack errors");

        // Deconstruct stack, return
        self.finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paragraph<'t>(elements: Vec<Element<'t>>) -> Element<'t> {
        Element::Container(Container::new(
            ContainerType::Paragraph,
            elements,
            AttributeMap::new(),
        ))
    }

    #[test]
    fn pop_line_break_removes_trailing_break_before_finishing() {
        let mut stack = ParagraphStack::new();

        stack.push_element(text!("alpha"), true);
        stack.push_element(Element::LineBreak, true);
        stack.pop_line_break();

        assert_eq!(stack.into_elements(), vec![paragraph(vec![text!("alpha")])]);
    }

    #[test]
    fn non_paragraph_safe_elements_finish_pending_paragraph() {
        let mut stack = ParagraphStack::new();

        stack.push_element(text!("alpha"), true);
        stack.push_element(Element::HorizontalRule, false);

        assert_eq!(
            stack.into_elements(),
            vec![paragraph(vec![text!("alpha")]), Element::HorizontalRule],
        );
    }

    #[test]
    fn an_empty_unsafe_construct_can_leave_the_current_paragraph_unwrapped() {
        let mut stack = ParagraphStack::new();
        stack.push_element(text!("alpha"), true);
        stack.push_element(Element::LineBreak, true);
        stack.mark_current_unwrapped();

        assert_eq!(
            stack.into_elements(),
            vec![text!("alpha"), Element::LineBreak],
        );
    }

    #[test]
    fn an_empty_unsafe_construct_does_not_unwrap_future_content() {
        let mut stack = ParagraphStack::new();
        stack.mark_current_unwrapped();
        stack.push_element(text!("alpha"), true);

        assert_eq!(stack.into_elements(), vec![paragraph(vec![text!("alpha")])]);
    }

    #[test]
    fn paragraph_safe_elements_adopt_vectors_and_skip_empty_leading_breaks() {
        let mut stack = ParagraphStack::new();

        stack.push_paragraph_safe_elements(vec![
            Element::LineBreak,
            text!("alpha"),
            Element::LineBreak,
        ]);

        assert_eq!(
            stack.into_elements(),
            vec![paragraph(vec![text!("alpha"), Element::LineBreak])],
        );
    }

    #[test]
    fn paragraph_safe_elements_ignore_all_leading_breaks() {
        let mut stack = ParagraphStack::new();

        stack.push_paragraph_safe_elements(vec![Element::LineBreak, Element::LineBreak]);

        assert_eq!(stack.into_elements(), Vec::<Element>::new());
    }

    #[test]
    fn paragraph_safe_elements_append_to_existing_paragraph() {
        let mut stack = ParagraphStack::new();

        stack.push_element(text!("alpha"), true);
        stack.push_paragraph_safe_elements(vec![Element::LineBreak, text!("beta")]);

        assert_eq!(
            stack.into_elements(),
            vec![paragraph(vec![
                text!("alpha"),
                Element::LineBreak,
                text!("beta"),
            ])],
        );
    }

    #[test]
    fn empty_inline_construct_space_is_trimmed_before_break_or_paragraph_end() {
        let mut before_break = ParagraphStack::new();
        before_break.push_element(text!("alpha"), true);
        before_break.push_element(text!(" "), true);
        before_break.push_element(Element::LineBreak, true);
        before_break.push_element(text!("beta"), true);
        assert_eq!(
            before_break.into_elements(),
            vec![paragraph(vec![
                text!("alpha"),
                Element::LineBreak,
                text!("beta"),
            ])],
        );

        let mut at_end = ParagraphStack::new();
        at_end.push_element(text!("alpha"), true);
        at_end.push_element(text!(" "), true);
        assert_eq!(
            at_end.into_elements(),
            vec![paragraph(vec![text!("alpha")])],
        );
    }

    #[test]
    fn discarded_control_trims_surrounding_spaces_but_preserves_a_line_break() {
        let mut stack = ParagraphStack::new_wikidot();
        stack.mark_discarded_control();
        stack.push_element(text!(" "), true);
        stack.push_element(Element::LineBreak, true);
        stack.push_element(text!("omega"), true);

        assert_eq!(
            stack.into_elements(),
            vec![paragraph(vec![Element::LineBreak, text!("omega")])],
        );

        let mut between_spaces = ParagraphStack::new_wikidot();
        between_spaces.push_element(text!("alpha"), true);
        between_spaces.push_element(text!(" "), true);
        between_spaces.mark_discarded_control();
        between_spaces.push_element(text!(" "), true);
        between_spaces.push_element(text!("omega"), true);

        assert_eq!(
            between_spaces.into_elements(),
            vec![paragraph(vec![text!("alpha"), text!(" "), text!("omega")])],
        );
    }

    #[test]
    fn paragraph_break_after_unwrapped_image_preserves_wikidot_separator() {
        let mut stack = ParagraphStack::new();
        stack.push_element(text!("alpha"), true);
        stack.current_unwrapped = true;
        stack.end_paragraph_at_break();
        stack.push_element(text!("beta"), true);

        assert_eq!(
            stack.into_elements(),
            vec![text!("alpha"), text!(" "), paragraph(vec![text!("beta")])],
        );
    }

    #[test]
    fn wikidot_definition_list_unwraps_adjacent_prose_with_a_break() {
        let mut stack = ParagraphStack::new_wikidot();
        stack.push_element(text!("alpha"), true);
        stack.push_element(Element::DefinitionList(Vec::new()), false);

        assert_eq!(
            stack.into_elements(),
            vec![
                text!("alpha"),
                Element::LineBreak,
                Element::DefinitionList(Vec::new()),
            ],
        );
    }
}
