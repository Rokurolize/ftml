/*
 * parsing/rule/impls/entity.rs
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

use entities::ENTITIES;
use std::borrow::Cow;
use std::char;
use std::collections::HashMap;
use std::sync::LazyLock;

static ENTITY_MAPPING: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| {
        let mut mapping = HashMap::new();

        for entity in &ENTITIES {
            let key = strip_entity(entity.entity);
            let value = entity.characters;

            mapping.insert(key, value);
        }

        mapping
    });

pub(crate) fn decode_semicolon_entities(s: &str) -> Cow<'_, str> {
    let mut output = None;
    let mut start = 0;
    let mut offset = 0;

    while let Some(relative_entity_start) = s[offset..].find('&') {
        let entity_start = offset + relative_entity_start;
        let entity_body_start = entity_start + 1;
        let Some(relative_entity_end) = s[entity_body_start..].find(';') else {
            break;
        };

        let entity_end = entity_body_start + relative_entity_end;
        let entity = &s[entity_body_start..entity_end];

        if let Some(decoded) = find_entity(entity) {
            let output = output.get_or_insert_with(|| String::with_capacity(s.len()));
            output.push_str(&s[start..entity_start]);
            output.push_str(&decoded);
            start = entity_end + 1;
        }

        offset = entity_end + 1;
    }

    match output {
        Some(mut output) => {
            output.push_str(&s[start..]);
            Cow::Owned(output)
        }
        None => Cow::Borrowed(s),
    }
}

/// Find the string corresponding to the passed entity, if any.
pub(crate) fn find_entity(entity: &str) -> Option<Cow<'_, str>> {
    // Named entity
    if let Some(result) = ENTITY_MAPPING.get(entity) {
        return Some(cow!(result));
    }

    // Hexadecimal entity
    if let Some(value) = entity.strip_prefix("#x")
        && let Some(result) = get_char(value, 16)
    {
        return Some(result);
    }

    // Decimal entity
    if let Some(value) = entity.strip_prefix('#')
        && let Some(result) = get_char(value, 10)
    {
        return Some(result);
    }

    // Not found
    None
}

/// Gets the appropriate character from the number specified in the string.
///
/// Using the passed radix, it gets the integer value, then finds the appropriate
/// character, if one exists.
///
/// Then converts the character into a string with only that value.
fn get_char(value: &str, radix: u32) -> Option<Cow<'_, str>> {
    let codepoint = match u32::from_str_radix(value, radix) {
        Ok(codepoint) => codepoint,
        Err(_) => return None,
    };

    let ch = char::from_u32(codepoint)?;
    Some(Cow::Owned(ch.to_string()))
}

/// If a string starts with `&` or ends with `;`, those are removed.
/// First trims the string of whitespace.
pub(crate) fn strip_entity(mut s: &str) -> &str {
    s = s.trim();

    if let Some(stripped) = s.strip_prefix('&') {
        s = stripped;
    }

    if let Some(stripped) = s.strip_suffix(';') {
        s = stripped;
    }

    s
}

#[test]
fn test_get_entity() {
    macro_rules! test {
        ($input:expr, $expected:expr $(,)?) => {{
            let actual = find_entity($input);
            let expected = $expected;

            assert_eq!(
                actual, expected,
                "Actual entity string doesn't match expected",
            );
        }};
    }

    test!("", None);

    // Names
    test!("amp", Some(cow!("&")));
    test!("lt", Some(cow!("<")));
    test!("gt", Some(cow!(">")));
    test!("copy", Some(cow!("\u{a9}")));
    test!("xxxzzz", None);

    // Decimal
    test!("#32", Some(cow!(" ")));
    test!("#255", Some(cow!("\u{ff}")));
    test!("#128175", Some(cow!("\u{1f4af}")));
    test!("#2097151", None);

    // Hex
    test!("#x20", Some(cow!(" ")));
    test!("#xff", Some(cow!("\u{ff}")));
    test!("#x1f4af", Some(cow!("\u{1f4af}")));
    test!("#x1fffff", None);
}

#[test]
fn test_get_char() {
    macro_rules! test {
        ($value:expr, $radix:expr, $expected:expr $(,)?) => {{
            let actual = get_char($value, $radix);
            let expected = $expected;

            assert_eq!(
                actual, expected,
                "Actual character value doesn't match expected",
            );
        }};
    }

    // Decimal
    test!("32", 10, Some(Cow::Owned(str!(' '))));
    test!("255", 10, Some(Cow::Owned(str!('\u{ff}'))));
    test!("128175", 10, Some(Cow::Owned(str!('\u{1f4af}'))));
    test!("2097151", 10, None);

    // Hex
    test!("20", 16, Some(Cow::Owned(str!(' '))));
    test!("ff", 16, Some(Cow::Owned(str!('\u{ff}'))));
    test!("1f4af", 16, Some(Cow::Owned(str!('\u{1f4af}'))));
    test!("1fffff", 16, None);
}

#[test]
fn test_strip_entity() {
    macro_rules! test {
        ($input:expr, $expected:expr $(,)?) => {{
            let actual = strip_entity($input);
            let expected = $expected;

            assert_eq!(
                actual, expected,
                "Actual stripped entity value didn't match expected",
            );
        }};
    }

    test!("", "");
    test!("abc", "abc");
    test!("legumes1", "legumes1");
    test!("&amp;", "amp");
    test!("&#100;", "#100");
    test!("&xdeadbeef;", "xdeadbeef");

    test!("&amp", "amp");
    test!("amp;", "amp");
    test!("&#100", "#100");
    test!("#100;", "#100");

    test!(" &amp; ", "amp");
}

#[test]
fn decode_semicolon_entities_decodes_complete_valid_entities() {
    assert_eq!(
        decode_semicolon_entities("&copy; &#252; &#8212; &#x2014;"),
        "\u{a9} \u{fc} \u{2014} \u{2014}",
    );
}

#[test]
fn decode_semicolon_entities_preserves_unknown_and_incomplete_entities() {
    assert_eq!(
        decode_semicolon_entities("&copy &not-an-entity; &copy"),
        "&copy &not-an-entity; &copy",
    );
}
