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
//! * Remove characters discarded by Wikidot's input filter
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
// Wikidot treats a line containing a non-breaking space as authored content:
// it remains in the surrounding paragraph and renders as line breaks around
// the space. Strip only ordinary indentation lines in the legacy layout;
// the leading-space pass below preserves an NBSP-only line so the parser can
// emit that authored content instead of manufacturing a paragraph break.
static WIKIDOT_WHITESPACE_ONLY_LINE: LazyLock<Replacer> =
    LazyLock::new(|| Replacer::RegexReplace {
        regex: RegexBuilder::new(r"^[ \t\r\n]+$")
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
static WIKIDOT_CONTINUED_DIV_OPENER: LazyLock<Regex> = LazyLock::new(|| {
    RegexBuilder::new(r"^\[\[div_?(?:[ \t].*)?\]\]$")
        .case_insensitive(true)
        .build()
        .unwrap()
});

pub(super) fn expose_wikidot_replacement_markers(text: &mut String) {
    if !text.contains('\u{fffd}') {
        return;
    }

    let characters = text.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(text.len());
    let mut index = 0;
    let mut line_has_content = false;
    let mut paragraph_line = 1;
    let mut document_has_content = false;

    while index < characters.len() {
        if characters[index] != '\u{fffd}' {
            let character = characters[index];
            if character == '\r' || character == '\n' {
                let mut newline_end = logical_newline_end(&characters, index);
                let mut newline_count = 1;
                while matches!(characters.get(newline_end), Some('\r' | '\n')) {
                    newline_end = logical_newline_end(&characters, newline_end);
                    newline_count += 1;
                }
                output.extend(characters[index..newline_end].iter().copied());
                if document_has_content {
                    if newline_count >= 2 {
                        paragraph_line = 1;
                    } else {
                        paragraph_line += 1;
                    }
                }
                line_has_content = false;
                index = newline_end;
            } else {
                output.push(character);
                line_has_content = true;
                if !character.is_ascii_whitespace() {
                    document_has_content = true;
                }
                index += 1;
            }
            continue;
        }

        let run_start = index;
        while characters.get(index) == Some(&'\u{fffd}') {
            index += 1;
        }
        if index - run_start != 1 {
            continue;
        }

        match characters.get(index).copied() {
            None => {
                output.push('2');
                line_has_content = true;
            }
            Some('\\') if index + 1 == characters.len() => {
                output.push_str(&paragraph_line.to_string());
                output.push('\u{fffd}');
                line_has_content = true;
            }
            Some('\r' | '\n') => {
                let first_end = logical_newline_end(&characters, index);
                if matches!(characters.get(first_end), Some('\r' | '\n')) {
                    output.push_str("23");
                    index = logical_newline_end(&characters, first_end);
                } else {
                    output.push(if line_has_content { '1' } else { '2' });
                    index = first_end;
                }
                line_has_content = true;
            }
            Some(_) => {}
        }
    }

    *text = output;
}

pub(super) fn preserve_wikidot_document_indentation_barrier(text: &mut String) {
    let leading_len = text
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t' | b'\n'))
        .count();
    if wikidot_spaced_inner_include_example(text, leading_len) {
        return;
    }
    let structural = text.as_bytes().get(leading_len).copied();
    if text[..leading_len]
        .bytes()
        .any(|byte| matches!(byte, b' ' | b'\t'))
        && matches!(structural, Some(b'>' | b'=' | b'+' | b'|' | b'[' | b'_'))
    {
        let replacement = if text.as_bytes().get(leading_len) == Some(&b'_') {
            "\0 "
        } else {
            "\0"
        };
        text.replace_range(..leading_len, replacement);
    }
}

pub(super) fn preserve_wikidot_terminal_backslash_run(text: &mut String) {
    let trailing_newline_start = text.trim_end_matches(['\r', '\n']).len();
    if trailing_newline_start < text.len()
        && text[..trailing_newline_start].ends_with('\\')
    {
        text.truncate(trailing_newline_start);
    }
    if !text.ends_with('\\') {
        return;
    }
    let prefix = &text[..text.len() - 1];
    if prefix.ends_with('\r') || prefix.ends_with('\n') {
        text.pop();
        return;
    }
    if prefix.ends_with('\\') {
        text.insert(text.len() - 1, '\u{0001}');
    }
}

fn logical_newline_end(characters: &[char], start: usize) -> usize {
    if characters[start] == '\r' && characters.get(start + 1) == Some(&'\n') {
        start + 2
    } else {
        start + 1
    }
}
/// Performs all whitespace substitutions in-place in the given text.
pub fn substitute(text: &mut String) {
    substitute_for_layout(text, false);
}

pub(super) fn substitute_wikidot(text: &mut String) {
    substitute_for_layout(text, true);
}

/// Collapse physical lines containing only whitespace without applying the
/// rest of the document-level whitespace preprocessor.
///
/// Delayed List-mode callers already hold source ranges for runtime values,
/// so they cannot safely rerun all preprocessing after substitution. This
/// source-preserving boundary is the one Wikidot operation they need before
/// constructing those ranges.
pub fn normalize_wikidot_whitespace_only_lines(text: &mut String) {
    let mut buffer = String::new();
    WHITESPACE_ONLY_LINE.replace(text, &mut buffer);
}

fn substitute_for_layout(text: &mut String, wikidot_compatibility: bool) {
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
    if wikidot_compatibility {
        preserve_document_leading_indentation_barrier(text);
    }
    replace!(LEADING_DOCUMENT_WHITESPACE);

    if wikidot_compatibility {
        // NBSP-only lines are not blank lines to Wikidot. Remove ordinary
        // indentation-only lines before converting leading NBSP characters,
        // then preserve the latter as paragraph content.
        replace!(WIKIDOT_WHITESPACE_ONLY_LINE);
        replace_leading_spaces(text);
    } else {
        // Replace leading non-standard spaces with regular spaces and strip
        // lines with only whitespace for the native parser.
        replace_leading_spaces(text);
        replace!(WHITESPACE_ONLY_LINE);
    }

    // Join concatenated lines (ending with '\').
    join_continued_lines(text, &mut buffer, wikidot_compatibility);

    // Replace tabs in one linear pass instead of repeatedly shifting the
    // remaining string for every match. Wikidot normally collapses a tab to
    // one parser-space, but preserves its four-column width inside a legacy
    // `getAttrs` quoted value. That distinction remains observable in custom
    // collapsible labels, where ordinary spaces become non-breaking spaces.
    replace_tabs(text, &mut buffer, wikidot_compatibility);

    if !wikidot_compatibility && text.contains('\0') {
        *text = text.replace('\0', " ");
    }

    // Remove trailing newlines
    replace!(TRAILING_NEWLINES);
}

fn append_replaced_tabs(output: &mut String, input: &str, width: usize) {
    for ch in input.chars() {
        if ch == '\t' {
            output.extend(std::iter::repeat_n(' ', width));
        } else {
            output.push(ch);
        }
    }
}

fn append_wikidot_block_head_tabs(output: &mut String, head: &str) {
    let delimiters = head
        .match_indices("=\"")
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let mut value_ranges = Vec::with_capacity(delimiters.len());

    for (index, delimiter) in delimiters.iter().copied().enumerate() {
        let value_start = delimiter + 2;
        let segment_end = delimiters.get(index + 1).copied().unwrap_or(head.len());
        let segment = &head[value_start..segment_end];
        if let Some(value_end) = segment.rfind('"') {
            value_ranges.push(value_start..value_start + value_end);
        }
    }

    let mut range_index = 0;
    for (index, ch) in head.char_indices() {
        while value_ranges
            .get(range_index)
            .is_some_and(|range| range.end <= index)
        {
            range_index += 1;
        }
        let in_value = value_ranges
            .get(range_index)
            .is_some_and(|range| range.contains(&index));

        if ch == '\t' {
            output.extend(std::iter::repeat_n(' ', if in_value { 4 } else { 1 }));
        } else {
            output.push(ch);
        }
    }
}

fn replace_tabs(text: &mut String, buffer: &mut String, wikidot_compatibility: bool) {
    if !text.contains('\t') {
        return;
    }

    buffer.clear();
    buffer.reserve(text.len());
    if !wikidot_compatibility {
        append_replaced_tabs(buffer, text, 4);
        std::mem::swap(text, buffer);
        return;
    }

    let mut cursor = 0;
    while let Some(open_offset) = text[cursor..].find("[[") {
        let open = cursor + open_offset;
        append_replaced_tabs(buffer, &text[cursor..open], 1);

        let end = text[open + 2..]
            .find("]]")
            .map_or(text.len(), |close_offset| open + 2 + close_offset + 2);
        append_wikidot_block_head_tabs(buffer, &text[open..end]);
        cursor = end;
        if cursor == text.len() {
            break;
        }
    }
    append_replaced_tabs(buffer, &text[cursor..], 1);
    std::mem::swap(text, buffer);
}

fn preserve_document_leading_indentation_barrier(text: &mut String) {
    let leading_len = text
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t' | b'\n'))
        .count();
    if leading_len == 0 {
        return;
    }
    if wikidot_spaced_inner_include_example(text, leading_len) {
        return;
    }
    let leading = &text[..leading_len];
    let structural = text.as_bytes().get(leading_len).copied();
    if leading.bytes().any(|byte| matches!(byte, b' ' | b'\t'))
        && structural == Some(b'>')
    {
        text.replace_range(..leading_len, "\0");
    }
}

fn wikidot_spaced_inner_include_example(text: &str, leading_len: usize) -> bool {
    const PREFIX: &str = "> > [[include ";
    let line = text[leading_len..]
        .split_once('\n')
        .map_or(&text[leading_len..], |(line, _)| line);
    line.get(..PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(PREFIX))
}

pub(super) fn filter_characters(text: &mut String, preserve_terminal_marker: bool) {
    // U+FFFD is an unsafe Wikidot-internal token delimiter. Other discarded
    // controls remain until tokenization because Wikidot uses them as invisible
    // syntax barriers before dropping their output.
    let terminal_marker = preserve_terminal_marker && text.ends_with("\u{fffd}\\");
    if terminal_marker {
        text.truncate(text.len() - "\u{fffd}\\".len());
    }
    text.retain(|character| character != '\u{fffd}');
    if terminal_marker {
        text.push_str("\u{fffd}\\");
    }
    let mut buffer = String::new();
    LEADING_DOCUMENT_WHITESPACE.replace(text, &mut buffer);
}

/// Removes line-continuation pairs, including pairs exposed by earlier removals.
///
/// The output buffer acts as a stack: a newline cancels the immediately preceding
/// backslash, whether that backslash was adjacent in the input or exposed by a
/// previous cancellation. Each character is pushed at most once and popped at
/// most once, so cascading continuations are handled in linear time.
fn join_continued_lines(
    text: &mut String,
    buffer: &mut String,
    wikidot_compatibility: bool,
) {
    if !text.contains("\\\n") {
        return;
    }

    buffer.clear();
    buffer.reserve(text.len());

    for (index, character) in text.char_indices() {
        if character == '\n' && buffer.as_bytes().last() == Some(&b'\\') {
            if wikidot_compatibility && wikidot_continued_div_opener(&text[index + 1..]) {
                buffer.push(character);
            } else {
                let removed = buffer.pop();
                debug_assert_eq!(removed, Some('\\'));
            }
        } else {
            buffer.push(character);
        }
    }

    std::mem::swap(text, buffer);
}

fn wikidot_continued_div_opener(text: &str) -> bool {
    // Wikidot suppresses the visible break but still recognizes a standalone
    // div opener on the next physical line.
    let line = text.split_once('\n').map_or(text, |(line, _)| line);
    let marker = line.trim_matches([' ', '\t']);
    WIKIDOT_CONTINUED_DIV_OPENER.is_match(marker)
}

/// In-place replaces leading non-standard spaces (such as nbsp) on content
/// lines with standard spaces. A line containing only those spaces and ASCII
/// indentation is authored Wikidot content and must retain its NBSP bytes.
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
        let line_suffix = text[mtch.end()..]
            .split_once('\n')
            .map_or(&text[mtch.end()..], |(line, _)| line);
        if line_suffix
            .chars()
            .all(|character| matches!(character, ' ' | '\t' | '\r'))
        {
            buffer.push_str(mtch.as_str());
        } else {
            buffer.extend(std::iter::repeat_n(' ', count));
        }
        last_copied = mtch.end();
    }

    buffer.push_str(&text[last_copied..]);
    *text = buffer;
}

