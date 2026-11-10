use proptest::prelude::*;

const LANGUAGES: &[&str] = &[
    "cpp",
    "css",
    "diff",
    "dtd",
    "html",
    "java",
    "javascript",
    "perl",
    "php",
    "python",
    "ruby",
    "sql",
    "xml",
];

fn assert_token_bounds(language: &str, body: &str) {
    let compiled = compiled(language).expect(language);
    let table = table_for(language).expect(language);
    let text = preprocess(body, compiled.tabsize);
    let tokens = tokenize(compiled, table, &text);
    let mut cursor = 0usize;
    for token in &tokens {
        assert!(
            text.is_char_boundary(token.start) && text.is_char_boundary(token.end),
            "{language} token {}..{} is not a char boundary of {text:?}",
            token.start,
            token.end
        );
        assert!(token.start <= token.end && token.end <= text.len());
        assert_eq!(
            token.start, cursor,
            "{language} token does not continue the source"
        );
        if token.end > token.start {
            cursor = token.end;
        }
    }
    assert_eq!(
        cursor,
        text.len(),
        "{language} tokens do not cover {text:?}"
    );
}

#[test]
fn highlight_tokens_cover_char_boundaries_on_fixed_inputs() {
    let samples = [
        "",
        "a",
        "aあb",
        "fn main() {\n    let x = 1;\n}",
        "/*",
        "\"",
        ":",
    ];
    for language in LANGUAGES {
        for sample in samples {
            assert_token_bounds(language, sample);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 16, ..ProptestConfig::default() })]

    #[test]
    fn highlight_tokens_cover_generated_inputs(
        language in prop::sample::select(LANGUAGES),
        body in "\\PC{0,24}"
    ) {
        assert_token_bounds(language, &body);
    }
}
