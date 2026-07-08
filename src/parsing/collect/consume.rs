/*
 * parsing/collect/consume.rs
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

/// Convenience wrapper around `collect()` to consume each token iteration.
///
/// Since simply consuming to produce an `Element<'t>` is a typical pattern,
/// this function implements it here to avoid code duplication.
///
/// This call always sets `step_on_final` to `true`.
pub fn collect_consume<'r, 't>(
    parser: &mut Parser<'r, 't>,
    rule: Rule,
    closes: &[ParseCondition],
    invalids: &[ParseCondition],
    kind: Option<ParseErrorKind>,
) -> ParseResult<'r, 't, Vec<Element<'t>>> {
    let success = collect_consume_keep(parser, rule, closes, invalids, kind)?;
    Ok(success.map(|(elements, _)| elements))
}

/// Modified form of `collect_consume()` that also returns the last token.
///
/// The last token terminating the collection is kept, and returned
/// to the caller alongside the string slice.
///
/// Compare with `collect_text_keep()`.
pub fn collect_consume_keep<'r, 't>(
    parser: &mut Parser<'r, 't>,
    rule: Rule,
    closes: &[ParseCondition],
    invalids: &[ParseCondition],
    kind: Option<ParseErrorKind>,
) -> ParseResult<'r, 't, (Vec<Element<'t>>, &'r ExtractedToken<'t>)> {
    let mut all_elements = Vec::new();

    let collection = collect(parser, rule, closes, invalids, kind, |parser| {
        consume(parser)?.map_ok(|elements| append_elements(&mut all_elements, elements))
    })?;
    let (last, errors, paragraph_safe) = collection.into();

    let item = (all_elements, last);
    Ok(ParseSuccess::new(item, errors, paragraph_safe))
}

fn append_elements<'t>(all_elements: &mut Vec<Element<'t>>, elements: Elements<'t>) {
    match elements {
        Elements::None => all_elements.reserve(0),
        Elements::Single(element) => all_elements.push(element),
        Elements::Multiple(mut elements) => {
            if all_elements.is_empty() {
                *all_elements = elements;
            } else {
                all_elements.append(&mut elements);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_elements_adopts_first_multiple_vector() {
        let mut all_elements = Vec::new();
        append_elements(&mut all_elements, Elements::None);
        assert!(all_elements.is_empty());

        append_elements(
            &mut all_elements,
            Elements::Multiple(vec![text!("a"), text!("b")]),
        );

        let capacity = all_elements.capacity();

        append_elements(&mut all_elements, Elements::Single(text!("c")));
        append_elements(&mut all_elements, Elements::Multiple(vec![text!("d")]));

        assert_eq!(
            all_elements,
            vec![text!("a"), text!("b"), text!("c"), text!("d")],
        );
        assert!(all_elements.capacity() >= capacity);
    }
}
