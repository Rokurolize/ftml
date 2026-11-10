use proptest::prelude::*;

const TAGS: &[&str] = &[
    "math",
    "mi",
    "mn",
    "mo",
    "mfrac",
    "mroot",
    "mrow",
    "msqrt",
    "mspace",
    "mstyle",
    "msub",
    "msubsup",
    "msup",
    "mtable",
    "mtd",
    "mtext",
    "mtr",
    "munder",
    "munderover",
    "mover",
];

const ATTRS: &[&str] = &[
    "accent",
    "columnalign",
    "display",
    "displaystyle",
    "form",
    "linethickness",
    "mathvariant",
    "stretchy",
    "width",
    "xmlns",
];

fn assert_emitted_mathml(output: &str) {
    let mut stack: Vec<&str> = Vec::new();
    let mut rest = output;
    while let Some(start) = rest.find('<') {
        let tag = &rest[start + 1..];
        let end = tag.find('>').expect("truncated tag");
        let raw = &tag[..end];
        let body = raw.strip_suffix('/').unwrap_or(raw).trim();
        let close = body.starts_with('/');
        let name_src = if close { &body[1..] } else { body };
        let name = name_src.split_whitespace().next().unwrap_or("").trim();
        assert!(TAGS.contains(&name), "emitted tag {name} from {output}");
        if !close {
            for piece in name_src.split_whitespace().skip(1) {
                let key = piece.split('=').next().unwrap_or("");
                assert!(
                    ATTRS.contains(&key),
                    "emitted attribute {key} from {output}"
                );
            }
        }
        if close {
            assert_eq!(
                stack.pop(),
                Some(name),
                "close </{name}> does not match the open stack in {output}"
            );
        } else if !raw.ends_with('/') {
            stack.push(name);
        }
        rest = &tag[end + 1..];
    }
}

#[test]
fn mathml_mismatched_close_stays_escaped() {
    let output = sanitize_mathml("<mrow><mi>a</mi></mfrac>");
    assert!(output.contains("<mrow>"), "{output}");
    assert!(output.contains("&lt;/mfrac&gt;"), "{output}");
    assert!(!output.contains("</mfrac>"), "{output}");
    assert_emitted_mathml(&output);
}

#[test]
fn mathml_balanced_input_ends_balanced() {
    let output =
        sanitize_mathml("<math><mrow><msup><mi>x</mi><mn>2</mn></msup></mrow></math>");
    assert_emitted_mathml(&output);
    assert!(output.contains("</math>"), "{output}");
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    #[test]
    fn mathml_property_emits_only_allowlisted_tags(ops in prop::collection::vec(0u8..8, 0..10)) {
        let mut input = String::new();
        let mut opens: Vec<&'static str> = Vec::new();
        for op in ops {
            match op {
                0 | 1 => {
                    let name = TAGS[op as usize % 5];
                    input.push('<');
                    input.push_str(name);
                    input.push('>');
                    opens.push(name);
                }
                2 | 3 => {
                    if let Some(name) = opens.pop() {
                        input.push_str("</");
                        input.push_str(name);
                        input.push('>');
                    }
                }
                4 => input.push_str("<mi>a<b</mi>"),
                5 => input.push_str("<script>alert(1)</script>"),
                6 => input.push_str("<mrow onclick=\"alert(1)\">"),
                _ => input.push_str("</mfrac><math xmlns=\"http://www.w3.org/1998/Math/MathML\">"),
            }
        }
        let output = sanitize_mathml(&input);
        assert_emitted_mathml(&output);
        prop_assert!(!output.contains("<script"));
    }
}
