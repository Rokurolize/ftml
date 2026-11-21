/*
 * includes/mod.rs
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

#[warn(missing_docs)]
#[cfg(test)]
mod test;

mod include_ref;
mod includer;
mod parse;

pub use self::include_ref::{IncludeArguments, IncludeRef};
pub use self::includer::{DebugIncluder, FetchedPage, Includer, NullIncluder};

use self::parse::parse_include_block;
use crate::data::PageRef;
use crate::settings::WikitextSettings;
use regex::{Regex, RegexBuilder};
use std::sync::LazyLock;

static INCLUDE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"^[ \t]*\[\[include\s+")
        .case_insensitive(true)
        .multi_line(true)
        .dot_matches_new_line(true)
        .build()
        .unwrap()
});

/// Replaces the include blocks in a string with the content of the pages referenced by those
/// blocks.
#[track_caller]
pub fn include<'t, I, E, F>(
    input: &'t str,
    settings: &WikitextSettings,
    mut includer: I,
    invalid_return: F,
) -> Result<(String, Vec<PageRef>), E>
where
    I: Includer<'t, Error = E>,
    F: FnOnce() -> E,
{
    #[cfg(feature = "test-source-recorder")]
    crate::source_recorder::record(
        "include-input",
        input,
        std::panic::Location::caller(),
    );

    if !settings.enable_page_syntax {
        debug!("Includes are disabled for this input, skipping");

        let output = str!(input);
        let pages = vec![];
        return Ok((output, pages));
    }

    let input_len = input.len();
    debug!("Inserting text for all include blocks in text ({input_len} bytes)");

    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    let mut includes = Vec::new();

    let mut search_start = 0;

    while let Some(mtch) = INCLUDE_REGEX.find_at(input, search_start) {
        let start = mtch.start();

        let slice = mtch.as_str();
        trace!("Found include regex match (start {start}, slice '{slice}')");

        if find_include_end(input, mtch.end(), input.len()).is_none() {
            warn!("Unable to find include terminator, skipping remaining input");
            search_start = input.len();
            continue;
        }

        let parsed = parse_include_block(input, start).ok();

        match parsed {
            Some((include, end)) => {
                ranges.push(start..end);
                includes.push(include);
                search_start = end;
            }
            None => {
                search_start = next_line_start(input, start);
                warn!("Unable to parse include regex match, resuming at {search_start}");
            }
        }
    }

    let fetched_pages = includer.include_pages(&includes)?;

    if includes.len() != fetched_pages.len() {
        return Err(invalid_return());
    }

    let mut replacements: Vec<_> = ranges
        .into_iter()
        .zip(includes.into_iter().zip(fetched_pages))
        .collect();
    replacements.sort_unstable_by_key(|(range, _)| range.start);

    substitute_replacements(input, replacements, &mut includer, invalid_return)
}

type IncludeReplacement<'t> = (std::ops::Range<usize>, (IncludeRef<'t>, FetchedPage<'t>));

fn substitute_replacements<'t, I, E, F>(
    input: &str,
    replacements: Vec<IncludeReplacement<'t>>,
    includer: &mut I,
    invalid_return: F,
) -> Result<(String, Vec<PageRef>), E>
where
    I: Includer<'t, Error = E>,
    F: FnOnce() -> E,
{
    let mut output = String::from(input);
    let mut pages = Vec::new();

    for (range, (include, fetched)) in replacements.into_iter().rev() {
        let (page_ref, arguments): (PageRef, IncludeArguments<'t>) = include.into();

        let range_start = range.start;
        let range_end = range.end;
        debug!("Replacing range for included page ({range_start}..{range_end})");

        if page_ref != fetched.page_ref {
            return Err(invalid_return());
        }

        let replace_with = match fetched.content {
            Some(mut content) => {
                replace_variables(content.to_mut(), &arguments);
                content
            }

            None => includer.no_such_include(&page_ref)?,
        };
        pages.push(page_ref);

        if replace_with.is_empty() {
            output.replace_range(range, "");
        } else {
            let before = (range.start > 0).then(|| {
                if input[..range.start].ends_with("\r\n") {
                    "\r\n"
                } else {
                    "\n"
                }
            });
            let after = (range.end < input.len()).then(|| {
                if input[range.end..].starts_with("\r\n") {
                    "\r\n"
                } else {
                    "\n"
                }
            });
            let mut separated = String::with_capacity(
                replace_with.len()
                    + before.map_or(0, str::len)
                    + after.map_or(0, str::len),
            );
            if let Some(line_ending) = before {
                separated.push_str(line_ending);
            }
            separated.push_str(&replace_with);
            if let Some(line_ending) = after {
                separated.push_str(line_ending);
            }

            output.replace_range(range, &separated);
        }
    }

    pages.reverse();

    Ok((output, pages))
}

fn next_line_start(input: &str, line_start: usize) -> usize {
    match input[line_start..].find('\n') {
        Some(index) => line_start + index + 1,
        None => input.len(),
    }
}

fn find_include_end(input: &str, start: usize, end_bound: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut index = start;

    while index + 2 <= end_bound {
        let end = index + 2;
        if bytes[index] == b']'
            && bytes[index + 1] == b']'
            && (end == input.len()
                || bytes.get(end..).is_some_and(|rest| {
                    rest.starts_with(b"\n") || rest.starts_with(b"\r\n")
                }))
        {
            return Some(end);
        }

        index += 1;
    }

    None
}

fn replace_variables(content: &mut String, arguments: &IncludeArguments<'_>) {
    for (key, value) in arguments {
        let placeholder = format!("{{${key}}}");
        if content.contains(&placeholder) {
            *content = content.replace(&placeholder, value);
        }
    }
}