#[cfg(test)]
const TEST_CASES: [(&str, &str); 10] = [
    ("\tapple\n\tbanana\tcherry\n", "apple\n banana cherry"),
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
    let _ = &*WIKIDOT_WHITESPACE_ONLY_LINE;
    let _ = &*LEADING_DOCUMENT_WHITESPACE;
    let _ = &*TRAILING_NEWLINES;
    let _ = &*DOS_MAC_NEWLINES;
}

#[test]
fn test_substitute() {
    use super::test::test_substitution;

    test_substitution("miscellaneous", substitute_wikidot, &TEST_CASES);
}

#[test]
fn wikidot_preserves_nbsp_only_lines_inside_a_paragraph() {
    let mut text = "Alpha\n\u{00a0}\nBeta\n\nGamma".to_owned();

    substitute_wikidot(&mut text);

    assert_eq!(text, "Alpha\n\u{00a0}\nBeta\n\nGamma");
}

#[test]
fn wikijump_substitution_keeps_native_tabs_and_null_handling() {
    let mut text = "a\tb\0c".to_owned();
    substitute(&mut text);
    assert_eq!(text, "a    b c");
}

#[test]
fn wikidot_substitution_preserves_tab_width_inside_getattrs_values() {
    let mut text = concat!(
        "plain\ttext\n",
        "[[collapsible show=\"OPEN\tSECOND\" ",
        "bogus=bare id=\"alpha\tbeta\"]]",
    )
    .to_owned();

    substitute_wikidot(&mut text);

    assert_eq!(
        text,
        concat!(
            "plain text\n",
            "[[collapsible show=\"OPEN    SECOND\" ",
            "bogus=bare id=\"alpha    beta\"]]",
        ),
    );
}

