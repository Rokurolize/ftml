/*
 * parsing/rule/impls/monospace.rs
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

use super::prelude::*;
use crate::tree::Container;

pub const RULE_MONOSPACE: Rule = Rule {
    name: "monospace",
    position: LineRequirement::Any,
    try_consume_fn,
};

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to create monospace container");
    assert_step(parser, Token::LeftMonospace)?;

    if is_ascii_space_padding(parser.current()) {
        let is_padding_only = parser.next_two_tokens().1 == Some(Token::RightMonospace);
        assert_step(parser, Token::Whitespace)?;

        if is_padding_only {
            assert_step(parser, Token::RightMonospace)?;
            return success_elements(Elements::None);
        }
    }

    let close = [
        ParseCondition::current(Token::RightMonospace),
        ParseCondition::token_pair(Token::Whitespace, Token::RightMonospace),
    ];
    let invalid = [
        ParseCondition::current(Token::ParagraphBreak),
        // Preserve the established fail-closed behavior for padded nested markers.
        ParseCondition::token_pair(Token::LeftMonospace, Token::Whitespace),
    ];
    let collected = collect_consume_keep(parser, RULE_MONOSPACE, &close, &invalid, None)?;
    let ((elements, terminator), errors, paragraph_safe) = collected.into();

    // The configured close conditions guarantee either a fully consumed direct
    // marker or a whitespace token whose following marker remains current.
    if terminator.token == Token::Whitespace {
        if !is_ascii_space_padding(terminator) {
            return Err(parser.make_err(ParseErrorKind::RuleFailed));
        }
        assert_step(parser, Token::RightMonospace)?;
    }

    let element = Element::Container(Container::new(
        ContainerType::Monospace,
        elements,
        AttributeMap::new(),
    ));
    ok!(paragraph_safe; element, errors)
}

fn is_ascii_space_padding(token: &ExtractedToken<'_>) -> bool {
    token.token == Token::Whitespace
        && !token.slice.is_empty()
        && token.slice.bytes().all(|byte| byte == b' ')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::parsing::token::ExtractedToken;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};
    use std::time::{Duration, Instant};

    fn render(input: &str) -> (String, Vec<ParseError>) {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        (html, errors)
    }

    #[test]
    fn monospace_trims_evidenced_ascii_space_padding() {
        for input in ["{{text}}", "{{ text}}", "{{text }}", "{{ text }}"] {
            let (html, errors) = render(input);
            assert!(errors.is_empty(), "{input:?}: {errors:?}");
            assert!(html.contains("<tt>text</tt>"), "{input:?}: {html}",);
        }
    }

    #[test]
    fn scuttle_padded_monospace_matches_the_public_wikidot_text() {
        let input = "**Affected Sites:** {{ Output Error: List object exceeds 10,000 characters. }}";
        let (html, errors) = render(input);

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(
            html,
            concat!(
                "<p><strong>Affected Sites:</strong> ",
                "<tt>",
                "Output Error: List object exceeds 10,000 characters.",
                "</tt></p>",
            ),
        );
    }

    #[test]
    fn monospace_trims_space_runs_but_preserves_internal_space_and_markup() {
        let (html, errors) = render("before {{   a  **b**  c   }} after");
        assert!(errors.is_empty(), "{errors:?}");
        assert!(html.contains("before <tt>a  <strong>b</strong>  c</tt> after",));
    }

    #[test]
    fn monospace_space_only_body_produces_no_inline_container() {
        let (html, errors) = render("before{{   }}after");
        assert!(errors.is_empty(), "{errors:?}");
        assert!(html.contains("beforeafter"), "{html}");
        assert!(!html.contains("<tt>"), "{html}");
    }

    #[test]
    fn monospace_close_consumes_exactly_its_own_terminator() {
        for input in ["{{x}}}}tail", "{{x }}}}tail"] {
            let page_info = PageInfo::dummy();
            let settings =
                WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
            let tokenization = crate::tokenize(input);
            let mut parser = Parser::new(&tokenization, &page_info, &settings);
            parser
                .step()
                .expect("opening marker should follow input start");
            parser.set_rule(RULE_MONOSPACE);

            let _parsed = try_consume_fn(&mut parser).expect("monospace should parse");

            assert_eq!(parser.current().token, Token::RightMonospace, "{input:?}");
            assert_eq!(parser.current().slice, "}}", "{input:?}");
        }

        let (html, errors) = render("{{x }}{{ y}}");
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(html.matches("<tt>").count(), 2, "{html}");
    }

    #[test]
    fn monospace_padding_failure_rolls_back_without_losing_source() {
        for input in ["{{ x", "{{x ", "prefix {{ x", "{{ x }", "{{ x\n\n y }}"] {
            let (html, _errors) = render(input);
            assert!(!html.contains("<tt>"), "{input:?}: {html}");
            assert!(html.contains("{{"), "{input:?}: {html}");
        }
    }

    #[test]
    fn monospace_keeps_padded_nested_marker_fail_closed() {
        let input = "{{outer {{ inner }} tail}}";
        let (html, _errors) = render(input);
        assert!(html.starts_with("<p>{{outer "), "{html}");
        assert!(html.contains("<tt>inner</tt> tail}}</p>"), "{html}",);
    }

    #[test]
    fn monospace_does_not_trim_unverified_tabs() {
        let (leading_html, leading_errors) = render("{{\ttext}}");
        assert!(leading_errors.is_empty(), "{leading_errors:?}");
        assert!(leading_html.contains("<tt>\ttext</tt>"), "{leading_html}",);

        let (trailing_html, _trailing_errors) = render("{{text\t}}");
        assert!(!trailing_html.contains("<tt>"), "{trailing_html}");
        assert!(trailing_html.contains("{{text\t}}"), "{trailing_html}");
    }

    #[test]
    fn repeated_quoted_monospace_emails_stay_bounded() {
        // Reduced from EN vivid-visions, whose message transcript repeats this shape.
        // Wikidot renders each address inside tt > span.wiki-email, so the email
        // token must stop before the monospace closer rather than consume it.
        let mut input = String::new();
        for _ in 0..32 {
            input.push_str("> FROM: <{{person@scp.foundation}}>\n> \n");
        }

        let started = Instant::now();
        let (html, errors) = render(&input);
        let elapsed = started.elapsed();

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html.matches("<tt>").count(), 32, "{html}");
        assert!(
            elapsed < Duration::from_millis(500),
            "repeated monospace email parse took {elapsed:?}",
        );
    }

    #[test]
    fn ascii_space_padding_predicate_is_narrow() {
        let spaces = ExtractedToken {
            token: Token::Whitespace,
            slice: "   ",
            span: 0..3,
        };
        let tab = ExtractedToken {
            token: Token::Whitespace,
            slice: "\t",
            span: 0..1,
        };
        let other = ExtractedToken {
            token: Token::Identifier,
            slice: " ",
            span: 0..1,
        };

        assert!(is_ascii_space_padding(&spaces));
        assert!(!is_ascii_space_padding(&tab));
        assert!(!is_ascii_space_padding(&other));
    }
}
