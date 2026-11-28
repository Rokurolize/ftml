/*
 * wikidot_raw_escape.rs
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

//! Wikidot raw inline escape placeholders.
//!
//! Live Wikidot escapes every complete single-line raw inline construct
//! (`@@...@@` or `@<...>@`) before block parsing and replaces it with a
//! placeholder token. The token is the byte `0xFF`, the 1-based ordinal of the
//! construct in source order as decimal digits, and `0xFF` again. Block heads
//! and attribute values observe the placeholders, which is why e.g. a date
//! `format` attribute containing raw markup serializes as `format_%FF1%FF`.
//!
//! Rust strings cannot carry `0xFF` (it is not valid UTF-8), so placeholders
//! are represented with private-use sentinels here and rendered to the live
//! byte spelling only where live exposes it.

/// Opening sentinel of an escaped raw placeholder (`0xFF` in live).
pub(crate) const PLACEHOLDER_OPEN: char = '\u{E000}';

/// Closing sentinel of an escaped raw placeholder (`0xFF` in live).
pub(crate) const PLACEHOLDER_CLOSE: char = '\u{E001}';

/// A complete raw inline construct as live's pre-parse escape pass finds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RawEscape {
    pub start: usize,
    pub end: usize,
}

/// Scan `source` for the complete `@<...>@` raw constructs live escapes.
fn scan_raw_html_escapes(source: &str) -> Vec<RawEscape> {
    let bytes = source.as_bytes();
    let mut escapes = Vec::new();
    let mut index = 0;
    let mut unclosed_line_end = None;
    while index + 1 < bytes.len() {
        if bytes[index] == b'@' && bytes[index + 1] == b'<' {
            if unclosed_line_end.is_some_and(|end| index < end) {
                index += 1;
                continue;
            }

            let mut probe = index + 2;
            let mut close = None;
            while probe + 1 < bytes.len() {
                if matches!(bytes[probe], b'\n' | b'\r') {
                    break;
                }
                if bytes[probe] == b'>' && bytes[probe + 1] == b'@' {
                    close = Some(probe);
                    break;
                }
                probe += 1;
            }

            if let Some(close) = close {
                let end = close + 2;
                escapes.push(RawEscape { start: index, end });
                index = end;
                continue;
            }
            unclosed_line_end = Some(probe);
        }
        index += 1;
    }
    escapes
}

/// Scan `source` for the complete `@@...@@` raw constructs live escapes.
///
/// Runs of adjacent closing/opening pairs (`@@x@@@@y@@`) count as one escaped
/// construct: live does not start a new construct at the junction of two
/// doubled-at runs.
fn scan_raw_at_escapes(source: &str, html: &[RawEscape]) -> Vec<RawEscape> {
    let bytes = source.as_bytes();
    let mut escapes = Vec::new();
    let mut index = 0;
    // The html ranges are sorted and disjoint, and both scans only move
    // forward, so a cursor answers "is this position inside an html range"
    // without walking the whole list at every byte.
    let mut html_at = 0;

    while index + 1 < bytes.len() {
        while html_at < html.len() && html[html_at].end <= index {
            html_at += 1;
        }

        if let Some(escape) = html.get(html_at)
            && index >= escape.start
            && index < escape.end
        {
            index = escape.end;
            html_at += 1;
            continue;
        }
        if bytes[index] == b'@' && bytes[index + 1] == b'@' {
            let mut cursor = index + 2;
            let mut cursor_html = html_at;
            let mut closed = None;
            while cursor + 1 < bytes.len() {
                while cursor_html < html.len() && html[cursor_html].end <= cursor {
                    cursor_html += 1;
                }

                if let Some(escape) = html.get(cursor_html)
                    && cursor >= escape.start
                    && cursor < escape.end
                {
                    break;
                }
                if matches!(bytes[cursor], b'\n' | b'\r') {
                    break;
                }
                if bytes[cursor] == b'@' && bytes[cursor + 1] == b'@' {
                    closed = Some(cursor);
                    break;
                }
                cursor += 1;
            }
            if let Some(close) = closed {
                escapes.push(RawEscape {
                    start: index,
                    end: close + 2,
                });
                index = close + 2;
                while index + 1 < bytes.len()
                    && bytes[index] == b'@'
                    && bytes[index + 1] == b'@'
                {
                    index += 2;
                }
                continue;
            }
        }
        index += 1;
    }
    escapes
}

/// All raw inline constructs live escapes, in live's numbering order: the
/// `@<...>@` pass runs first, then the `@@...@@` pass.
pub(crate) fn raw_escapes(source: &str) -> Vec<RawEscape> {
    let html = scan_raw_html_escapes(source);
    let mut escapes = html.clone();
    escapes.extend(scan_raw_at_escapes(source, &html));
    escapes
}

/// A raw escape paired with the placeholder ordinal live assigns it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NumberedEscape {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) ordinal: usize,
}

pub(crate) fn rawold_escapes(source: &str) -> Vec<RawEscape> {
    let html = scan_raw_html_escapes(source);
    let at = scan_raw_at_escapes(source, &html);
    scan_rawold_escapes(source, &html, &at)
}

/// Placeholder ordinals for attribute-value encoding.
///
/// Live numbers `@<...>@` escapes first, then reserves one number for each
/// recognized code block (whose bodies are opaque), then numbers `@@...@@`
/// escapes in document order, including those inside comments, and finally
/// numbers `` ``...`` `` escapes. Reserved code-block numbers emit no
/// placeholder themselves.
pub(crate) fn numbered_escapes(source: &str) -> Vec<NumberedEscape> {
    let html = scan_raw_html_escapes(source);
    let at = scan_raw_at_escapes(source, &html);
    let rawold = scan_rawold_escapes(source, &html, &at);
    let code_ranges = crate::wikidot_code::active_ranges(source);

    let mut ordinal = 0;
    let mut numbered = Vec::new();

    for escape in &html {
        ordinal += 1;
        numbered.push(NumberedEscape {
            start: escape.start,
            end: escape.end,
            ordinal,
        });
    }

    ordinal += code_ranges.len();

    for escape in at.iter().chain(rawold.iter()) {
        if code_ranges
            .iter()
            .any(|range| escape.start >= range.start && escape.end <= range.end)
        {
            continue;
        }
        ordinal += 1;
        numbered.push(NumberedEscape {
            start: escape.start,
            end: escape.end,
            ordinal,
        });
    }

    numbered
}

fn scan_rawold_escapes(
    source: &str,
    html: &[RawEscape],
    at: &[RawEscape],
) -> Vec<RawEscape> {
    let bytes = source.as_bytes();
    let mut escapes = Vec::new();
    let mut index = 0;

    while index + 1 < bytes.len() {
        if bytes[index] != b'`'
            || bytes[index + 1] != b'`'
            || (index > 0 && bytes[index - 1] == b'`')
        {
            index += 1;
            continue;
        }
        let body_start = index + 2;
        let mut cursor = body_start;
        let mut body_end = None;
        while let Some(offset) = source.get(cursor..).and_then(|rest| rest.find("``")) {
            let candidate_end = cursor + offset;
            let body = &source[body_start..candidate_end];
            if !body.is_empty()
                && !body
                    .chars()
                    .any(|c| matches!(c, '\'' | '\u{2018}' | '\u{2019}'))
                && !body.ends_with('`')
            {
                body_end = Some(candidate_end);
                break;
            }
            cursor = candidate_end + 1;
        }
        let Some(body_end) = body_end else {
            index += 1;
            continue;
        };
        let end = body_end + 2;
        if html
            .iter()
            .chain(at.iter())
            .any(|range| index >= range.start && index < range.end)
        {
            index = end;
            continue;
        }
        escapes.push(RawEscape { start: index, end });
        index = end;
    }

    escapes
}

/// Append `text` to `output` using live's date-format percent encoding,
/// spelling escaped placeholders as their `%FF<ordinal>%FF` byte run.
pub(crate) fn push_percent_encoded_placeholders(output: &mut String, text: &str) {
    fn push_plain(output: &mut String, character: char) {
        for byte in character.to_string().bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    output.push(char::from(byte));
                }
                _ => {
                    output.push('%');
                    let written = format!("{byte:02X}");
                    output.push_str(&written);
                }
            }
        }
    }

    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character != PLACEHOLDER_OPEN {
            push_plain(output, character);
            continue;
        }
        let mut digits = String::new();
        while let Some(&next) = chars.peek() {
            if next == PLACEHOLDER_CLOSE {
                chars.next();
                break;
            }
            if next.is_ascii_digit() {
                digits.push(next);
                chars.next();
                continue;
            }
            digits.clear();
            break;
        }
        if digits.is_empty() {
            push_plain(output, character);
            continue;
        }
        output.push_str("%FF");
        output.push_str(&digits);
        output.push_str("%FF");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_complete_single_line_raw_constructs() {
        assert_eq!(raw_escapes("@@a@@"), vec![RawEscape { start: 0, end: 5 }]);
        assert_eq!(raw_escapes("@<b>@"), vec![RawEscape { start: 0, end: 5 }],);
        assert!(raw_escapes("@@a").is_empty());
        assert!(raw_escapes("@@a\nb@@").is_empty());
        assert_eq!(
            raw_escapes("x@@a@@ @@b@@"),
            vec![
                RawEscape { start: 1, end: 6 },
                RawEscape { start: 7, end: 12 },
            ],
        );
    }

    #[test]
    fn html_and_at_escapes_do_not_overlap() {
        // The cursor-based scan has to agree with a scan that tests every
        // html range at every byte.
        fn reference(source: &str) -> Vec<RawEscape> {
            let bytes = source.as_bytes();
            let html = scan_raw_html_escapes(source);
            let mut escapes = Vec::new();
            let mut index = 0;

            while index + 1 < bytes.len() {
                if let Some(range) = html
                    .iter()
                    .find(|escape| index >= escape.start && index < escape.end)
                {
                    index = range.end;
                    continue;
                }
                if bytes[index] == b'@' && bytes[index + 1] == b'@' {
                    let mut cursor = index + 2;
                    let mut closed = None;

                    while cursor + 1 < bytes.len() {
                        if html
                            .iter()
                            .any(|escape| cursor >= escape.start && cursor < escape.end)
                        {
                            break;
                        }
                        if matches!(bytes[cursor], b'\n' | b'\r') {
                            break;
                        }
                        if bytes[cursor] == b'@' && bytes[cursor + 1] == b'@' {
                            closed = Some(cursor);
                            break;
                        }
                        cursor += 1;
                    }

                    if let Some(close) = closed {
                        escapes.push(RawEscape {
                            start: index,
                            end: close + 2,
                        });
                        index = close + 2;
                        while index + 1 < bytes.len()
                            && bytes[index] == b'@'
                            && bytes[index + 1] == b'@'
                        {
                            index += 2;
                        }
                        continue;
                    }
                }
                index += 1;
            }

            escapes
        }

        for source in [
            "@<x>@ @@a@@",
            "@@a@@@<x>@",
            "@@a@<x>@b@@",
            "@@a@<x>@",
            "@@a@@@<x>@@@b@@",
            "no escapes at all",
            "@@",
            "@",
        ] {
            let html = scan_raw_html_escapes(source);
            assert_eq!(
                scan_raw_at_escapes(source, &html),
                reference(source),
                "at-escape scan differs for {source:?}",
            );
        }
    }

    #[test]
    fn percent_encodes_placeholders_like_live() {
        let mut output = String::new();
        push_percent_encoded_placeholders(
            &mut output,
            &format!("x{PLACEHOLDER_OPEN}1{PLACEHOLDER_CLOSE}y"),
        );
        assert_eq!(output, "x%FF1%FFy");
    }

    fn ordinal_after(source: &str, needle: &str) -> usize {
        let start = source.find(needle).unwrap();
        numbered_escapes(source)
            .into_iter()
            .find(|escape| escape.start == start)
            .unwrap()
            .ordinal
    }

    #[test]
    fn code_blocks_reserve_one_number_and_hide_their_bodies() {
        let anchor = "[[a href=\"@@x@@\"]]y[[/a]]";
        assert_eq!(ordinal_after(anchor, "@@x@@"), 1);
        assert_eq!(
            ordinal_after(&format!("[[code]]plain[[/code]]\n{anchor}"), "@@x@@"),
            2
        );
        assert_eq!(
            ordinal_after(&format!("{anchor}\n[[code]]plain[[/code]]"), "@@x@@"),
            2
        );
        assert_eq!(
            ordinal_after(
                &format!("[[code]]a[[/code]]\n[[code]]b[[/code]]\n{anchor}"),
                "@@x@@"
            ),
            3
        );
        let opaque = format!("[[code]]@@q@@ @@r@@[[/code]]\n{anchor}");
        assert_eq!(ordinal_after(&opaque, "@@x@@"), 2);
        assert!(
            numbered_escapes(&opaque)
                .iter()
                .all(|escape| escape.start != opaque.find("@@q@@").unwrap())
        );
    }

    #[test]
    fn comment_tokens_count_in_document_order() {
        let anchor = "[[a href=\"@@x@@\"]]y[[/a]]";
        assert_eq!(
            ordinal_after(&format!("[!-- @@q@@ --]\n{anchor}"), "@@x@@"),
            2
        );
        assert_eq!(
            ordinal_after(&format!("{anchor}\n[!-- @@q@@ --]"), "@@x@@"),
            1
        );
        assert_eq!(
            ordinal_after(&format!("[!-- plain --]\n{anchor}"), "@@x@@"),
            1
        );
    }

    #[test]
    fn rawold_tokens_are_numbered_after_at_escapes() {
        let source = "[[a href=\"@@x@@ ``y``\"]]z[[/a]]";
        assert_eq!(ordinal_after(source, "@@x@@"), 1);
        assert_eq!(ordinal_after(source, "``y``"), 2);
        assert_eq!(ordinal_after("``y``\n@@x@@", "@@x@@"), 1);
    }
}
