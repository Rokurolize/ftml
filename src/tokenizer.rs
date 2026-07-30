/*
 * tokenizer.rs
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

use crate::delayed::{GeneratedInput, GeneratedKind, InputSegment};
use crate::parsing::{ExtractedToken, Token};
use crate::text::FullText;
use std::collections::BTreeMap;

/// Struct that represents both a list of tokens and the text the tokens were generated from.
#[derive(Debug, Clone)]
pub struct Tokenization<'t> {
    tokens: Vec<ExtractedToken<'t>>,
    full_text: FullText<'t>,
    generated: BTreeMap<usize, GeneratedInput>,
}

#[cfg(not(tarpaulin))]
impl<'t> Tokenization<'t> {
    #[inline]
    pub fn tokens<'r>(&'r self) -> &'r [ExtractedToken<'t>] {
        &self.tokens
    }

    #[inline]
    pub(crate) fn full_text(&self) -> FullText<'t> {
        self.full_text
    }

    #[inline]
    pub(crate) fn generated(&self) -> &BTreeMap<usize, GeneratedInput> {
        &self.generated
    }
}

// Tarpaulin maps the generic impl header as executable unless the first method
// starts on the same line.
#[cfg(tarpaulin)]
#[rustfmt::skip]
impl<'t> Tokenization<'t> { pub fn tokens<'r>(&'r self) -> &'r [ExtractedToken<'t>] { &self.tokens }
    pub(crate) fn full_text(&self) -> FullText<'t> { self.full_text }
    pub(crate) fn generated(&self) -> &BTreeMap<usize, GeneratedInput> { &self.generated }
}

impl<'t> From<Tokenization<'t>> for Vec<ExtractedToken<'t>> {
    #[inline]
    fn from(tokenization: Tokenization<'t>) -> Vec<ExtractedToken<'t>> {
        tokenization.tokens
    }
}

/// Take an input string and produce a list of tokens for consumption by the parser.
#[track_caller]
pub fn tokenize(text: &str) -> Tokenization<'_> {
    #[cfg(feature = "test-source-recorder")]
    crate::source_recorder::record("tokenize", text, std::panic::Location::caller());

    debug!(
        "Running lexer on text ({} bytes) to produce tokens",
        text.len(),
    );

    let tokens = Token::extract_all(text);
    let full_text = FullText::new(text);

    Tokenization {
        tokens,
        full_text,
        generated: BTreeMap::new(),
    }
}

pub(crate) fn tokenize_delayed_segments<'t>(
    text: &'t str,
    segments: &[InputSegment],
) -> Tokenization<'t> {
    let mut tokens = vec![ExtractedToken {
        token: Token::InputStart,
        slice: &text[..0],
        span: 0..0,
    }];

    let mut generated_slots = BTreeMap::new();
    for segment in segments {
        match segment {
            InputSegment::Text { source_range, .. } => {
                let segment_text = &text[source_range.clone()];
                tokens.extend(
                    Token::extract_all(segment_text)
                        .into_iter()
                        .filter(|token| {
                            !matches!(token.token, Token::InputStart | Token::InputEnd)
                        })
                        .map(|mut token| {
                            token.span.start += source_range.start;
                            token.span.end += source_range.start;
                            token
                        }),
                );
            }
            InputSegment::Generated(generated) => {
                let start = generated.source_range.start;
                generated_slots.insert(start, generated.clone());
                tokens.push(ExtractedToken {
                    token: match generated.kind {
                        GeneratedKind::PageLink => Token::GeneratedPageLink,
                        GeneratedKind::TagLinks => Token::GeneratedTagLinks,
                    },
                    slice: &text[start..start],
                    span: start..start,
                });
            }
        }
    }
    tokens.push(ExtractedToken {
        token: Token::InputEnd,
        slice: &text[text.len()..],
        span: text.len()..text.len(),
    });
    Tokenization {
        tokens,
        full_text: FullText::new(text),
        generated: generated_slots,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::prelude::*;
    use std::sync::mpsc;
    use std::time::Duration;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(4096))]

        #[test]
        #[ignore = "slow test"]
        fn tokenizer_prop(s in ".*") {
            let _ = tokenize(&s);
        }
    }

    #[test]
    fn tokenizer_handles_long_punctuation_runs_without_email_scan_blowup() {
        let input = "%".repeat(16_384);
        let expected_tokens = input.len() + 2;
        let (sender, receiver) = mpsc::channel();

        std::thread::spawn(move || {
            let tokenization = tokenize(&input);
            let _ = sender.send(tokenization.tokens().len());
        });

        let token_count = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("tokenizing a punctuation-only run should stay bounded");

        assert_eq!(token_count, expected_tokens);
    }

    #[test]
    fn email_scan_does_not_swallow_a_wikidot_block_closer() {
        // Corpus provenance: scp-wiki/scp-9945.
        let input = "[[span]]name[[/span]]19@scip.net";
        let tokenization = tokenize(input);

        assert!(
            tokenization
                .tokens()
                .iter()
                .any(|token| token.token == Token::Email && token.slice == "19@scip.net"),
            "{:#?}",
            tokenization.tokens(),
        );
        assert!(
            tokenization
                .tokens()
                .iter()
                .all(|token| token.slice != "span]]19@scip.net"),
            "{:#?}",
            tokenization.tokens(),
        );
    }

    #[test]
    fn discarded_control_is_an_invisible_email_barrier() {
        let input = "name@\u{0006}site.com";
        let tokenization = tokenize(input);

        assert!(
            tokenization
                .tokens()
                .iter()
                .any(|token| token.token == Token::DiscardedControl),
            "{:#?}",
            tokenization.tokens(),
        );
        assert!(
            tokenization
                .tokens()
                .iter()
                .all(|token| token.token != Token::Email),
            "{:#?}",
            tokenization.tokens(),
        );
    }
}
