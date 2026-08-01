/*
 * preproc/typography.rs
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

//! Perform Wikidot's typographical modifications.
//! For full information, see the original source file:
//! <https://github.com/gabrys/wikidot/blob/master/lib/Text_Wiki/Text/Wiki/Parse/Default/Typography.php>
//!
//! The transformations performed here are listed:
//! * `` .. '' to fancy double quotes
//! * ` .. ' to fancy single quotes
//! * ,, .. '' to fancy lowered double quotes
//! * ... to an ellipsis
//!
//! Em dash conversion was originally implemented here, however
//! it was moved to the parser to prevent typography from converting
//! the `--` in `[!--` and `--]` into em dashes.

use super::Replacer;
use super::parser_functions::LiteralRegionIndex;
use regex::Regex;
use std::ops::Range;
use std::sync::LazyLock;

// ‘ - LEFT SINGLE QUOTATION MARK
// ’ - RIGHT SINGLE QUOTATION MARK
static SINGLE_QUOTES: LazyLock<Replacer> = LazyLock::new(|| Replacer::RegexSurround {
    regex: Regex::new(r"(?s)`(.*?)'").unwrap(),
    begin: "\u{2018}",
    end: "\u{2019}",
});

// “ - LEFT DOUBLE QUOTATION MARK
// ” - RIGHT DOUBLE QUOTATION MARK
static DOUBLE_QUOTES: LazyLock<Replacer> = LazyLock::new(|| Replacer::RegexSurround {
    regex: Regex::new(r"(?s)``(.*?)''").unwrap(),
    begin: "\u{201c}",
    end: "\u{201d}",
});

// „ - DOUBLE LOW-9 QUOTATION MARK
static LOW_DOUBLE_QUOTES: LazyLock<Replacer> =
    LazyLock::new(|| Replacer::RegexSurround {
        regex: Regex::new(r"(?s),,(.*?)''").unwrap(),
        begin: "\u{201e}",
        end: "\u{201d}",
    });

static NATIVE_SINGLE_QUOTES: LazyLock<Replacer> =
    LazyLock::new(|| Replacer::RegexSurround {
        regex: Regex::new(r"`(.*?)'").unwrap(),
        begin: "\u{2018}",
        end: "\u{2019}",
    });

static NATIVE_DOUBLE_QUOTES: LazyLock<Replacer> =
    LazyLock::new(|| Replacer::RegexSurround {
        regex: Regex::new(r"``(.*?)''").unwrap(),
        begin: "\u{201c}",
        end: "\u{201d}",
    });

static NATIVE_LOW_DOUBLE_QUOTES: LazyLock<Replacer> =
    LazyLock::new(|| Replacer::RegexSurround {
        regex: Regex::new(r",,(.*?)''").unwrap(),
        begin: "\u{201e}",
        end: "\u{201d}",
    });

// « - LEFT-POINTING DOUBLE ANGLE QUOTATION MARK
static LEFT_ANGLE_QUOTES: LazyLock<Replacer> = LazyLock::new(|| Replacer::RegexReplace {
    regex: Regex::new(r"<<").unwrap(),
    replacement: "\u{ab}",
});

// » - RIGHT-POINTING DOUBLE ANGLE QUOTATION MARK
static RIGHT_ANGLE_QUOTES: LazyLock<Replacer> =
    LazyLock::new(|| Replacer::RegexReplace {
        regex: Regex::new(r"[^\n>](?<repl>>>)").unwrap(),
        replacement: "\u{bb}",
    });

static NUMBER_UNITS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
        (?P<number>[0-9])\x20
        (?P<unit>
            [pµmcdhkMGT]?
            (?:mol|rad|Hz|Pa|Wb|lm|lx|Bq|Gy|Sv|kat|cd|Ohm|Ω|&micro;|&\#0*181;|&\#[xX]0*[Bb]5;|&Omega;|&\#0*937;|&\#[xX]0*3[Aa]9;|[mgstAKNJWCVFSTHBL])
            |
            [kKMGT]?(?:[oBb](?:ps)?|flops)
            |
            ¢|&cent;|&\#0*162;|&\#[xX]0*[Aa]2;
            |
            M?(?:£|&pound;|&\#0*163;|&\#[xX]0*[Aa]3;|¥|&yen;|&\#0*165;|&\#[xX]0*[Aa]5;|€|&euro;|&\#0*8364;|&\#[xX]0*20[Aa][Cc];|\$)
            |
            (?:°|&deg;|&\#0*176;|&\#[xX]0*[Bb]0;)[CF]?
            |
            %|pt|pi|M?px|em|en|gal|lb|[NSEOW]|[NS][EOW]|ha|mbar
        )",
    )
    .unwrap()
});

// … - HORIZONTAL ELLIPSIS
static HORIZONTAL_ELLIPSIS: LazyLock<Replacer> =
    LazyLock::new(|| Replacer::RegexReplace {
        regex: Regex::new(r"(?<repl>\.\.\.|\. \. \.)").unwrap(),
        replacement: "\u{2026}",
    });

fn replace_within_paragraphs(
    replacer: &Replacer,
    text: &mut String,
    buffer: &mut String,
) {
    let mut output = String::with_capacity(text.len());
    for paragraph in text.split_inclusive("\n\n") {
        let mut paragraph = paragraph.to_owned();
        replacer.replace(&mut paragraph, buffer);
        output.push_str(&paragraph);
    }
    *text = output;
}

fn replace_low_quotes_within_paragraphs(text: &mut String, buffer: &mut String) {
    let mut output = String::with_capacity(text.len());
    for paragraph in text.split_inclusive("\n\n") {
        let positions = paragraph
            .match_indices(",,")
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let paired_count = positions.len() / 2 * 2;
        let protected_end = paired_count
            .checked_sub(1)
            .map_or(0, |index| positions[index] + 2);
        output.push_str(&paragraph[..protected_end]);

        let mut suffix = paragraph[protected_end..].to_owned();
        LOW_DOUBLE_QUOTES.replace(&mut suffix, buffer);
        output.push_str(&suffix);
    }
    *text = output;
}

fn wikidot_uses_getattrs(head: &str) -> bool {
    let name = head
        .strip_prefix("[[")
        .unwrap_or(head)
        .trim_start_matches([' ', '\t'])
        .split(|character: char| {
            character.is_ascii_whitespace() || matches!(character, ']' | '_')
        })
        .next()
        .unwrap_or_default();
    ["collapsible", "div", "table", "row", "cell", "hcell"]
        .iter()
        .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

fn wikidot_getattrs_attribute_ranges(text: &str) -> Vec<Range<usize>> {
    if !text.contains("=\"") {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative_open) = text[cursor..].find("[[") {
        let open = cursor + relative_open;
        let end = text[open + 2..]
            .find("]]")
            .map_or(text.len(), |relative_close| open + 2 + relative_close + 2);
        let head = &text[open..end];
        if !wikidot_uses_getattrs(head) {
            cursor = end;
            if cursor == text.len() {
                break;
            }
            continue;
        }

        // These six Wikidot blocks use the legacy getAttrs path, not FTML's
        // generic quoted-argument parser. Match its exact `="` delimiter,
        // last-quote recovery, and first-`]]` head boundary.
        let delimiters = head
            .match_indices("=\"")
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        for (index, delimiter) in delimiters.iter().copied().enumerate() {
            let value_start = delimiter + 2;
            let segment_end = delimiters.get(index + 1).copied().unwrap_or(head.len());
            let segment = &head[value_start..segment_end];
            if let Some(value_end) = segment.rfind('"') {
                ranges.push(open + value_start..open + value_start + value_end);
            }
        }
        cursor = end;
        if cursor == text.len() {
            break;
        }
    }
    ranges
}

fn wikidot_local_link_separator_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative_open) = text[cursor..].find('[') {
        let open = cursor + relative_open;
        let body_start = open + '['.len_utf8();
        if text[body_start..].starts_with('[') {
            cursor = body_start + '['.len_utf8();
            continue;
        }
        let line_end = text[body_start..]
            .find('\n')
            .map_or(text.len(), |offset| body_start + offset);
        let Some(relative_close) = text[body_start..line_end].find(']') else {
            // There can be no further local-link opener before this line's
            // end. Leave the incomplete bracket literal and finish safely.
            break;
        };
        let close = body_start + relative_close;
        let body = &text[body_start..close];
        let Some(separator) = body.find(' ') else {
            cursor = close + ']'.len_utf8();
            continue;
        };
        let target = &body[..separator];
        if target.starts_with('/')
            && target.len() > 1
            && !target.contains(char::is_whitespace)
        {
            let separator = body_start + separator;
            ranges.push(separator..separator + ' '.len_utf8());
        }
        cursor = close + ']'.len_utf8();
    }
    ranges
}

fn wikidot_typography_protected_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = wikidot_getattrs_attribute_ranges(text);
    ranges.extend(wikidot_local_link_separator_ranges(text));
    ranges.sort_by_key(|range| range.start);
    ranges
}

fn digit_space_positions(text: &str) -> Vec<usize> {
    text.match_indices(' ')
        .filter_map(|(space_index, _)| {
            space_index
                .checked_sub(1)
                .and_then(|index| text.as_bytes().get(index))
                .is_some_and(u8::is_ascii_digit)
                .then_some(space_index)
        })
        .collect()
}

fn index_in_ranges(ranges: &[Range<usize>], index: usize) -> bool {
    let candidate = ranges.partition_point(|range| range.end <= index);
    ranges
        .get(candidate)
        .is_some_and(|range| range.start <= index)
}

fn replace_number_spaces(
    text: &mut String,
    protected: &[Range<usize>],
    digit_spaces: &[usize],
) -> bool {
    let mut spaces = Vec::new();
    for &space_index in digit_spaces {
        if !text
            .as_bytes()
            .get(space_index + 1)
            .is_some_and(u8::is_ascii_digit)
        {
            continue;
        }
        if index_in_ranges(protected, space_index) {
            continue;
        }
        spaces.push(space_index);
    }
    if spaces.is_empty() {
        return false;
    }

    let mut buffer = String::with_capacity(text.len() + spaces.len());
    let mut last_copied = 0;
    for space_index in spaces {
        buffer.push_str(&text[last_copied..space_index]);
        buffer.push('\u{a0}');
        last_copied = space_index + 1;
    }
    buffer.push_str(&text[last_copied..]);
    *text = buffer;
    true
}

fn replace_unit_spaces(text: &mut String, protected: &[Range<usize>]) {
    let mut buffer = String::with_capacity(text.len());
    let mut last_copied = 0;

    for captures in NUMBER_UNITS.captures_iter(text) {
        let full_match = captures.get(0).unwrap();
        let following = text[full_match.end()..].chars().next();
        if following.is_some_and(|character| character.is_ascii_alphanumeric()) {
            continue;
        }

        let number = captures.name("number").unwrap();
        let unit = captures.name("unit").unwrap();
        if index_in_ranges(protected, number.end()) {
            continue;
        }
        buffer.push_str(&text[last_copied..number.end()]);
        buffer.push('\u{a0}');
        buffer.push_str(unit.as_str());
        last_copied = full_match.end();
    }

    if last_copied > 0 {
        buffer.push_str(&text[last_copied..]);
        *text = buffer;
    }
}

/// Performs all typographic substitutions in-place in the given text
pub fn substitute(text: &mut String) {
    let mut buffer = String::new();
    NATIVE_DOUBLE_QUOTES.replace(text, &mut buffer);
    NATIVE_LOW_DOUBLE_QUOTES.replace(text, &mut buffer);
    NATIVE_SINGLE_QUOTES.replace(text, &mut buffer);
    HORIZONTAL_ELLIPSIS.replace(text, &mut buffer);
}

/// Performs Wikidot-compatible typographic substitutions in-place.
///
/// This is available separately for typed delayed inputs whose authored
/// segments have already passed the other layout-specific preprocessing
/// phases.
pub fn substitute_wikidot(text: &mut String) {
    let mut buffer = String::new();
    debug!("Performing typography substitutions");

    macro_rules! replace {
        ($replacer:expr) => {
            $replacer.replace(text, &mut buffer)
        };
    }

    // Quotes
    replace_within_paragraphs(&DOUBLE_QUOTES, text, &mut buffer);
    replace_low_quotes_within_paragraphs(text, &mut buffer);
    replace_within_paragraphs(&SINGLE_QUOTES, text, &mut buffer);
    replace!(LEFT_ANGLE_QUOTES);
    replace!(RIGHT_ANGLE_QUOTES);

    let digit_spaces = digit_space_positions(text);
    let mut protected_ranges = if digit_spaces.is_empty() {
        Vec::new()
    } else {
        wikidot_typography_protected_ranges(text)
    };
    if replace_number_spaces(text, &protected_ranges, &digit_spaces) {
        // Recompute after replacing number spaces because U+00A0 occupies one
        // more UTF-8 byte than the ASCII space it replaces.
        protected_ranges = wikidot_typography_protected_ranges(text);
    }
    replace_unit_spaces(text, &protected_ranges);

    // Miscellaneous
    replace_wikidot_ellipsis_outside_literals(text, &mut buffer);
}

fn replace_wikidot_ellipsis_outside_literals(text: &mut String, buffer: &mut String) {
    let literal_regions = LiteralRegionIndex::new(text);
    if literal_regions.ranges().is_empty() {
        HORIZONTAL_ELLIPSIS.replace(text, buffer);
        return;
    }

    let source = std::mem::take(text);
    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;
    for range in literal_regions.ranges() {
        let mut authored = source[cursor..range.start].to_owned();
        HORIZONTAL_ELLIPSIS.replace(&mut authored, buffer);
        output.push_str(&authored);
        output.push_str(&source[range.clone()]);
        cursor = range.end;
    }
    let mut authored = source[cursor..].to_owned();
    HORIZONTAL_ELLIPSIS.replace(&mut authored, buffer);
    output.push_str(&authored);
    *text = output;
}

#[cfg(test)]
const TEST_CASES: [(&str, &str); 39] = [
    (
        "John laughed. ``You'll never defeat me!''\n``That's where you're wrong...''",
        "John laughed. “You'll never defeat me!”\n“That's where you're wrong…”",
    ),
    ("``outer ``inner'' outer''", "“outer ‘`inner” outer’'"),
    (
        ",,あんたは馬鹿です！''\n``Ehh?''\n,,本当！''\n[[footnoteblock]]",
        ",,あんたは馬鹿です！''\n“Ehh?”\n,,本当！''\n[[footnoteblock]]",
    ),
    ("`one\nline'", "‘one\nline’"),
    ("`one\n\nline'", "`one\n\nline'"),
    (
        "**ENTITY MAKES DRAMATIC MOTION** . . . ",
        "**ENTITY MAKES DRAMATIC MOTION** … ",
    ),
    ("Whales... they are cool", "Whales… they are cool"),
    ("Whales ... they are cool", "Whales … they are cool"),
    ("Whales. . . they are cool", "Whales… they are cool"),
    ("Whales . . . they are cool", "Whales … they are cool"),
    ("...why would you think that?", "…why would you think that?"),
    (
        "... why would you think that?",
        "… why would you think that?",
    ),
    (
        ". . .why would you think that?",
        "…why would you think that?",
    ),
    (
        ". . . why would you think that?",
        "… why would you think that?",
    ),
    ("how could you...", "how could you…"),
    ("how could you ...", "how could you …"),
    ("how could you. . .", "how could you…"),
    ("how could you . . .", "how could you …"),
    // Spaced with an extra dot after the third.
    (". . .. ....", "…. …."),
    // Multiple spaced dots in a row
    ("... . . . . . .", "… … …"),
    // Live Wikidot greedily consumes every complete group of three periods
    // from a longer run and leaves the one- or two-period remainder.
    (".... ..", "…. .."),
    (".....", "….."),
    ("......", "……"),
    (".......", "……."),
    ("........", "…….."),
    (".........", "………"),
    ("..........", "………."),
    ("...........", "……….."),
    ("............", "…………"),
    // Groups of three dots
    ("... ... ...", "… … …"),
    // Groups of three, mixed spaced and continuous
    ("... . . . ...", "… … …"),
    // Context characters can overlap between replacement matches.
    ("x... ...y. . . z", "x… …y… z"),
    ("<<French quotes>>", "«French quotes»"),
    ("1 234 567", "1\u{a0}234\u{a0}567"),
    ("12 kg", "12\u{a0}kg"),
    ("12 m", "12\u{a0}m"),
    ("12 Mpx", "12\u{a0}Mpx"),
    ("12 foo", "12 foo"),
    ("12 kgx", "12 kgx"),
];

#[test]
fn regexes() {
    let _ = &*SINGLE_QUOTES;
    let _ = &*DOUBLE_QUOTES;
    let _ = &*LOW_DOUBLE_QUOTES;
    let _ = &*NATIVE_SINGLE_QUOTES;
    let _ = &*NATIVE_DOUBLE_QUOTES;
    let _ = &*NATIVE_LOW_DOUBLE_QUOTES;
    let _ = &*LEFT_ANGLE_QUOTES;
    let _ = &*RIGHT_ANGLE_QUOTES;
    let _ = &*NUMBER_UNITS;
    let _ = &*HORIZONTAL_ELLIPSIS;
}

#[test]
fn test_substitute() {
    use super::test::test_substitution;

    test_substitution("typography", substitute_wikidot, &TEST_CASES);
}

#[test]
fn wikidot_typography_leaves_an_unmatched_local_bracket_literal() {
    let mut text = "1 [".to_owned();

    substitute_wikidot(&mut text);

    assert_eq!(text, "1 [");
}

#[test]
fn wikijump_substitution_keeps_native_typography_scope() {
    let mut text = "`one\nline' << 1 234".to_owned();
    substitute(&mut text);
    assert_eq!(text, "`one\nline' << 1 234");
}

#[test]
fn wikidot_preprocessing_keeps_literal_dot_runs_outside_authored_prose() {
    let mut text = concat!(
        "PROSE:x....x\n",
        "SPACED:x. . .x\n",
        "[[code]]\n",
        "CODE:x....x\n",
        "[[/code]]\n",
        "@@ESCAPED:x....x@@",
    )
    .to_owned();

    crate::preprocess_for_layout(&mut text, crate::layout::Layout::Wikidot);

    assert!(text.contains("PROSE:x….x"), "{text}");
    assert!(text.contains("SPACED:x…x"), "{text}");
    assert!(text.contains("CODE:x....x"), "{text}");
    assert!(text.contains("ESCAPED:x....x"), "{text}");
}

#[test]
fn wikidot_unit_typography_skips_quoted_block_attributes() {
    let mut text = concat!(
        "NUMBER: 1 2\n",
        "[[hcell style =\"padding: 0 2px; width: 75%;\"]]CELL[[/hcell]]\n",
        "[[hcell style=\"padding: 0\u{00a0}2px; width: 75%;\"]]NBSP[[/hcell]]\n",
        "PROSE: 0 2px",
    )
    .to_owned();

    crate::preprocess_for_layout(&mut text, crate::layout::Layout::Wikidot);

    assert!(text.contains("NUMBER: 1\u{00a0}2"), "{text}");
    assert!(
        text.contains(r#"style ="padding: 0 2px; width: 75%;""#),
        "{text}",
    );
    assert!(
        text.contains("style=\"padding: 0\u{00a0}2px; width: 75%;\""),
        "{text}",
    );
    assert!(text.contains("PROSE: 0\u{00a0}2px"), "{text}");
}

#[test]
fn wikidot_unit_typography_keeps_local_link_label_separators() {
    let mut text = concat!(
        "[/scp-536-fr/offset/0 V]\n",
        "[/other/12 M]\n",
        "PROSE: 0 V",
    )
    .to_owned();

    substitute_wikidot(&mut text);

    assert!(text.contains("[/scp-536-fr/offset/0 V]"), "{text}");
    assert!(text.contains("[/other/12 M]"), "{text}");
    assert!(text.contains("PROSE: 0\u{00a0}V"), "{text}");
}

#[test]
fn wikidot_inline_escape_markers_do_not_pair_across_lines() {
    let mut text = concat!(
        "UNCLOSED:@@OPEN...\n",
        "CROSS:@@OPEN\n",
        "CLOSE:END...@@ AFTER...\n",
        "CLOSED:@@KEEP...@@ CHANGE...",
    )
    .to_owned();

    crate::preprocess_for_layout(&mut text, crate::layout::Layout::Wikidot);

    assert!(text.contains("UNCLOSED:@@OPEN…"), "{text}");
    assert!(text.contains("CROSS:@@OPEN"), "{text}");
    assert!(text.contains("CLOSE:END…@@ AFTER…"), "{text}");
    assert!(text.contains("CLOSED:@@KEEP...@@ CHANGE…"), "{text}");
}
