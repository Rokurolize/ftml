/*
 * preproc/modulepre.rs
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

const OPENER: &[u8] = b"[[module654";
const CLOSER: &[u8] = b"[[/module]]";

fn find_from(bytes: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    if start > bytes.len() || needle.is_empty() {
        return None;
    }
    bytes[start..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| start + offset)
}

fn find_ignore_case_from(bytes: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    if start > bytes.len() || needle.is_empty() {
        return None;
    }
    bytes[start..]
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
        .map(|offset| start + offset)
}

fn match_end(source: &[u8], start: usize) -> Option<usize> {
    let mut cursor = start + OPENER.len();
    let separator_start = cursor;
    while matches!(source.get(cursor), Some(b' ' | b'\t')) {
        cursor += 1;
    }
    if cursor == separator_start {
        return None;
    }

    let name_start = cursor;
    while source.get(cursor).is_some_and(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'/')
    }) {
        cursor += 1;
    }
    if cursor == name_start {
        return None;
    }

    let head_close = find_from(source, cursor, b"]]")?;
    if head_close > cursor && !source[cursor].is_ascii_whitespace() {
        return None;
    }

    let Some(body_close) = find_ignore_case_from(source, head_close + 2, CLOSER) else {
        return Some(head_close + 2);
    };
    Some(body_close + CLOSER.len())
}

pub(crate) fn rewrite(text: &mut String) {
    if !text.contains("[[module654") {
        return;
    }

    let source = text.as_bytes();
    let mut output = String::with_capacity(text.len());
    let mut copied = 0usize;
    let mut pos = 0usize;

    while pos + OPENER.len() <= source.len() {
        let at_line_start = pos == 0 || matches!(source[pos - 1], b'\n' | b'\r');
        if at_line_start
            && source[pos..].starts_with(OPENER)
            && let Some(end) = match_end(source, pos)
        {
            output.push_str(&text[copied..pos]);
            output.push_str("[[module");
            copied = pos + OPENER.len();
            pos = end;
            continue;
        }
        pos += 1;
    }

    if copied > 0 {
        output.push_str(&text[copied..]);
        *text = output;
    }
}
