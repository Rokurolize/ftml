/*
 * parsing/token/test.rs
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

use super::*;
use crate::utf16::Utf16IndexMap;
use proptest::prelude::*;
use std::fs;
use std::path::Path;

#[test]
fn extracted_token_to_utf16_indices_converts_span() {
    let text = "a🦀bc";
    let map = Utf16IndexMap::new(text);
    let token = ExtractedToken {
        token: Token::Identifier,
        slice: "🦀b",
        span: 1..6,
    };

    let converted = token.to_utf16_indices(&map);

    assert_eq!(converted.token, Token::Identifier);
    assert_eq!(converted.slice, "🦀b");
    assert_eq!(converted.span, 1..4);
    assert_eq!(token.span, 1..6);
}

#[test]
fn token_names_match_variant_names() {
    assert_eq!(Token::LeftBracketStar.name(), "LeftBracketStar");
    assert_eq!(Token::ParagraphBreak.name(), "ParagraphBreak");
    assert_eq!(Token::InputEnd.name(), "InputEnd");
}

fn assert_fast_tokens_match_pest(input: &str) {
    let fast = Token::extract_all(input);
    let pest = Token::extract_all_pest(input);
    assert_eq!(
        fast, pest,
        "fast tokenizer differed from pest for {input:?}"
    );
}

#[test]
fn fast_tokenizer_matches_pest_on_adversarial_inputs() {
    for input in [
        "",
        "\n",
        "\r",
        "\r\n",
        "\n\n",
        "\r\n\r\n",
        "[[[[quadLinkTest]]]]",
        "[[[[[",
        "[[[[[[",
        "]]]]]",
        "]]]]]]",
        "[[[*user]]] [[[* user]]]",
        "[[include component:start]] {$title} [[/include]]",
        "abc@example.com foo%bar@example.com",
        "{{abc@example.com}}",
        "http://example.com/a|b https://example.com/[x]",
        "@<https://example.com/raw>@ https://example.com/a>b https://example.com/a>@b",
        "url(@<https://example.com/raw>@)",
        "ftp://example.com/path \"quoted\"",
        "+* heading\n+** not-starred\n+++++++ seven",
        "* item\n**bold**\n# item\n##color##",
        "~~~<\n~~~>\n~~~~\n~~\n~",
        "--- -- --] ---]",
        "@@ @< >@ [!-- comment --]",
        "\\\" \\\\ \\x",
        "雪 & 火",
        "abc@example.com",
        "foo%bar@example.com",
        "abc.def@example.com",
        "abc@example.com]",
        "abc@",
        "abc@example",
        "abc@.com",
        "abc@example.",
        "abc.def no-at",
    ] {
        assert_fast_tokens_match_pest(input);
    }
}

#[test]
fn fast_tokenizer_matches_pest_on_fixture_inputs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test");
    let mut inputs = Vec::new();
    collect_fixture_inputs(&root, &mut inputs);
    inputs.sort();

    for path in inputs {
        let input = fs::read_to_string(&path).expect("fixture input should be readable");
        let fast = Token::extract_all(&input);
        let pest = Token::extract_all_pest(&input);
        assert_eq!(
            fast,
            pest,
            "fast tokenizer differed from pest for fixture {}",
            path.display(),
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn fast_tokenizer_matches_pest_for_short_random_inputs(input in ".{0,128}") {
        assert_fast_tokens_match_pest(&input);
    }
}

fn collect_fixture_inputs(path: &Path, inputs: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(path).expect("fixture directory should be readable") {
        let entry = entry.expect("fixture directory entry should be readable");
        let path = entry.path();

        if path.is_dir() {
            collect_fixture_inputs(&path, inputs);
        } else if path.file_name().is_some_and(|name| name == "input.ftml") {
            inputs.push(path);
        }
    }
}

#[test]
fn lexer_error_tokens_return_input_as_other_token() {
    let tokens = lexer_error_tokens("bad input", "synthetic lexer error");

    assert_eq!(
        tokens,
        vec![ExtractedToken {
            token: Token::Other,
            slice: "bad input",
            span: 0..9,
        }],
    );
}

#[test]
fn token_extraction_falls_back_on_lexer_error() {
    let result: Result<Pairs<'_, Rule>, &str> = Err("synthetic lexer error");
    let tokens = Token::extract_tokens_from_pairs("bad input", result);

    assert_eq!(
        tokens,
        vec![ExtractedToken {
            token: Token::Other,
            slice: "bad input",
            span: 0..9,
        }],
    );
}

#[test]
#[should_panic(expected = "Received invalid pest rule")]
fn get_from_rule_rejects_document_rule() {
    Token::get_from_rule(Rule::document);
}

#[test]
#[should_panic(expected = "Received invalid pest rule")]
fn get_from_rule_rejects_token_rule() {
    Token::get_from_rule(Rule::token);
}

#[test]
fn tokens() {
    macro_rules! test {
        ($input:expr, $expected:expr $(,)?) => {{
            debug!("Testing tokens! Input: {}", $input);

            let expected: Vec<ExtractedToken> = $expected;
            let result = {
                let tokenization = crate::tokenize($input);
                let mut tokens: Vec<ExtractedToken> = tokenization.into();

                let first = tokens.remove(0);
                let last = tokens.pop().expect("No final element in resultant tokens");

                assert_eq!(first.token, Token::InputStart, "First token wasn't Token::InputStart");
                assert_eq!(first.slice, "", "First slice wasn't an empty string");

                assert_eq!(last.token, Token::InputEnd, "Final token wasn't Token::InputEnd");
                assert_eq!(last.slice, "", "Final slice wasn't an empty string");

                tokens
            };

            // Manually implement "assert_eq!" here so we can use full, {:#?} formatting

            if result != expected {
                panic!(
                    "Extracted tokens from lexer do not match expected!\n\nExpected: {:#?}\nActual: {:#?}",
                    result,
                    expected,
                );
            }
        }};
    }

    // Test cases:

    test!("", vec![]);

    test!(
        "text",
        vec![ExtractedToken {
            token: Token::Identifier,
            slice: "text",
            span: 0..4,
        }],
    );

    test!(
        "-- doubleDash",
        vec![
            ExtractedToken {
                token: Token::DoubleDash,
                slice: "--",
                span: 0..2,
            },
            ExtractedToken {
                token: Token::Whitespace,
                slice: " ",
                span: 2..3,
            },
            ExtractedToken {
                token: Token::Identifier,
                slice: "doubleDash",
                span: 3..13,
            },
        ],
    );

    test!(
        "--doubleDash",
        vec![
            ExtractedToken {
                token: Token::DoubleDash,
                slice: "--",
                span: 0..2,
            },
            ExtractedToken {
                token: Token::Identifier,
                slice: "doubleDash",
                span: 2..12,
            },
        ],
    );

    test!(
        "__[[*user }}",
        vec![
            ExtractedToken {
                token: Token::Underline,
                slice: "__",
                span: 0..2,
            },
            ExtractedToken {
                token: Token::LeftBlockStar,
                slice: "[[*",
                span: 2..5,
            },
            ExtractedToken {
                token: Token::Identifier,
                slice: "user",
                span: 5..9,
            },
            ExtractedToken {
                token: Token::Whitespace,
                slice: " ",
                span: 9..10,
            },
            ExtractedToken {
                token: Token::RightMonospace,
                slice: "}}",
                span: 10..12,
            },
        ],
    );

    test!(
        r#"[[> unsure = "malformed \string"#,
        vec![
            ExtractedToken {
                token: Token::LeftBlock,
                slice: "[[",
                span: 0..2,
            },
            ExtractedToken {
                token: Token::Quote,
                slice: ">",
                span: 2..3,
            },
            ExtractedToken {
                token: Token::Whitespace,
                slice: " ",
                span: 3..4,
            },
            ExtractedToken {
                token: Token::Identifier,
                slice: "unsure",
                span: 4..10,
            },
            ExtractedToken {
                token: Token::Whitespace,
                slice: " ",
                span: 10..11,
            },
            ExtractedToken {
                token: Token::Equals,
                slice: "=",
                span: 11..12,
            },
            ExtractedToken {
                token: Token::Whitespace,
                slice: " ",
                span: 12..13,
            },
            ExtractedToken {
                token: Token::DoubleQuote,
                slice: "\"",
                span: 13..14,
            },
            ExtractedToken {
                token: Token::Identifier,
                slice: "malformed",
                span: 14..23,
            },
            ExtractedToken {
                token: Token::Whitespace,
                slice: " ",
                span: 23..24,
            },
            ExtractedToken {
                token: Token::Other,
                slice: "\\",
                span: 24..25,
            },
            ExtractedToken {
                token: Token::Identifier,
                slice: "string",
                span: 25..31,
            },
        ],
    );

    test!(
        r#"\""#,
        vec![ExtractedToken {
            token: Token::EscapedDoubleQuote,
            slice: r#"\""#,
            span: 0..2,
        }],
    );

    test!(
        "[[[[quadLinkTest]]]]",
        vec![
            ExtractedToken {
                token: Token::LeftBracket,
                slice: "[",
                span: 0..1,
            },
            ExtractedToken {
                token: Token::LeftLink,
                slice: "[[[",
                span: 1..4,
            },
            ExtractedToken {
                token: Token::Identifier,
                slice: "quadLinkTest",
                span: 4..16,
            },
            ExtractedToken {
                token: Token::RightLink,
                slice: "]]]",
                span: 16..19,
            },
            ExtractedToken {
                token: Token::RightBracket,
                slice: "]",
                span: 19..20,
            },
        ],
    );

    test!(
        "abc@example.com foo%bar@example.com",
        vec![
            ExtractedToken {
                token: Token::Email,
                slice: "abc@example.com",
                span: 0..15,
            },
            ExtractedToken {
                token: Token::Whitespace,
                slice: " ",
                span: 15..16,
            },
            ExtractedToken {
                token: Token::Email,
                slice: "foo%bar@example.com",
                span: 16..35,
            },
        ],
    );

    test!(
        "{{abc@example.com}}",
        vec![
            ExtractedToken {
                token: Token::LeftMonospace,
                slice: "{{",
                span: 0..2,
            },
            ExtractedToken {
                token: Token::Email,
                slice: "abc@example.com",
                span: 2..17,
            },
            ExtractedToken {
                token: Token::RightMonospace,
                slice: "}}",
                span: 17..19,
            },
        ],
    );

    test!(
        "@<https://example.com/raw>@",
        vec![
            ExtractedToken {
                token: Token::LeftRaw,
                slice: "@<",
                span: 0..2,
            },
            ExtractedToken {
                token: Token::Url,
                slice: "https://example.com/raw",
                span: 2..25,
            },
            ExtractedToken {
                token: Token::RightRaw,
                slice: ">@",
                span: 25..27,
            },
        ],
    );

    test!(
        "https://example.com/a>b",
        vec![ExtractedToken {
            token: Token::Url,
            slice: "https://example.com/a>b",
            span: 0..23,
        }],
    );

    test!(
        "https://example.com/a>@b",
        vec![
            ExtractedToken {
                token: Token::Url,
                slice: "https://example.com/a",
                span: 0..21,
            },
            ExtractedToken {
                token: Token::RightRaw,
                slice: ">@",
                span: 21..23,
            },
            ExtractedToken {
                token: Token::Identifier,
                slice: "b",
                span: 23..24,
            },
        ],
    );

    test!(
        "url(@<https://example.com/raw>@)",
        vec![
            ExtractedToken {
                token: Token::Identifier,
                slice: "url",
                span: 0..3,
            },
            ExtractedToken {
                token: Token::Other,
                slice: "(",
                span: 3..4,
            },
            ExtractedToken {
                token: Token::LeftRaw,
                slice: "@<",
                span: 4..6,
            },
            ExtractedToken {
                token: Token::Url,
                slice: "https://example.com/raw",
                span: 6..29,
            },
            ExtractedToken {
                token: Token::RightRaw,
                slice: ">@",
                span: 29..31,
            },
            ExtractedToken {
                token: Token::Other,
                slice: ")",
                span: 31..32,
            },
        ],
    );
}
