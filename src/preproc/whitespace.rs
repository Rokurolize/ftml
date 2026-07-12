/*
 * preproc/whitespace.rs
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

//! This performs the various miscellaneous substitutions that Wikidot does
//! in preparation for its parsing and handling processes. These are:
//! * Replacing DOS and legacy Mac newlines
//! * Trimming whitespace lines
//! * Concatenating lines that end with backslashes
//! * Convert tabs to four spaces
//! * Convert null characters to regular spaces
//! * Compress groups of 3+ newlines into 2 newlines

use super::Replacer;
use regex::{Regex, RegexBuilder};
use std::sync::LazyLock;

static LEADING_NONSTANDARD_WHITESPACE: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new("^[\u{00a0}\u{2007}]+")
        .multi_line(true)
        .build()
        .unwrap()
});
static WHITESPACE_ONLY_LINE: LazyLock<Replacer> =
    LazyLock::new(|| Replacer::RegexReplace {
        regex: RegexBuilder::new(r"^\s+$")
            .multi_line(true)
            .build()
            .unwrap(),
        replacement: "",
    });
static LEADING_DOCUMENT_WHITESPACE: LazyLock<Replacer> =
    LazyLock::new(|| Replacer::RegexReplace {
        regex: Regex::new(r"^[ \t\n]+").unwrap(),
        replacement: "",
    });
static TRAILING_NEWLINES: LazyLock<Replacer> = LazyLock::new(|| Replacer::RegexReplace {
    regex: Regex::new(r"\n+$").unwrap(),
    replacement: "",
});
static DOS_MAC_NEWLINES: LazyLock<Replacer> = LazyLock::new(|| Replacer::RegexReplace {
    regex: Regex::new(r"\r\n?").unwrap(),
    replacement: "\n",
});
/// Performs all whitespace substitutions in-place in the given text.
pub fn substitute(text: &mut String) {
    let mut buffer = String::new();

    macro_rules! replace {
        ($replacer:expr) => {
            $replacer.replace(text, &mut buffer)
        };
    }

    // Replace DOS and Mac newlines
    replace!(DOS_MAC_NEWLINES);

    // Saved Wikidot trims ASCII whitespace at the beginning of the document,
    // while preserving the same indentation on later physical lines. This is
    // observably different from preview rendering for structural prefixes such
    // as native blockquotes, so the saved-page behavior is authoritative.
    replace!(LEADING_DOCUMENT_WHITESPACE);

    // Replace leading non-standard spaces with regular spaces
    // Leave other non-standard spaces as-is (such as nbsp in
    // the middle of paragraphs)
    replace_leading_spaces(text);

    // Strip lines with only whitespace
    replace!(WHITESPACE_ONLY_LINE);

    // Join concatenated lines (ending with '\').
    join_continued_lines(text, &mut buffer);

    // Tabs and null characters are common one-character substitutions.
    // Replace each class in one linear pass instead of repeatedly shifting
    // the remaining string for every match.
    if text.contains('\t') {
        *text = text.replace('\t', "    ");
    }

    if text.contains('\0') {
        *text = text.replace('\0', " ");
    }

    // Remove trailing newlines
    replace!(TRAILING_NEWLINES);
}

/// Removes line-continuation pairs, including pairs exposed by earlier removals.
///
/// The output buffer acts as a stack: a newline cancels the immediately preceding
/// backslash, whether that backslash was adjacent in the input or exposed by a
/// previous cancellation. Each character is pushed at most once and popped at
/// most once, so cascading continuations are handled in linear time.
fn join_continued_lines(text: &mut String, buffer: &mut String) {
    if !text.contains("\\\n") {
        return;
    }

    buffer.clear();
    buffer.reserve(text.len());

    for character in text.chars() {
        if character == '\n' && buffer.as_bytes().last() == Some(&b'\\') {
            let removed = buffer.pop();
            debug_assert_eq!(removed, Some('\\'));
        } else {
            buffer.push(character);
        }
    }

    std::mem::swap(text, buffer);
}

/// In-place replaces the leading non-standard spaces (such as nbsp) on each line with standard spaces
fn replace_leading_spaces(text: &mut String) {
    trace!("Replacing leading non-standard spaces with regular spaces");

    let mut captures = LEADING_NONSTANDARD_WHITESPACE.captures_iter(text);
    let Some(first_capture) = captures.next() else {
        return;
    };

    let mut buffer = String::with_capacity(text.len());
    let mut last_copied = 0;

    for capture in std::iter::once(first_capture).chain(captures) {
        let mtch = capture
            .get(0)
            .expect("Regular expression lacks a full match");

        let count = mtch.as_str().chars().count();

        buffer.push_str(&text[last_copied..mtch.start()]);
        buffer.extend(std::iter::repeat_n(' ', count));
        last_copied = mtch.end();
    }

    buffer.push_str(&text[last_copied..]);
    *text = buffer;
}

#[cfg(test)]
const TEST_CASES: [(&str, &str); 10] = [
    ("\tapple\n\tbanana\tcherry\n", "apple\n    banana    cherry"),
    (
        "newlines:\r\n* apple\r* banana\r\ncherry\n\r* durian",
        "newlines:\n* apple\n* banana\ncherry\n\n* durian",
    ),
    (
        "apple\nbanana\n\ncherry\n\n\npineapple\n\n\n\nstrawberry\n\n\n\n\nblueberry\n\n\n\n\n\n",
        "apple\nbanana\n\ncherry\n\npineapple\n\nstrawberry\n\nblueberry",
    ),
    (
        "apple\rbanana\r\rcherry\r\r\rpineapple\r\r\r\rstrawberry\r\r\r\r\rblueberry\r\r\r\r\r\r",
        "apple\nbanana\n\ncherry\n\npineapple\n\nstrawberry\n\nblueberry",
    ),
    (
        "concat:\napple banana \\\nCherry\\\nPineapple \\ grape\nblueberry\n",
        "concat:\napple banana CherryPineapple \\ grape\nblueberry",
    ),
    ("<\n        \n      \n  \n      \n>", "<\n\n>"),
    ("\u{00a0}\u{00a0}\u{2007} apple", "    apple"),
    ("x\\\\\n\ny", "xy"),
    ("\\\\\n\nX", "X"),
    (
        "\u{00a0}apple\n\u{2007}\u{00a0}banana\ncherry\u{00a0}",
        " apple\n  banana\ncherry\u{00a0}",
    ),
];

#[test]
fn regexes() {
    let _ = &*LEADING_NONSTANDARD_WHITESPACE;
    let _ = &*WHITESPACE_ONLY_LINE;
    let _ = &*LEADING_DOCUMENT_WHITESPACE;
    let _ = &*TRAILING_NEWLINES;
    let _ = &*DOS_MAC_NEWLINES;
}

#[test]
fn test_substitute() {
    use super::test::test_substitution;

    test_substitution("miscellaneous", substitute, &TEST_CASES);
}

#[test]
fn strips_only_document_leading_ascii_whitespace() {
    let mut text = "\n\t  > first\n  > second".to_owned();

    substitute(&mut text);

    assert_eq!(text, "> first\n  > second");
}

#[test]
fn preserves_indentation_after_non_whitespace_content() {
    let mut text = "[!-- comment --]\n  > literal".to_owned();

    substitute(&mut text);

    assert_eq!(text, "[!-- comment --]\n  > literal");
}

#[test]
fn line_continuations_cascade_across_exposed_boundaries() {
    for depth in [1, 2, 3, 8, 32] {
        let mut text =
            format!("prefix{}{}suffix", "\\".repeat(depth), "\n".repeat(depth),);
        let mut buffer = String::new();

        join_continued_lines(&mut text, &mut buffer);

        assert_eq!(text, "prefixsuffix", "cascade depth {depth}");
    }
}

#[test]
fn linear_line_continuation_join_matches_repeated_replacement() {
    const ALPHABET: [char; 3] = ['\\', '\n', 'x'];

    for length in 0..=9 {
        let combinations = ALPHABET.len().pow(length);
        for mut encoded in 0..combinations {
            let mut input = String::with_capacity(length as usize);
            for _ in 0..length {
                input.push(ALPHABET[encoded % ALPHABET.len()]);
                encoded /= ALPHABET.len();
            }

            let mut expected = input.clone();
            while expected.contains("\\\n") {
                expected = expected.replace("\\\n", "");
            }

            let mut actual = input.clone();
            let mut buffer = String::new();
            join_continued_lines(&mut actual, &mut buffer);

            assert_eq!(actual, expected, "input {input:?}");
        }
    }
}

#[test]
fn line_continuation_cascade_scales_to_large_inputs() {
    // A repeated full-rescan implementation performs quadratic work on this
    // shape because each pass exposes exactly one new continuation boundary.
    const DEPTH: usize = 32 * 1024;
    let mut text = format!("prefix{}{}suffix", "\\".repeat(DEPTH), "\n".repeat(DEPTH),);
    let mut buffer = String::new();

    join_continued_lines(&mut text, &mut buffer);

    assert_eq!(text, "prefixsuffix");
}
