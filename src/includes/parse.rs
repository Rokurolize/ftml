/*
 * includes/parse.rs
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

//! This module provides functions to parse strings into [`IncludeRef`]s

mod parser {
    // Since pest generates some code that clippy doesn't like
    #![allow(clippy::empty_docs)]

    #[derive(Parser, Debug)]
    #[grammar = "includes/grammar.pest"]
    pub struct IncludeParser;
}

use self::parser::*;
use super::{IncludeRef, VARIABLE_REGEX};
use crate::data::{PageRef, PageRefParseError};
use crate::tree::VariableMap;
use pest::Parser;
use pest::iterators::Pairs;
use std::collections::HashMap;

#[derive(Debug)]
struct RawIncludeArgument<'t> {
    key: &'t str,
    value: &'t str,
    spaced_empty_value: bool,
}

/// Parses a single include block in the text.
///
/// # Arguments
/// The "start" argument is the index at which the include block starts.
/// It does not necessarily relate to the index of the include within the text str.
///
/// # Return values
/// Returns a tuple of an [`IncludeRef`] that represents the included text and a usize that
/// represents the end index of the include block, such that start..end covers the full include
/// block (before the include goes through).
pub fn parse_include_block<'t>(
    text: &'t str,
    start: usize,
) -> Result<(IncludeRef<'t>, usize), IncludeParseError> {
    match IncludeParser::parse(Rule::include, &text[start..]) {
        Ok(mut pairs) => {
            // Extract inner pairs
            // These actually make up the include block's tokens
            let first = pairs.next().expect("No pairs returned on successful parse");
            let span = first.as_span();

            debug!("Parsed include block");

            // Convert into an IncludeRef
            let include = process_pairs(first.into_inner())?;

            // Adjust offset and return
            Ok((include, start + span.end()))
        }
        Err(error) => {
            warn!("Include block was invalid: {error}");
            Err(IncludeParseError)
        }
    }
}

/// Creates an [`IncludeRef`] out of pest [`Pairs`].
fn process_pairs(mut pairs: Pairs<Rule>) -> Result<IncludeRef, IncludeParseError> {
    let page_raw = pairs.next().ok_or(IncludeParseError)?.as_str();
    let page_ref = validate_include_page_ref(PageRef::parse(page_raw)?)?;

    trace!("Got page for include {page_ref:?}");
    let mut raw_arguments = Vec::new();
    let mut spaced_empty_separator = false;

    for pair in pairs {
        if pair.as_rule() == Rule::spaced_empty_separator {
            spaced_empty_separator = true;
            continue;
        }
        debug_assert_eq!(pair.as_rule(), Rule::argument);

        let argument_source = pair.as_str();
        let (key, value) = {
            let mut argument_pairs = pair.into_inner();

            let key = argument_pairs
                .next()
                .expect("Argument pairs terminated early")
                .as_str();

            let value = argument_pairs
                .next()
                .expect("Argument pairs terminated early")
                .as_str();

            (key, value)
        };

        trace!("Adding argument for include (key '{key}', value '{value}')");

        let spaced_empty_value = value.is_empty()
            && argument_source.split_once('=').is_some_and(|(_, after)| {
                matches!(after.as_bytes().first(), Some(b' ' | b'\t'))
            });

        raw_arguments.push(RawIncludeArgument {
            key,
            value,
            spaced_empty_value,
        });
    }
    let arguments = resolve_include_arguments(&raw_arguments);

    Ok(IncludeRef::new(page_ref, arguments)
        .with_spaced_empty_separator(spaced_empty_separator))
}

fn resolve_include_arguments(
    raw_arguments: &[RawIncludeArgument<'_>],
) -> VariableMap<'static> {
    let round_limit = raw_arguments.len().saturating_mul(2).clamp(1, 128);
    let mut arguments = HashMap::<String, String>::new();

    // Seed the graph with the first literal fallback for each fixed key. This
    // gives dynamic expressions such as `{$mode_{$mode}}` a bounded value from
    // which to resolve, while preserving Wikidot's first-concrete-value rule.
    for argument in raw_arguments {
        if argument.spaced_empty_value
            || !is_static_identifier(argument.key)
            || argument.value.contains("{$")
        {
            continue;
        }
        arguments
            .entry(argument.key.to_owned())
            .or_insert_with(|| argument.value.to_owned());
    }

    for _ in 0..round_limit {
        let mut next = HashMap::new();
        for argument in raw_arguments {
            if argument.spaced_empty_value {
                continue;
            }
            let key = expand_argument_expression(argument.key, &arguments, round_limit);
            if !is_static_identifier(&key) {
                continue;
            }
            let value =
                expand_argument_expression(argument.value, &arguments, round_limit);
            let fallback_reference = value.trim_end_matches([' ', '\t', '\r', '\n']);
            if fallback_reference == format!("{{${key}}}") {
                continue;
            }
            next.entry(key).or_insert(value);
        }
        if next == arguments {
            break;
        }
        arguments = next;
    }

    arguments
        .into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect()
}

fn expand_argument_expression(
    expression: &str,
    arguments: &HashMap<String, String>,
    round_limit: usize,
) -> String {
    let mut output = expression.to_owned();
    for _ in 0..round_limit {
        let expanded = VARIABLE_REGEX
            .replace_all(&output, |capture: &regex::Captures<'_>| {
                arguments
                    .get(&capture["name"])
                    .map(|value| {
                        value.trim_end_matches([' ', '\t', '\r', '\n']).to_owned()
                    })
                    .unwrap_or_else(|| capture[0].to_owned())
            })
            .into_owned();
        if expanded == output {
            break;
        }
        output = expanded;
    }
    output
}

fn is_static_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
        })
}

fn validate_include_page_ref(page_ref: PageRef) -> Result<PageRef, IncludeParseError> {
    if page_ref.page().is_empty() {
        return Err(IncludeParseError);
    }

    Ok(page_ref)
}

#[derive(Debug, PartialEq, Eq)]
pub struct IncludeParseError;

impl From<PageRefParseError> for IncludeParseError {
    #[inline]
    fn from(_: PageRefParseError) -> Self {
        std::convert::identity(IncludeParseError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;

    #[test]
    fn include_parse_error_converts_from_page_ref_parse_error() {
        let error = IncludeParseError::from(PageRefParseError);
        assert_eq!(error, IncludeParseError);
    }

    #[test]
    fn validate_include_page_ref_rejects_empty_page() {
        let empty = PageRef {
            site: Some(str!("scp-wiki")),
            page: String::new(),
            extra: None,
        };

        assert_eq!(
            validate_include_page_ref(empty),
            Err(IncludeParseError),
            "empty include page references should reject",
        );
    }

    #[test]
    fn validate_include_page_ref_accepts_non_empty_page() {
        let page_ref = PageRef::page_only("component:ok");

        assert_eq!(
            validate_include_page_ref(page_ref.clone()),
            Ok(page_ref),
            "non-empty include page references should pass through",
        );
    }

    #[test]
    fn parse_include_block_rejects_empty_page_reference() {
        let error = parse_include_block("[[include :scp-wiki:/]]", 0)
            .expect_err("empty include page should reject");

        assert_eq!(error, IncludeParseError);
    }

    #[test]
    fn parse_include_block_ignores_commented_arguments() {
        let source = concat!(
            "[[include :scp-wiki:component:mapping-source\n",
            "facility=-- |\n",
            "x=66 |\n",
            "[!--textclass=subtitlemap|\n",
            "text=G-GOC|--]\n",
            "]]\n",
        );

        let (include, end) = parse_include_block(source, 0)
            .expect("comments may disable arguments inside an include");

        assert_eq!(end, source.len() - 1);
        assert_eq!(
            include.page_ref(),
            &PageRef::page_and_site("scp-wiki", "component:mapping-source")
        );
        assert_eq!(
            include.variables().get("facility").map(Cow::as_ref),
            Some("-- ")
        );
        assert_eq!(include.variables().get("x").map(Cow::as_ref), Some("66 "));
        assert!(!include.variables().contains_key("textclass"));
        assert!(!include.variables().contains_key("text"));
    }

    #[test]
    fn parse_include_block_records_only_the_live_spaced_empty_separator_shape() {
        for source in ["[[include page | ]]", "[[include PAGE |\t]]"] {
            let (include, end) =
                parse_include_block(source, 0).expect("include should parse");
            assert_eq!(end, source.len(), "{source:?}");
            assert!(include.has_spaced_empty_separator(), "{source:?}");
            assert!(include.variables().is_empty(), "{source:?}");
        }

        for source in [
            "[[include page]]",
            "[[include page |]]",
            "[[include page ||]]",
            "[[include page || ]]",
            "[[include page | a=1]]",
            "[[include page a=1 | ]]",
        ] {
            let (include, _) = parse_include_block(source, 0)
                .unwrap_or_else(|_| panic!("include should parse: {source:?}"));
            assert!(!include.has_spaced_empty_separator(), "{source:?}");
        }
    }
}
