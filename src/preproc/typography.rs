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

fn replace_number_spaces(text: &mut String) {
    let chars = text.char_indices().collect::<Vec<_>>();
    let mut buffer = String::with_capacity(text.len());
    let mut last_copied = 0;

    for window in chars.windows(3) {
        let [(_, left), (space_index, ' '), (_, right)] = window else {
            continue;
        };
        if !left.is_ascii_digit() || !right.is_ascii_digit() {
            continue;
        }

        buffer.push_str(&text[last_copied..*space_index]);
        buffer.push('\u{a0}');
        last_copied = space_index + 1;
    }

    if last_copied > 0 {
        buffer.push_str(&text[last_copied..]);
        *text = buffer;
    }
}

fn replace_unit_spaces(text: &mut String) {
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

pub(super) fn substitute_wikidot(text: &mut String) {
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

    replace_number_spaces(text);
    replace_unit_spaces(text);

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
fn wikijump_substitution_keeps_native_typography_scope() {
    let mut text = "`one\nline' << 1 234".to_owned();
    substitute(&mut text);
    assert_eq!(text, "`one\nline' << 1 234");
}

#[test]
fn wikidot_preprocessing_keeps_literal_dot_runs_outside_authored_prose() {
    let mut text = concat!(
        "PROSE:x....x\n",
        "[[code]]\n",
        "CODE:x....x\n",
        "[[/code]]\n",
        "@@ESCAPED:x....x@@",
    )
    .to_owned();

    crate::preprocess_for_layout(&mut text, crate::layout::Layout::Wikidot);

    assert!(text.contains("PROSE:x….x"), "{text}");
    assert!(text.contains("CODE:x....x"), "{text}");
    assert!(text.contains("ESCAPED:x....x"), "{text}");
}
