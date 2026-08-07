/*
 * wikidot_code.rs
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

use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WikidotCodeCandidate {
    pub(crate) body: Range<usize>,
    pub(crate) owner_end: usize,
    pub(crate) end_blocks_to_skip: usize,
}

pub(crate) fn candidate_at(
    source: &str,
    opener_start: usize,
) -> Option<WikidotCodeCandidate> {
    candidate_at_inner(source, opener_start, true)
}

pub(crate) fn candidate_at_owned_opener(
    source: &str,
    opener_start: usize,
) -> Option<WikidotCodeCandidate> {
    candidate_at_inner(source, opener_start, false)
}

fn candidate_at_inner(
    source: &str,
    opener_start: usize,
    require_line_owner: bool,
) -> Option<WikidotCodeCandidate> {
    let body_start = code_opener_end(source, opener_start, require_line_owner)?;
    let mut cursor = body_start;
    let mut nested_depth = 0usize;
    let mut end_blocks_to_skip = 0usize;

    while let Some(relative_marker) = source[cursor..].find("[[") {
        let marker_start = cursor + relative_marker;
        if let Some(marker_end) = code_closer_end(source, marker_start) {
            if nested_depth > 0 {
                nested_depth -= 1;
                end_blocks_to_skip += 1;
                cursor = marker_end;
                continue;
            }

            let body = body_start..marker_start;
            if crosses_bold_closer(&source[body.clone()], &source[marker_end..]) {
                return None;
            }
            return Some(WikidotCodeCandidate {
                body,
                owner_end: marker_end,
                end_blocks_to_skip,
            });
        }

        if code_opener_end(source, marker_start, true).is_some() {
            nested_depth += 1;
        }
        cursor = marker_start + 2;
    }

    None
}

pub(crate) fn active_ranges(source: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut cursor = 0usize;

    while let Some(relative_marker) = source[cursor..].find("[[") {
        let opener_start = cursor + relative_marker;
        if let Some(candidate) = candidate_at(source, opener_start) {
            ranges.push(opener_start..candidate.owner_end);
            cursor = candidate.owner_end;
        } else {
            cursor = opener_start + 2;
        }
    }

    ranges
}

pub(crate) fn active_body_ranges(source: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut cursor = 0usize;

    while let Some(relative_marker) = source[cursor..].find("[[") {
        let opener_start = cursor + relative_marker;
        if let Some(candidate) = candidate_at(source, opener_start) {
            ranges.push(candidate.body);
            cursor = candidate.owner_end;
        } else {
            cursor = opener_start + 2;
        }
    }

    ranges
}

fn code_opener_end(
    source: &str,
    opener_start: usize,
    require_line_owner: bool,
) -> Option<usize> {
    let suffix = source.get(opener_start..)?;
    let head_start = opener_start + "[[".len();
    if !suffix.starts_with("[[")
        || (require_line_owner && !has_physical_line_ownership(source, opener_start))
    {
        return None;
    }

    let relative_end = source[head_start..].find("]]")?;
    let head_end = head_start + relative_end;
    let head = &source[head_start..head_end];
    let name_end = head
        .find(|character: char| character.is_ascii_whitespace())
        .unwrap_or(head.len());
    let name = &head[..name_end];

    name.eq_ignore_ascii_case("code").then_some(head_end + 2)
}

fn code_closer_end(source: &str, closer_start: usize) -> Option<usize> {
    let suffix = source.get(closer_start..)?;
    if !suffix.starts_with("[[/") {
        return None;
    }
    let head_start = closer_start + "[[/".len();
    let relative_end = source[head_start..].find("]]")?;
    let head_end = head_start + relative_end;
    let name = source[head_start..head_end]
        .trim_matches(|character: char| character.is_ascii_whitespace());

    name.eq_ignore_ascii_case("code").then_some(head_end + 2)
}

fn has_physical_line_ownership(source: &str, opener_start: usize) -> bool {
    let line_start = source[..opener_start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    source[line_start..opener_start]
        .bytes()
        .all(|byte| matches!(byte, b' ' | b'\t' | b'\0'))
}

fn crosses_bold_closer(body: &str, suffix: &str) -> bool {
    let Some((closing, name, _)) = marker_at(suffix, 0) else {
        return false;
    };
    if !closing || !name.eq_ignore_ascii_case("bold") {
        return false;
    }

    let mut depth = 0usize;
    let mut cursor = 0usize;
    while let Some(relative_marker) = body[cursor..].find("[[") {
        let marker_start = cursor + relative_marker;
        let Some((closing, name, marker_end)) = marker_at(body, marker_start) else {
            cursor = marker_start + 2;
            continue;
        };
        cursor = marker_end;
        if !name.eq_ignore_ascii_case("bold") {
            continue;
        }
        if closing {
            depth = depth.saturating_sub(1);
        } else {
            depth += 1;
        }
    }
    depth > 0
}

fn marker_at(source: &str, marker_start: usize) -> Option<(bool, &str, usize)> {
    let suffix = source.get(marker_start..)?;
    if !suffix.starts_with("[[") {
        return None;
    }
    let closing = suffix.starts_with("[[/");
    let head_start = marker_start + if closing { "[[/".len() } else { "[[".len() };
    let relative_end = source[head_start..].find("]]")?;
    let head_end = head_start + relative_end;
    let head = source[head_start..head_end]
        .trim_matches(|character: char| character.is_ascii_whitespace());
    let name_end = head
        .find(|character: char| character.is_ascii_whitespace())
        .unwrap_or(head.len());
    let name = &head[..name_end];
    (!name.is_empty()).then_some((closing, name, head_end + 2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_tracks_nested_closer_ownership() {
        let source = "[[code]]\n[[code]]\nnested\n[[/code]]\n[[/code]]";
        let candidate = candidate_at(source, 0).expect("outer code candidate");

        assert_eq!(candidate.body, 8..35);
        assert_eq!(candidate.owner_end, source.len());
        assert_eq!(candidate.end_blocks_to_skip, 1);
    }

    #[test]
    fn candidate_rejects_unclosed_prefixed_and_crossed_blocks() {
        for source in [
            "[[code]]\nunclosed",
            "prefix[[code]]\nbody\n[[/code]]",
            "[[code]]\n[[code]]\nnested\n[[/code]]",
            "[[code]]\nouter [[bold]]inner\n[[/code]][[/bold]]",
        ] {
            let opener = source.find("[[code]]").unwrap();
            assert!(candidate_at(source, opener).is_none(), "{source:?}");
        }
    }
}
