/*
 * parsing/rule/impls/block/arguments.rs
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

use crate::parsing::{ParseError, ParseErrorKind, Parser, parse_boolean};
use crate::settings::WikitextSettings;
use crate::tree::{AttributeMap, RawModuleArgument};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::str::FromStr;

macro_rules! make_err {
    ($parser:expr) => {
        $parser.make_err(ParseErrorKind::BlockMalformedArguments)
    };
}

fn normalize_wikidot_html_attribute_value<'t>(value: &Cow<'t, str>) -> Cow<'t, str> {
    let input = value.as_ref();
    let mut output = String::with_capacity(input.len());
    let mut pending_space = false;
    let mut changed = false;

    for ch in input.chars() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                if output.is_empty() {
                    changed = true;
                    continue;
                }
                if pending_space || ch != ' ' {
                    changed = true;
                }
                pending_space = true;
            }
            '\0'..='\u{001F}' | '\u{007F}' => {
                changed = true;
            }
            _ => {
                if pending_space {
                    output.push(' ');
                    pending_space = false;
                }
                output.push(ch);
            }
        }
    }

    if pending_space {
        changed = true;
    }

    if changed {
        Cow::Owned(output)
    } else {
        value.clone()
    }
}

#[derive(Debug, Clone, Copy)]
struct ArgumentKey<'t> {
    value: &'t str,
    case_sensitive: bool,
}

impl<'t> ArgumentKey<'t> {
    #[inline]
    fn as_str(self) -> &'t str {
        self.value
    }
}

impl PartialEq for ArgumentKey<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.case_sensitive == other.case_sensitive
            && if self.case_sensitive {
                self.value == other.value
            } else {
                self.value.eq_ignore_ascii_case(other.value)
            }
    }
}

impl Eq for ArgumentKey<'_> {}

impl Hash for ArgumentKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.case_sensitive.hash(state);
        self.value.len().hash(state);
        for byte in self.value.bytes() {
            state.write_u8(if self.case_sensitive {
                byte
            } else {
                byte.to_ascii_lowercase()
            });
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Arguments<'t> {
    inner: HashMap<ArgumentKey<'t>, Cow<'t, str>>,
    raw: Vec<RawModuleArgument<'t>>,
    bare: HashSet<ArgumentKey<'t>>,
    case_sensitive: bool,
    source_present: bool,
    spaced_equals: bool,
}

impl<'t> Arguments<'t> {
    #[inline]
    pub fn new() -> Self {
        Arguments::default()
    }

    #[inline]
    pub(crate) fn new_case_sensitive() -> Self {
        Arguments {
            case_sensitive: true,
            ..Arguments::default()
        }
    }

    #[inline]
    fn key(&self, key: &'t str) -> ArgumentKey<'t> {
        ArgumentKey {
            value: key,
            case_sensitive: self.case_sensitive,
        }
    }

    /// Inserts a key / value pair into the list of arguments.
    pub fn insert(&mut self, key: &'t str, value: Cow<'t, str>) {
        self.source_present = true;
        let argument_key = self.key(key);
        self.bare.remove(&argument_key);
        self.raw.push(RawModuleArgument {
            name: cow!(key),
            value: value.clone(),
        });
        self.inner.insert(argument_key, value);
    }

    pub fn insert_bare(&mut self, key: &'t str, value: Cow<'t, str>) {
        self.insert(key, value);
        self.bare.insert(self.key(key));
    }

    /// Gets **and removes** a string value from the arguments from its key.
    #[must_use = "non-idempotent getter method"]
    pub fn get(&mut self, key: &'t str) -> Option<Cow<'t, str>> {
        let key = self.key(key);
        self.bare.remove(&key);
        self.inner.remove(&key)
    }

    pub fn get_with_bare(&mut self, key: &'t str) -> Option<(Cow<'t, str>, bool)> {
        let key = self.key(key);
        let bare = self.bare.remove(&key);
        self.inner.remove(&key).map(|value| (value, bare))
    }

    /// Gets **and removes** a boolean value from the arguments from its the key.
    #[must_use = "non-idempotent getter method"]
    pub fn get_bool(
        &mut self,
        parser: &Parser<'_, 't>,
        key: &'t str,
    ) -> Result<Option<bool>, ParseError> {
        match self.get(key) {
            Some(argument) => match parse_boolean(argument) {
                Ok(value) => Ok(Some(value)),
                Err(_) => Err(make_err!(parser)),
            },
            None => Ok(None),
        }
    }

    /// Gets **and removes** a parseable value from the arguments from its key.
    #[must_use = "non-idempotent getter method"]
    pub fn get_value<T: FromStr>(
        &mut self,
        parser: &Parser<'_, 't>,
        key: &'t str,
    ) -> Result<Option<T>, ParseError> {
        match self.get(key) {
            Some(argument) => match argument.parse() {
                Ok(value) => Ok(Some(value)),
                Err(_) => Err(make_err!(parser)),
            },
            None => Ok(None),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline]
    pub(crate) fn mark_source_present(&mut self) {
        self.source_present = true;
    }

    #[inline]
    pub(crate) fn has_source(&self) -> bool {
        self.source_present
    }

    pub fn mark_spaced_equals(&mut self) {
        self.spaced_equals = true;
    }

    pub fn has_spaced_equals(&self) -> bool {
        self.spaced_equals
    }

    /// Produces a separate hash map of argument keys to values.
    ///
    /// This returns a new `HashMap` suitable for inclusion in final `Element`s.
    /// It does not clone any string allocations, as they are all borrowed
    /// (or already owned, per `Cow`).
    /// It only makes a new allocation for the new `HashMap`.
    pub fn to_hash_map(&self) -> HashMap<Cow<'t, str>, Cow<'t, str>> {
        self.inner
            .iter()
            .map(|(key, value)| {
                let key = cow!(key.as_str());
                let value = value.clone();

                (key, value)
            })
            .collect()
    }

    pub fn into_raw_vec(self) -> Vec<RawModuleArgument<'t>> {
        self.raw
    }

    /// Similar to `to_hash_map()`, but creates an `AttributeMap` instead.
    ///
    /// Because all fields are passed from the user, this does ID isolation
    /// if that is enabled, and so needs `WikitextSettings` to be passed in.
    #[inline]
    pub fn to_attribute_map(&self, settings: &WikitextSettings) -> AttributeMap<'t> {
        let mut map = self.attribute_map_from_entries(settings, |_| true);
        map.isolate_id(settings);
        map
    }

    pub fn to_wikidot_anchor_attribute_map(
        &self,
        settings: &WikitextSettings,
    ) -> AttributeMap<'t> {
        let mut map = self.to_attribute_map(settings);
        map.remove("title");
        let href = self.inner.get(&self.key("href")).filter(|value| {
            matches!(
                crate::url::classify_href(value),
                crate::url::HrefKind::Relative
            ) && !value.contains(':')
        });
        if let Some(href) = href {
            map.insert_wikidot_relative_href(normalize_wikidot_html_attribute_value(
                href,
            ));
        }
        map.isolate_id(settings);
        map
    }

    pub fn to_attribute_map_without_bare(
        &self,
        settings: &WikitextSettings,
    ) -> AttributeMap<'t> {
        let mut map =
            self.attribute_map_from_entries(settings, |key| !self.bare.contains(key));
        map.isolate_id(settings);
        map
    }

    fn attribute_map_from_entries(
        &self,
        settings: &WikitextSettings,
        include: impl Fn(&ArgumentKey<'t>) -> bool,
    ) -> AttributeMap<'t> {
        let mut attributes = AttributeMap::new();

        for (key, value) in &self.inner {
            if !include(key) {
                continue;
            }

            if settings.layout.legacy() && contains_unresolved_wikidot_variable(value) {
                continue;
            }

            let key = key.as_str();
            if self.case_sensitive && key.bytes().any(|byte| byte.is_ascii_uppercase()) {
                continue;
            }

            let value = if settings.layout.legacy() {
                normalize_wikidot_html_attribute_value(value)
            } else {
                value.clone()
            };
            attributes.insert(key, value);
        }

        attributes
    }
}

fn contains_unresolved_wikidot_variable(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index + 3 < bytes.len() {
        if bytes[index] != b'{' || bytes[index + 1] != b'$' {
            index += 1;
            continue;
        }

        let escaped = bytes[..index]
            .iter()
            .rev()
            .take_while(|byte| **byte == b'\\')
            .count()
            % 2
            == 1;
        if escaped {
            index += 2;
            continue;
        }

        let name_start = index + 2;
        let mut end = name_start;
        while bytes.get(end).is_some_and(u8::is_ascii_alphanumeric) {
            end += 1;
        }
        if end > name_start && bytes.get(end) == Some(&b'}') {
            return true;
        }
        index += 2;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wikidot_attribute_maps_drop_only_complete_unescaped_variables() {
        let settings = WikitextSettings::from_mode(
            crate::settings::WikitextMode::Page,
            crate::layout::Layout::Wikidot,
        );
        let mut arguments = Arguments::new();
        arguments.insert("title", cow!("{$x}"));
        arguments.insert("class", cow!("prefix {$X1} suffix"));
        arguments.insert("data-malformed", cow!("{$x"));
        arguments.insert("data-escaped", cow!(r"\{$x}"));
        arguments.insert("data-dash", cow!("{$foo-bar}"));
        arguments.insert("data-underscore", cow!("{$foo_bar}"));
        arguments.insert("data-double-dollar", cow!("{$$x}"));

        let attributes = arguments.to_attribute_map(&settings);
        assert!(!attributes.get().contains_key("title"));
        assert!(!attributes.get().contains_key("class"));
        assert_eq!(
            attributes.get().get("data-malformed").map(Cow::as_ref),
            Some("{$x"),
        );
        assert_eq!(
            attributes.get().get("data-escaped").map(Cow::as_ref),
            Some(r"\{$x}"),
        );
        assert_eq!(
            attributes.get().get("data-dash").map(Cow::as_ref),
            Some("{$foo-bar}"),
        );
        assert_eq!(
            attributes.get().get("data-underscore").map(Cow::as_ref),
            Some("{$foo_bar}"),
        );
        assert_eq!(
            attributes.get().get("data-double-dollar").map(Cow::as_ref),
            Some("{$$x}"),
        );
    }
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn get_bool_rejects_malformed_boolean_argument() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[collapsible folded=\"maybe\"]]");
        let parser = Parser::new(&tokenization, &page_info, &settings);
        let mut arguments = Arguments::new();
        arguments.insert("folded", cow!("maybe"));

        let error = arguments
            .get_bool(&parser, "folded")
            .expect_err("malformed boolean should fail");
        assert_eq!(error.kind(), ParseErrorKind::BlockMalformedArguments);
    }

    #[test]
    fn get_value_rejects_malformed_parseable_argument() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[module PageTree depth=\"many\"]]");
        let parser = Parser::new(&tokenization, &page_info, &settings);
        let mut arguments = Arguments::new();
        arguments.insert("depth", cow!("many"));
        arguments.insert("root", cow!("start"));

        let snapshot = arguments.to_hash_map();
        assert_eq!(
            snapshot.get("root").map(|value| value.as_ref()),
            Some("start")
        );

        let error = arguments
            .get_value::<u32>(&parser, "depth")
            .expect_err("malformed integer should fail");
        assert_eq!(error.kind(), ParseErrorKind::BlockMalformedArguments);
    }
}
