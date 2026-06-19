/*
 * parsing/result.rs
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

use crate::parsing::Parser;
use crate::parsing::error::ParseError;
use crate::tree::{Element, Elements};
use std::marker::PhantomData;

pub type ParseResult<'r, 't, T> = Result<ParseSuccess<'r, 't, T>, ParseError>;
pub type ParseSuccessTuple<T> = (T, Vec<ParseError>, bool);

#[must_use]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParseSuccess<'r, 't, T>
where
    T: 't,
    'r: 't,
{
    pub item: T,
    pub errors: Vec<ParseError>,
    pub paragraph_safe: bool,

    // Marker fields to assert that the 'r lifetime is at least as long as 't.
    #[doc(hidden)]
    _ref_marker: PhantomData<&'r ()>,
    #[doc(hidden)]
    _text_marker: PhantomData<&'t str>,
}

impl<T> ParseSuccess<'_, '_, T> {
    #[inline]
    pub fn new(item: T, errors: Vec<ParseError>, paragraph_safe: bool) -> Self {
        ParseSuccess {
            item,
            errors,
            paragraph_safe,
            _ref_marker: PhantomData,
            _text_marker: PhantomData,
        }
    }

    pub fn chain(
        self,
        all_errors: &mut Vec<ParseError>,
        all_paragraph_safe: &mut bool,
    ) -> T {
        let ParseSuccess {
            item,
            mut errors,
            paragraph_safe,
            ..
        } = self;

        // Append previous errors
        all_errors.append(&mut errors);

        // Update paragraph safety
        *all_paragraph_safe &= paragraph_safe;

        // Return resultant item
        item
    }
}

impl<'r, 't, T> ParseSuccess<'r, 't, T> {
    pub fn map<F, U>(self, f: F) -> ParseSuccess<'r, 't, U>
    where
        F: FnOnce(T) -> U,
    {
        let ParseSuccess {
            item,
            errors,
            paragraph_safe,
            ..
        } = self;

        let new_item = f(item);

        ParseSuccess {
            item: new_item,
            errors,
            paragraph_safe,
            _ref_marker: PhantomData,
            _text_marker: PhantomData,
        }
    }

    #[inline]
    pub fn map_ok<F, U>(self, f: F) -> ParseResult<'r, 't, U>
    where
        F: FnOnce(T) -> U,
    {
        Ok(self.map(f))
    }
}

impl<'t> ParseSuccess<'_, 't, Elements<'t>> {
    pub fn check_partials(&self, parser: &Parser) -> Result<(), ParseError> {
        for element in &self.item {
            // This check only applies if the element is a partial.
            if let Element::Partial(partial) = element {
                // Check if the current rule is looking for a partial.
                if !parser.accepts_partial().matches(partial) {
                    // Found a partial when not looking for one. Raise the appropriate error.
                    return Err(parser.make_err(partial.parse_error_kind()));
                }
            }
        }

        Ok(())
    }
}

impl ParseSuccess<'_, '_, ()> {
    #[inline]
    pub fn into_errors(self) -> Vec<ParseError> {
        self.errors
    }
}

impl<'r, 't, T> From<ParseSuccess<'r, 't, T>> for ParseSuccessTuple<T> {
    #[inline]
    fn from(success: ParseSuccess<'r, 't, T>) -> ParseSuccessTuple<T> {
        let ParseSuccess {
            item,
            errors,
            paragraph_safe,
            ..
        } = success;

        (item, errors, paragraph_safe)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::parsing::{ParseErrorKind, ParserWrap};
    use crate::settings::{WikitextMode, WikitextSettings};
    use crate::tree::{
        AcceptsPartial, AttributeMap, Element, Elements, ListItem, PartialElement,
    };

    #[test]
    fn parse_success_helpers_preserve_state() {
        let mapped =
            ParseSuccess::new(7, Vec::new(), false).map(|value| value.to_string());
        assert_eq!(mapped.item, "7");
        assert!(mapped.errors.is_empty());
        assert!(!mapped.paragraph_safe);

        let mapped_ok = ParseSuccess::new(5, Vec::new(), true)
            .map_ok(|value| value + 1)
            .expect("map_ok should wrap mapped output");
        assert_eq!(mapped_ok.item, 6);
        assert!(mapped_ok.paragraph_safe);

        let (item, errors, paragraph_safe): ParseSuccessTuple<_> =
            ParseSuccess::new("tuple", Vec::new(), true).into();
        assert_eq!(item, "tuple");
        assert!(errors.is_empty());
        assert!(paragraph_safe);

        let mut all_errors = Vec::new();
        let mut all_paragraph_safe = true;
        let chained = ParseSuccess::new("chained", Vec::new(), false)
            .chain(&mut all_errors, &mut all_paragraph_safe);
        assert_eq!(chained, "chained");
        assert!(all_errors.is_empty());
        assert!(!all_paragraph_safe);

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("errors");
        let parser = Parser::new(&tokenization, &page_info, &settings);
        let existing_error = parser.make_err(ParseErrorKind::NoRulesMatch);
        let appended_error = parser.make_err(ParseErrorKind::RuleFailed);
        let mut all_errors = vec![existing_error.clone()];
        let mut all_paragraph_safe = true;
        let chained = ParseSuccess::new("errors", vec![appended_error.clone()], true)
            .chain(&mut all_errors, &mut all_paragraph_safe);
        assert_eq!(chained, "errors");
        assert_eq!(all_errors, vec![existing_error, appended_error]);
        assert!(all_paragraph_safe);

        assert!(
            ParseSuccess::new((), Vec::new(), true)
                .into_errors()
                .is_empty()
        );
    }

    #[test]
    fn check_partials_accepts_matching_partial_only() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("* item");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);

        let item = Element::Partial(PartialElement::ListItem(ListItem::Elements {
            attributes: AttributeMap::new(),
            elements: vec![text!("item")],
        }));

        let rejected =
            ParseSuccess::new(Elements::Single(item.clone()), Vec::new(), true)
                .check_partials(&parser);
        let error = rejected.expect_err("partial should be rejected outside its parent");
        assert_eq!(error.kind(), ParseErrorKind::ListItemOutsideList);

        {
            let parser = ParserWrap::new(&mut parser, AcceptsPartial::ListItem);
            ParseSuccess::new(Elements::Single(item), Vec::new(), true)
                .check_partials(&parser)
                .expect("matching partial should be accepted");
        }

        ParseSuccess::new(Elements::None, Vec::new(), true)
            .check_partials(&parser)
            .expect("plain element list should not require a partial context");
    }
}