#[test]
fn preserves_a_syntax_barrier_for_an_indented_leading_quote() {
    let mut text = "\n\t  > first\n  > second".to_owned();

    substitute_wikidot(&mut text);

    assert_eq!(text, "\0> first\n  > second");
}

#[test]
fn trims_document_indentation_before_a_spaced_inner_include_example() {
    let mut text = "  > > [[include component:box |name=x]]".to_owned();

    preserve_wikidot_document_indentation_barrier(&mut text);
    substitute_wikidot(&mut text);

    assert_eq!(text, "> > [[include component:box |name=x]]");

    let mut source = "  > > [[include component:box |name=x]]".to_owned();
    let page_info = crate::data::PageInfo::dummy();
    let settings = crate::settings::WikitextSettings::from_mode(
        crate::settings::WikitextMode::Page,
        crate::layout::Layout::Wikidot,
    );
    crate::preprocess_for_layout(&mut source, settings.layout);
    let tokens = crate::tokenize(&source);
    let (tree, _) = crate::parse(&tokens, &page_info, &settings).into();
    let html = crate::render::Render::render(
        &crate::render::html::HtmlRender,
        &tree,
        &page_info,
        &settings,
    )
    .body;
    assert_eq!(
        html,
        "<blockquote><p>&gt; [[include component:box |name=x]]</p></blockquote>",
    );
}

