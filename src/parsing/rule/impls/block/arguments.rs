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
use std::collections::HashMap;
use std::str::FromStr;
use unicase::UniCase;

macro_rules! make_err {
    ($parser:expr) => {
        $parser.make_err(ParseErrorKind::BlockMalformedArguments)
    };
}

#[derive(Debug, Clone, Default)]
pub struct Arguments<'t> {
    inner: HashMap<UniCase<&'t str>, Cow<'t, str>>,
    raw: Vec<RawModuleArgument<'t>>,
}

impl<'t> Arguments<'t> {
    #[inline]
    pub fn new() -> Self {
        Arguments::default()
    }

    /// Inserts a key / value pair into the list of arguments.
    pub fn insert(&mut self, key: &'t str, value: Cow<'t, str>) {
        let key = UniCase::ascii(key);
        self.raw.push(RawModuleArgument {
            name: cow!(key.into_inner()),
            value: value.clone(),
        });
        self.inner.insert(key, value);
    }

    /// Gets **and removes** a string value from the arguments from its key.
    #[must_use = "non-idempotent getter method"]
    pub fn get(&mut self, key: &'t str) -> Option<Cow<'t, str>> {
        let key = UniCase::ascii(key);
        self.inner.remove(&key)
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

    /// Removes the `UniCase` wrappers to produce a separate hash map of keys to values.
    ///
    /// This returns a new `HashMap` suitable for inclusion in final `Element`s.
    /// It does not clone any string allocations, as they are all borrowed
    /// (or already owned, per `Cow`).
    /// It only makes a new allocation for the new `HashMap`.
    pub fn to_hash_map(&self) -> HashMap<Cow<'t, str>, Cow<'t, str>> {
        self.inner
            .iter()
            .map(|(key, value)| {
                let key = cow!(key.into_inner());
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
        let mut map = AttributeMap::from_arguments(&self.inner);
        map.isolate_id(settings);
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
