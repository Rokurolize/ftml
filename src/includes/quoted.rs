/*
 * includes/quoted.rs
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

//! Normalization for legacy includes written inside native blockquotes.

use super::IncludeRef;
use super::parse::parse_include_block;
use crate::tree::VariableMap;
use std::borrow::Cow;

#[derive(Debug)]
pub(super) struct ParsedQuotedInclude {
    pub include: IncludeRef<'static>,
    pub end: usize,
}

#[derive(Clone, Copy, Debug)]
struct OffsetSegment {
    normalized_start: usize,
    normalized_end: usize,
    original_start: usize,
}

/// Parse an include whose physical lines carry a native quote prefix.
///
/// The ordinary include grammar intentionally starts at `[[`. We temporarily
/// remove exactly one caller-supplied quote depth from each physical line,
/// parse that normalized block, then own its arguments before the temporary
/// buffer is dropped. Offset segments translate the grammar's end position
/// back to the original source without assuming uniform prefix spacing.
pub(super) fn parse_quoted_include(
    input: &str,
    line_start: usize,
    marker_start: usize,
    quote_depth: usize,
) -> Option<ParsedQuotedInclude> {
    debug_assert!(marker_start > line_start);
    debug_assert!(quote_depth > 0);

    let mut normalized = String::new();
    let mut segments = Vec::new();
    let mut original_line_start = line_start;

    for (line_index, line) in input[line_start..].split_inclusive('\n').enumerate() {
        let content_offset = if line_index == 0 {
            marker_start - line_start
        } else {
            strip_quote_prefix(line, quote_depth)?
        };
        let content = &line[content_offset..];
        let normalized_start = normalized.len();
        normalized.push_str(content);
        segments.push(OffsetSegment {
            normalized_start,
            normalized_end: normalized.len(),
            original_start: original_line_start + content_offset,
        });

        if line_has_include_terminator(content)
            && let Ok((include, normalized_end)) = parse_include_block(&normalized, 0)
        {
            let end = original_offset(&segments, normalized_end)?;
            return Some(ParsedQuotedInclude {
                include: own_include(include),
                end,
            });
        }

        original_line_start += line.len();
    }

    None
}

/// Prefix every physical line produced by a quoted include expansion.
pub(super) fn quote_expansion(content: &str, quote_prefix: &str) -> String {
    let line_count = content.bytes().filter(|&byte| byte == b'\n').count()
        + usize::from(!content.is_empty() && !content.ends_with('\n'));
    let mut output = String::with_capacity(
        content.len() + quote_prefix.len().saturating_mul(line_count),
    );

    for line in content.split_inclusive('\n') {
        output.push_str(quote_prefix);
        output.push_str(line);
    }

    output
}

fn strip_quote_prefix(line: &str, quote_depth: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut offset = skip_horizontal_space(bytes, 0);

    for _ in 0..quote_depth {
        if bytes.get(offset) != Some(&b'>') {
            return None;
        }
        offset += 1;
        offset = skip_horizontal_space(bytes, offset);
    }

    Some(offset)
}

fn skip_horizontal_space(bytes: &[u8], mut offset: usize) -> usize {
    while matches!(bytes.get(offset), Some(b' ' | b'\t')) {
        offset += 1;
    }
    offset
}

fn line_has_include_terminator(line: &str) -> bool {
    let line = line.strip_suffix('\n').unwrap_or(line);
    let line = line.strip_suffix('\r').unwrap_or(line);
    line.ends_with("]]")
}

fn original_offset(segments: &[OffsetSegment], normalized: usize) -> Option<usize> {
    segments.iter().find_map(|segment| {
        (normalized > segment.normalized_start && normalized <= segment.normalized_end)
            .then_some(segment.original_start + normalized - segment.normalized_start)
    })
}

fn own_include(include: IncludeRef<'_>) -> IncludeRef<'static> {
    let (page_ref, variables) = include.into();
    let variables: VariableMap<'static> = variables
        .into_iter()
        .map(|(key, value)| {
            (Cow::Owned(key.into_owned()), Cow::Owned(value.into_owned()))
        })
        .collect();

    IncludeRef::new(page_ref, variables)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoted_parser_normalizes_spacing_and_maps_original_end() {
        let source = concat!(
            "before\n",
            "> [[include :scp-wiki:component:author-label-source start=—\n",
            ">|name=toadking07]]\n",
            "after\n",
        );
        let line_start = source.find("> [[include").unwrap();
        let marker_start = source[line_start..].find("[[").unwrap() + line_start;

        let parsed = parse_quoted_include(source, line_start, marker_start, 1)
            .expect("quoted include should parse");

        assert_eq!(
            parsed.include.page_ref(),
            &crate::data::PageRef::page_and_site(
                "scp-wiki",
                "component:author-label-source",
            ),
        );
        assert_eq!(
            parsed.include.variables().get("name").map(Cow::as_ref),
            Some("toadking07"),
        );
        assert_eq!(&source[parsed.end..], "\nafter\n");
    }

    #[test]
    fn quoted_parser_requires_every_continuation_line_to_remain_quoted() {
        let source = "> [[include component:box\n|name=unquoted]]\n";
        let marker_start = source.find("[[").unwrap();

        assert!(parse_quoted_include(source, 0, marker_start, 1).is_none());
    }

    #[test]
    fn quote_expansion_preserves_empty_crlf_and_unterminated_lines() {
        assert_eq!(quote_expansion("", "> "), "");
        assert_eq!(
            quote_expansion("first\r\nsecond", "> "),
            "> first\r\n> second"
        );
        assert_eq!(quote_expansion("first\n", ">>"), ">>first\n");
    }
}