#[test]
fn preserves_a_syntax_barrier_for_indented_non_list_document_openers() {
    for (input, expected) in [
        ("\t= x", "\0= x"),
        (" + x", "\0+ x"),
        (" [[div]]x[[/div]]", "\0[[div]]x[[/div]]"),
    ] {
        let mut text = input.to_owned();
        preserve_wikidot_document_indentation_barrier(&mut text);
        substitute_wikidot(&mut text);
        assert_eq!(text, expected);
    }
}

#[test]
fn trims_document_indentation_before_list_openers() {
    for (input, expected) in [(" * x", "* x"), ("\t# x", "# x")] {
        let mut text = input.to_owned();
        preserve_wikidot_document_indentation_barrier(&mut text);
        substitute_wikidot(&mut text);
        assert_eq!(text, expected);
    }
}

#[test]
fn trims_indentation_before_plain_document_text() {
    let mut text = "\t\rplain".to_owned();
    preserve_wikidot_document_indentation_barrier(&mut text);
    substitute_wikidot(&mut text);
    assert_eq!(text, "plain");
}

#[test]
fn preserves_wikidot_terminal_backslash_runs() {
    for (input, expected) in [
        ("a\\", "a\\"),
        ("a\\\\", "a\\\u{0001}\\"),
        ("a\\\\\\", "a\\\\\u{0001}\\"),
        ("a\\\n\n", "a\\"),
        ("a\n\\", "a\n"),
        ("a\r\\", "a\r"),
    ] {
        let mut text = input.to_owned();
        preserve_wikidot_terminal_backslash_run(&mut text);
        assert_eq!(text, expected);
    }
}

#[test]
fn preserves_indentation_after_non_whitespace_content() {
    let mut text = "[!-- comment --]\n  > literal".to_owned();

    substitute(&mut text);

    assert_eq!(text, "[!-- comment --]\n  > literal");
}

