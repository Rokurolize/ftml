/*
 * parsing/rule/impls/block/blocks/note.rs
 *
 * ftml - Library to parse Wikidot text
 * Copyright (C) 2019-2026 Wikijump Team
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 */

use super::prelude::*;
use crate::parsing::rule::impls::block::BlockBodyStart;
use crate::tree::AttributeMap;

pub const BLOCK_NOTE: BlockRule = BlockRule {
    name: "block-note",
    accepts_names: &["note"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: true,
    parse_fn,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Parsing note block (in-head {in_head})");
    assert!(!flag_star, "Note doesn't allow star flag");
    assert!(!flag_score, "Note doesn't allow score flag");
    assert_block_name(&BLOCK_NOTE, name);

    if !parser.settings().layout.legacy() {
        let (arguments, body_start) =
            parser.get_head_map_with_body_start(&BLOCK_NOTE, in_head)?;
        if matches!(body_start, BlockBodyStart::Inline) {
            return Err(parser.make_err(ParseErrorKind::NotSupportedInline));
        }
        let (elements, errors, paragraph_safe) =
            parser.get_body_elements(&BLOCK_NOTE, false)?.into();
        let element = Element::Container(Container::new(
            ContainerType::Note,
            elements,
            arguments.to_attribute_map(parser.settings()),
        ));
        return ok!(paragraph_safe; element, errors);
    }

    let wikidot_note =
        parser.settings().layout.legacy() && !parser.discarding_hidden_body();
    if wikidot_note && parser.in_wikidot_note_body() {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let body_start = parser.get_head_none_with_body_start(&BLOCK_NOTE, in_head)?;
    if wikidot_note && !parser.has_body_end_block(&BLOCK_NOTE) {
        return Err(parser.make_err(ParseErrorKind::BlockExpectedEnd));
    }
    if wikidot_note {
        parser.enter_wikidot_note_body();
    }
    let body = parser.get_body_elements_with_context(&BLOCK_NOTE, true, body_start);
    if wikidot_note {
        parser.leave_wikidot_note_body();
    }
    let (elements, errors, _) = body?.into();

    let mut attributes = AttributeMap::new();
    assert!(attributes.insert("class", cow!("wiki-note")));
    let element =
        Element::Container(Container::new(ContainerType::Div, elements, attributes));

    ok!(element, errors)
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};
    use std::time::{Duration, Instant};

    fn render_wikidot_with_html(
        source: &str,
        enable_html_blocks: bool,
    ) -> (String, Vec<crate::parsing::ParseError>) {
        let page_info = PageInfo::dummy();
        let mut settings =
            WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        settings.enable_html_blocks = enable_html_blocks;
        let mut source = source.to_owned();
        crate::preprocess_for_layout(&mut source, settings.layout);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        (html, errors)
    }

    fn render_wikidot(source: &str) -> (String, Vec<crate::parsing::ParseError>) {
        render_wikidot_with_html(source, true)
    }

    fn render_wikijump(source: &str) -> (String, Vec<crate::parsing::ParseError>) {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        (html, errors)
    }

    #[test]
    fn note_block_renders_wikidot_note_dom_with_paragraph_body() {
        let (html, errors) = render_wikidot("[[note]]\nEvidence-backed note.[[/note]]");

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            r#"<div class="wiki-note"><p>Evidence-backed note.</p></div>"#
        );
    }

    #[test]
    fn wikijump_note_uses_native_dom_and_preserves_attributes() {
        let (html, errors) = render_wikijump(
            "[[note class=\"custom\" data-kind=\"example\"]]\nBody\n[[/note]]",
        );

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            r#"<p><div class="wj-note custom" data-kind="example">Body</div></p>"#,
        );
    }

    #[test]
    fn wikijump_note_rejects_inline_openers() {
        let (html, errors) = render_wikijump("prefix [[note]]Body[[/note]]");

        assert!(
            errors.iter().any(|error| error.kind()
                == crate::parsing::ParseErrorKind::NotSupportedInline),
            "html={html}; errors={errors:#?}",
        );
        assert!(!html.contains("class=\"wj-note"), "{html}");
    }

    #[test]
    fn nested_note_opener_is_literal_and_first_closer_ends_outer_note() {
        let source = "[[note]]\nA\n[[note]]\nB\n[[/note]]\nC\n[[/note]]";
        let (html, _errors) = render_wikidot(source);

        assert_eq!(
            html,
            concat!(
                r#"<div class="wiki-note"><p>A<br>"#,
                "\n[[note]]<br>\nB</p></div><br>\nC<br>\n[[/note]]",
            ),
        );
        assert_eq!(html.matches(r#"class="wiki-note""#).count(), 1, "{html}");
    }

    #[test]
    fn preview_html_child_remains_literal_inside_note() {
        let source = "[[note]]\n[[html]]\nX\n[[/html]]\n[[/note]]";
        let (html, _errors) = render_wikidot_with_html(source, false);

        assert_eq!(
            html,
            concat!(
                r#"<div class="wiki-note"><p>[[html]]<br>"#,
                "\nX<br>\n[[/html]]</p></div>",
            ),
        );
        assert!(!html.contains("<iframe"), "{html}");
    }

    #[test]
    fn saved_page_html_child_remains_active_inside_note() {
        let source = "[[note]]\n[[html]]\nX\n[[/html]]\n[[/note]]";
        let (html, errors) = render_wikidot_with_html(source, true);

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            concat!(
                r#"<div class="wiki-note"><p><iframe src="https://example.com/" "#,
                r#"allowtransparency="true" frameborder="0" "#,
                r#"class="html-block-iframe"></iframe></p></div>"#,
            ),
        );
    }

    #[test]
    fn escaped_prefix_remains_a_text_node_before_the_note() {
        let source = "\\[[note]]\nBODY\n[[/note]]";
        let (html, _errors) = render_wikidot(source);

        assert_eq!(html, r#"\<div class="wiki-note"><p>BODY</p></div>"#,);
        assert!(!html.contains("<p>\\</p>"), "{html}");
    }

    #[test]
    fn repeated_unclosed_note_openers_recover_in_bounded_time() {
        let source = "[[note]]\n".repeat(4_096);
        let started = Instant::now();
        let (html, _errors) = render_wikidot(&source);
        let elapsed = started.elapsed();

        assert_eq!(html.matches("[[note]]").count(), 4_096, "{html}");
        assert!(!html.contains(r#"class="wiki-note""#), "{html}");
        assert!(
            elapsed < Duration::from_secs(5),
            "unclosed note recovery took {elapsed:?}",
        );
    }

    #[test]
    fn note_child_constructs_keep_their_live_owner_semantics() {
        for (name, source, expected) in [
            (
                "code",
                "[[note]]\n[[code]]\nX\n[[/code]]\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note">"#,
                    r#"<div class="code"><pre><code>X</code></pre></div>"#,
                    "</div>",
                ),
            ),
            (
                "parser function",
                "[[note]]\n[[#if 1 | A | B ]]\n[[/note]]",
                r#"<div class="wiki-note"><p>A</p></div>"#,
            ),
            (
                "table",
                "[[note]]\n|| A || B ||\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><table class="wiki-content-table">"#,
                    "\n<tr>\n<td>A</td>\n<td>B</td>\n</tr>\n</table></div>",
                ),
            ),
            (
                "list",
                "[[note]]\n* A\n* B\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><ul>"#,
                    "\n<li>A</li>\n<li>B</li>\n</ul></div>",
                ),
            ),
            (
                "heading",
                "[[note]]\n+ H\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><h1 id="toc0">"#,
                    "<span>H</span></h1></div>",
                ),
            ),
            (
                "link",
                "[[note]]\n[https://example.com X]\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><p>"#,
                    r#"<a href="https://example.com">X</a></p></div>"#,
                ),
            ),
            (
                "raw span",
                "[[note]]\n@@X@@\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><p>"#,
                    r#"<span style="white-space: pre-wrap;">X</span></p></div>"#,
                ),
            ),
            (
                "comment",
                "[[note]]\n[!--hidden--]X\n[[/note]]",
                r#"<div class="wiki-note"><p>X</p></div>"#,
            ),
        ] {
            let (html, errors) = render_wikidot(source);

            assert!(errors.is_empty(), "{name}: {errors:#?}");
            assert_eq!(html, expected, "{name}");
        }
    }

    #[test]
    fn note_positive_controls_keep_their_exact_dom() {
        for (name, source, expected) in [
            (
                "simple",
                "[[note]]\nBODY\n[[/note]]",
                r#"<div class="wiki-note"><p>BODY</p></div>"#,
            ),
            (
                "adjacent",
                "[[note]]\nA\n[[/note]]\n[[note]]\nB\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><p>A</p></div>"#,
                    r#"<div class="wiki-note"><p>B</p></div>"#,
                ),
            ),
            (
                "empty",
                "[[note]]\n\n[[/note]]",
                r#"<div class="wiki-note"></div>"#,
            ),
            (
                "blank",
                "[[note]]\n\n\n[[/note]]",
                r#"<div class="wiki-note"></div>"#,
            ),
            (
                "paragraphs",
                "[[note]]\nA\n\nB\n[[/note]]",
                r#"<div class="wiki-note"><p>A</p><p>B</p></div>"#,
            ),
            (
                "quote",
                "[[note]]\n> A\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><blockquote>"#,
                    "<p>A</p></blockquote></div>",
                ),
            ),
            (
                "title case",
                "[[Note]]\nBODY\n[[/Note]]",
                r#"<div class="wiki-note"><p>BODY</p></div>"#,
            ),
            (
                "uppercase",
                "[[NOTE]]\nBODY\n[[/NOTE]]",
                r#"<div class="wiki-note"><p>BODY</p></div>"#,
            ),
        ] {
            let (html, errors) = render_wikidot(source);

            assert!(errors.is_empty(), "{name}: {errors:#?}");
            assert_eq!(html, expected, "{name}");
        }
    }

    #[test]
    fn malformed_note_owners_roll_back_without_hiding_source() {
        for (name, source, expected) in [
            ("unclosed", "[[note]]\nBODY", "<p>[[note]]<br>\nBODY</p>"),
            (
                "wrong closer",
                "[[note]]\nBODY\n[[/div]]",
                "<p>[[note]]<br>\nBODY<br>\n[[/div]]</p>",
            ),
            (
                "arguments",
                "[[note class=\"x\"]]\nBODY\n[[/note]]",
                concat!(
                    "<p>[[note class=&quot;x&quot;]]<br>\n",
                    "BODY<br>\n[[/note]]</p>",
                ),
            ),
            (
                "empty argument",
                "[[note class=\"\"]]\nBODY\n[[/note]]",
                concat!(
                    "<p>[[note class=&quot;&quot;]]<br>\n",
                    "BODY<br>\n[[/note]]</p>",
                ),
            ),
            (
                "crossed quote",
                "[[note class=\"a id=\"b\"]]\nBODY\n[[/note]]",
                concat!(
                    "<p>[[note class=&quot;a id=&quot;b&quot;]]<br>\n",
                    "BODY<br>\n[[/note]]</p>",
                ),
            ),
            (
                "duplicate argument",
                "[[note class=\"a\" class=\"b\"]]\nBODY\n[[/note]]",
                concat!(
                    "<p>[[note class=&quot;a&quot; class=&quot;b&quot;]]<br>\n",
                    "BODY<br>\n[[/note]]</p>",
                ),
            ),
            (
                "event argument",
                "[[note onclick=\"alert(1)\"]]\nBODY\n[[/note]]",
                concat!(
                    "<p>[[note onclick=&quot;alert(1)&quot;]]<br>\n",
                    "BODY<br>\n[[/note]]</p>",
                ),
            ),
            (
                "style argument",
                "[[note style=\"color:red\"]]\nBODY\n[[/note]]",
                concat!(
                    "<p>[[note style=&quot;color:red&quot;]]<br>\n",
                    "BODY<br>\n[[/note]]</p>",
                ),
            ),
            (
                "unknown argument",
                "[[note unknown=\"x\"]]\nBODY\n[[/note]]",
                concat!(
                    "<p>[[note unknown=&quot;x&quot;]]<br>\n",
                    "BODY<br>\n[[/note]]</p>",
                ),
            ),
            (
                "unsafe style argument",
                concat!(
                    "[[note style=\"background:url(javascript:alert(1))\"]]\n",
                    "BODY\n[[/note]]",
                ),
                concat!(
                    "<p>[[note style=&quot;background:url(javascript:alert(1))&quot;]]",
                    "<br>\nBODY<br>\n[[/note]]</p>",
                ),
            ),
            (
                "extra opener bracket",
                "[[note]]]\nBODY\n[[/note]]",
                "<p>[[note]]]<br>\nBODY<br>\n[[/note]]</p>",
            ),
        ] {
            let (html, _errors) = render_wikidot(source);

            assert_eq!(html, expected, "{name}");
            assert!(html.contains("[[note"), "{name}: {html}");
        }
    }

    #[test]
    fn residual_note_markers_remain_visible() {
        for (name, source, expected) in [
            (
                "extra closer",
                "[[note]]\nBODY\n[[/note]]\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><p>BODY</p></div>"#,
                    "<br>\n[[/note]]",
                ),
            ),
            (
                "inline owner",
                "[[note]]BODY[[/note]]",
                "<p>[[note]]BODY[[/note]]</p>",
            ),
        ] {
            let (html, _errors) = render_wikidot(source);

            assert_eq!(html, expected, "{name}");
        }
    }

    #[test]
    fn live_backed_note_corpus_controls_keep_their_exact_dom() {
        for (name, source, expected) in [
            (
                "community inline example",
                "[[note]] ... [[/note]]",
                "<p>[[note]] … [[/note]]</p>",
            ),
            (
                "API warning",
                concat!(
                    "[[note]]\n",
                    "This method works for small files only (max 6MB).\n",
                    "[[/note]]",
                ),
                concat!(
                    r#"<div class="wiki-note"><p>"#,
                    "This method works for small files only (max 6MB).",
                    "</p></div>",
                ),
            ),
            (
                "theme reminder",
                concat!(
                    "[[note]]\n",
                    "💡 **REMEMBER** -- enclose every div with {{@@[[/div]]@@}}.\n",
                    "[[/note]]",
                ),
                concat!(
                    r#"<div class="wiki-note"><p>💡 <strong>REMEMBER</strong> "#,
                    "— enclose every div with ",
                    r#"<tt><span style="white-space: pre-wrap;">[[/div]]</span></tt>."#,
                    "</p></div>",
                ),
            ),
            (
                "colored heading",
                "[[note]]\n+++ ###990000|Records Notice: Discontinuation of Deletion##\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><h3 id="toc0"><span>"#,
                    r#"<span style="color: #990000">"#,
                    "Records Notice: Discontinuation of Deletion",
                    "</span></span></h3></div>",
                ),
            ),
            (
                "size span",
                "[[note]]\n[[size 75%]]bloblobloblobloblobloblobloblobbbbbbbb[[/size]]\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><p><span style="font-size:75%;">"#,
                    "bloblobloblobloblobloblobloblobbbbbbbb",
                    "</span></p></div>",
                ),
            ),
            (
                "plain sentence one",
                "[[note]]\nThe surface is just wonderful. Really wonderful! Extraordinary!\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><p>"#,
                    "The surface is just wonderful. Really wonderful! Extraordinary!",
                    "</p></div>",
                ),
            ),
            (
                "bold document title",
                "[[note]]\n**Document # ███-002: Excerpt from the \"von Reiter Collection\"**\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><p><strong>"#,
                    "Document # ███-002: Excerpt from the &quot;von Reiter Collection&quot;",
                    "</strong></p></div>",
                ),
            ),
            (
                "second-level heading",
                "[[note]]\n++ Abstract\n[[/note]]",
                r#"<div class="wiki-note"><h2 id="toc0"><span>Abstract</span></h2></div>"#,
            ),
            (
                "CJK sentence",
                "[[note]]\n当你看见这句时，你已经死了。\n[[/note]]",
                r#"<div class="wiki-note"><p>当你看见这句时，你已经死了。</p></div>"#,
            ),
            (
                "plain sentence two",
                "[[note]]\nThe sun is sooooo beautiful.\n[[/note]]",
                r#"<div class="wiki-note"><p>The sun is sooooo beautiful.</p></div>"#,
            ),
            (
                "apostrophe",
                "[[note]]\nCome back! It's warm up here.\n[[/note]]",
                r#"<div class="wiki-note"><p>Come back! It&#39;s warm up here.</p></div>"#,
            ),
            (
                "triple link",
                "[[note]]\n[[[http://www.scp-wiki.net |Log out]]]\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><p>"#,
                    r#"<a href="http://www.scp-wiki.net">Log out</a></p></div>"#,
                ),
            ),
            (
                "plain sentence three",
                "[[note]]\nGOC Under-Secretary-General D.C. al Fine\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><p>"#,
                    "GOC Under-Secretary-General D.C. al Fine</p></div>",
                ),
            ),
            (
                "local link",
                "[[note]]\n[/css-theme-preparation-tool return to tool]\n[[/note]]",
                concat!(
                    r#"<div class="wiki-note"><p>"#,
                    r#"<a href="/css-theme-preparation-tool">return to tool</a>"#,
                    "</p></div>",
                ),
            ),
        ] {
            let (html, _errors) = render_wikidot(source);

            assert_eq!(html, expected, "{name}");
        }
    }

    #[test]
    fn email_corpus_control_keeps_note_ownership_without_freezing_issue_294() {
        let source = concat!(
            "[[note]]\n",
            "For more information, please contact: support@wikidot.com\n",
            "[[/note]]",
        );
        let (html, errors) = render_wikidot(source);

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.starts_with(r#"<div class="wiki-note"><p>"#), "{html}");
        assert!(html.ends_with("</p></div>"), "{html}");
        assert!(html.contains(r#"class="wiki-email""#), "{html}");
        assert!(!html.contains("[[note]]"), "{html}");
        assert!(!html.contains("[[/note]]"), "{html}");
    }

    #[test]
    fn note_and_surrounding_prose_keep_separate_owners() {
        let (html, errors) = render_wikidot("BEFORE\n[[note]]\nBODY\n[[/note]]\nAFTER");

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            concat!(
                "<p>BEFORE</p>",
                r#"<div class="wiki-note"><p>BODY</p></div>"#,
                "<br>\nAFTER",
            ),
        );
    }

    #[test]
    fn wikijump_layout_keeps_recursive_note_semantics() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        let source = "[[note]]\nA\n[[note]]\nB\n[[/note]]\nC\n[[/note]]";
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html.matches(r#"class="wj-note""#).count(), 2, "{html}");
        assert!(!html.contains("[[note]]"), "{html}");
        assert!(!html.contains("[[/note]]"), "{html}");
    }
}
