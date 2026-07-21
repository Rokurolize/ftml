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
use crate::tree::{AttributeMap, Container, ContainerType};
use std::mem;

#[derive(Debug, Default)]
pub struct ParagraphStack<'t> {
    /// Elements being accumulated in the current paragraph.
    current: Vec<Element<'t>>,

    /// Whether Wikidot renders the current physical paragraph without a
    /// paragraph wrapper because it contains a naked image block.
    current_unwrapped: bool,

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
    pub fn current_empty(&self) -> bool {
        self.current.is_empty()
    }

    #[cfg(test)]
    pub fn current_capacity(&self) -> usize {
        self.current.capacity()
    }

    #[inline]
    pub fn push_element(&mut self, element: Element<'t>, paragraph_safe: bool) {
        let image_starts_physical_line = matches!(element, Element::Image { .. })
            && (self.current.is_empty()
                || matches!(self.current.last(), Some(Element::LineBreak)));

        if image_starts_physical_line {
            // A naked [[image]] that begins a source line suppresses the
            // wrapper for its contiguous physical paragraph on Wikidot.
            // An image following text on the same line remains inline and
            // paragraph-safe.
            self.current.push(element);
            self.current_unwrapped = true;
        } else if paragraph_safe {
            // Add it to the current (or new) paragraph. Nothing special.
            self.current.push(element);
        } else {
            // This has to be its own "finished" element, outside of any
            // paragraph wrapper. So finish up what we have, then add this element.
            self.end_paragraph();
            self.finished.push(element);
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
        }
    }

    /// Creates a paragraph element out of this instance's current elements.
    pub fn build_paragraph(&mut self) -> Option<Element<'t>> {
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

    /// Set the finished field in this struct to the paragraph element.
    pub fn end_paragraph(&mut self) {
        if self.current_unwrapped {
            self.finished.append(&mut self.current);
            self.current_unwrapped = false;
        } else if let Some(paragraph) = self.build_paragraph() {
            self.finished.push(paragraph);
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
}