#[test]
fn retains_control_barriers_and_removes_unsafe_replacement_marker() {
    let mut text = String::new();
    text.extend([
        'A', '\0', '\u{0001}', '\u{0008}', '\u{000b}', '\u{000c}', '\u{000e}',
        '\u{001a}', '\u{001b}', '\u{001c}', '\u{001f}', '\u{007f}', '\u{fffd}', 'B',
    ]);

    filter_characters(&mut text, false);

    assert_eq!(
        text,
        "A\0\u{0001}\u{0008}\u{000b}\u{000c}\u{000e}\u{001a}\u{001b}\u{001c}\u{001f}\u{007f}B"
    );
}

#[test]
fn exposes_wikidot_replacement_markers_at_input_boundaries() {
    for (input, expected) in [
        ("\u{fffd}", "2"),
        ("a\u{fffd}", "a2"),
        ("\u{fffd}a", "a"),
        ("a\u{fffd}b", "ab"),
        ("\u{fffd}\u{fffd}", ""),
        ("a\u{fffd}\nb", "a1b"),
        ("a\u{fffd}\r\nb", "a1b"),
        ("a\u{fffd}\n\nb", "a23b"),
        ("a\n\u{fffd}\nb", "a\n2b"),
        ("a\u{fffd}\\", "a1\u{fffd}\\"),
        ("a\rb\u{fffd}\\", "a\rb2\u{fffd}\\"),
        ("a\rb\rc\u{fffd}\\", "a\rb\rc3\u{fffd}\\"),
        ("a\n\nb\u{fffd}\\", "a\n\nb1\u{fffd}\\"),
        ("\ra\u{fffd}\\", "\ra1\u{fffd}\\"),
    ] {
        let mut text = input.to_owned();
        expose_wikidot_replacement_markers(&mut text);
        assert_eq!(text, expected, "{input:?}");
    }
}

#[test]
fn character_filter_preserves_a_control_as_a_line_continuation_barrier() {
    let mut text = "A\\\u{0001}\rB".to_owned();

    crate::preprocess_for_layout(&mut text, crate::layout::Layout::Wikidot);

    assert_eq!(text, "A\\\u{0001}\nB");
}

#[test]
fn wikidot_preserves_a_continued_standalone_div_boundary() {
    for (opener, normalized) in [
        ("[[div]]", "[[div]]"),
        ("[[div class=\"box\"]]", "[[div class=\"box\"]]"),
        ("[[div_ class=\"box\"]]", "[[div_ class=\"box\"]]"),
        ("[[div\tclass=\"box\"]]", "[[div class=\"box\"]]"),
        ("[[div_\tclass=\"box\"]]", "[[div_ class=\"box\"]]"),
        ("[[div @=\"value\"]]", "[[div @=\"value\"]]"),
        ("[[div_ class]]", "[[div_ class]]"),
    ] {
        let mut wikidot = format!("alpha\\\n{opener}\nbody\n[[/div]]");
        substitute_wikidot(&mut wikidot);
        assert_eq!(wikidot, format!("alpha\\\n{normalized}\nbody\n[[/div]]"),);

        let mut wikijump = format!("alpha\\\n{opener}\nbody\n[[/div]]");
        substitute(&mut wikijump);
        assert_eq!(
            wikijump,
            format!("alpha{}\nbody\n[[/div]]", opener.replace('\t', "    ")),
        );
    }

    let mut inline = "alpha\\\nbeta".to_owned();
    substitute_wikidot(&mut inline);
    assert_eq!(inline, "alphabeta");
}

#[test]
fn malformed_div_like_lines_remain_ordinary_continuations() {
    for opener in ["[[division]]", "[[div__]]", "[[divx]]"] {
        let mut text = format!("alpha\\\n{opener}");
        substitute_wikidot(&mut text);
        assert_eq!(text, format!("alpha{opener}"));
    }
}

#[test]
fn control_barrier_prevents_late_leading_whitespace_trimming() {
    let mut text = "\u{0001}\tA".to_owned();

    crate::preprocess_for_layout(&mut text, crate::layout::Layout::Wikidot);

    assert_eq!(text, "\u{0001} A");
}

#[test]
fn line_continuations_cascade_across_exposed_boundaries() {
    for depth in [1, 2, 3, 8, 32] {
        let mut text =
            format!("prefix{}{}suffix", "\\".repeat(depth), "\n".repeat(depth),);
        let mut buffer = String::new();

        join_continued_lines(&mut text, &mut buffer, false);

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
            join_continued_lines(&mut actual, &mut buffer, false);

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

    join_continued_lines(&mut text, &mut buffer, false);

    assert_eq!(text, "prefixsuffix");
}
