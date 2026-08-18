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

    if parser.settings().layout.legacy() {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    let (arguments, body_start) =
        parser.get_head_map_with_body_start(&BLOCK_NOTE, in_head)?;
    if matches!(body_start, BlockBodyStart::Inline) {
        return Err(parser.make_err(ParseErrorKind::NotSupportedInline));
    }
    let (elements, errors, _paragraph_safe) =
        parser.get_body_elements(&BLOCK_NOTE, false)?.into();
    let element = Element::Container(Container::new(
        ContainerType::Note,
        elements,
        arguments.to_attribute_map(parser.settings()),
    ));

    ok!(false; element, errors)
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};

    fn render(source: &str, layout: Layout) -> (String, Vec<crate::parsing::ParseError>) {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
        let mut source = source.to_owned();
        crate::preprocess_for_layout(&mut source, settings.layout);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        (html, errors)
    }

    #[test]
    fn wikidot_note_matches_literal_live_pagepreview() {
        let (html, _errors) =
            render("[[note]]\nEvidence-backed note.[[/note]]", Layout::Wikidot);

        assert!(html.contains("[[note]]"), "{html}");
        assert!(html.contains("[[/note]]"), "{html}");
        assert!(!html.contains("class=\"wiki-note\""), "{html}");
        assert!(!html.contains("class=\"wj-note\""), "{html}");
    }

    #[test]
    fn wikijump_note_uses_native_dom_and_preserves_attributes() {
        let (html, errors) = render(
            "[[note class=\"custom\" data-kind=\"example\"]]\nBody\n[[/note]]",
            Layout::Wikijump,
        );

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            r#"<div class="wj-note custom" data-kind="example">Body</div>"#,
        );
    }

    #[test]
    fn wikijump_note_rejects_inline_openers() {
        let (html, errors) = render("prefix [[note]]Body[[/note]]", Layout::Wikijump);

        assert!(
            errors.iter().any(|error| error.kind()
                == crate::parsing::ParseErrorKind::NotSupportedInline),
            "html={html}; errors={errors:#?}",
        );
        assert!(!html.contains("class=\"wj-note"), "{html}");
    }

    #[test]
    fn wikijump_nested_notes_keep_native_ownership() {
        let source = "[[note]]\nA\n[[note]]\nB\n[[/note]]\nC\n[[/note]]";
        let (html, errors) = render(source, Layout::Wikijump);

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html.matches(r#"class="wj-note""#).count(), 2, "{html}");
        assert!(!html.contains("[[note]]"), "{html}");
        assert!(!html.contains("[[/note]]"), "{html}");
    }
}
