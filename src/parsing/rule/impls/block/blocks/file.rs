/*
 * parsing/rule/impls/block/blocks/file.rs
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
pub const BLOCK_FILE: BlockRule = BlockRule {
    name: "block-file",
    accepts_names: &["file"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: false,
    parse_fn,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Parsing file link block (name '{name}', in-head {in_head})");
    assert!(!flag_star, "File doesn't allow star flag");
    assert!(!flag_score, "File doesn't allow score flag");
    assert_block_name(&BLOCK_FILE, name);

    let (file, label) =
        parser.get_head_value(&BLOCK_FILE, in_head, parse_evidenced_file_link)?;

    success_elements(Element::FileLink { file, label })
}

fn parse_evidenced_file_link<'t>(
    parser: &Parser<'_, 't>,
    value: Option<&'t str>,
) -> Result<(std::borrow::Cow<'t, str>, std::borrow::Cow<'t, str>), ParseError> {
    if !parser.settings().allow_local_paths {
        return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments));
    }

    let value = require_trimmed_block_argument(parser, value)?;
    if !parser.settings().layout.legacy() {
        let Some((file, label)) = value.split_once(" | ") else {
            return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments));
        };
        if !is_evidenced_file_name(file) || label.is_empty() || label.contains('|') {
            return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments));
        }
        return Ok((cow!(file), cow!(label)));
    }

    let (file, label) = match value.split_once('|') {
        Some((file, label)) if !label.contains('|') => {
            let file = file.trim();
            let label = label.trim();
            (
                file,
                if label.is_empty() {
                    file_name(file)
                } else {
                    label
                },
            )
        }
        Some(_) => return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments)),
        None => (value, file_name(value)),
    };
    if !is_evidenced_wikidot_file_source(file) {
        return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments));
    }

    Ok((cow!(file), cow!(label)))
}

fn file_name(source: &str) -> &str {
    source.rsplit('/').next().unwrap_or(source)
}

fn is_evidenced_wikidot_file_source(source: &str) -> bool {
    for part in source.split('/') {
        if part.is_empty()
            || part == "."
            || part == ".."
            || !is_evidenced_file_path_part(part)
        {
            return false;
        }
    }
    true
}

fn is_evidenced_file_name(source: &str) -> bool {
    !matches!(source, "" | "." | "..") && is_evidenced_file_path_part(source)
}

fn is_evidenced_file_path_part(part: &str) -> bool {
    part.chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::render::html::HtmlRender;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn wikidot_file_link_accepts_live_evidenced_forms_without_file_state() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        // Saved-page cases record--a31ae3233a367367f65eb4cb, record--95406cc12956c54f1dd849bf, record--5a933922d44be7b45247533e, and record--c99e6dd95cff0355c4ac6408 produced anchors on Wikidot without uploaded-file state.
        for (source, file, label) in [
            ("[[file elements.tsv]]", "elements.tsv", "elements.tsv"),
            (
                "[[file other-page/elements.tsv | Download Catalog]]",
                "other-page/elements.tsv",
                "Download Catalog",
            ),
            (
                "[[file elements.tsv|Download Catalog]]",
                "elements.tsv",
                "Download Catalog",
            ),
            ("[[file elements.tsv | ]]", "elements.tsv", "elements.tsv"),
        ] {
            let tokenization = crate::tokenize(source);
            let (tree, errors) =
                crate::parse(&tokenization, &page_info, &settings).into();

            assert!(errors.is_empty(), "{source} should parse: {errors:#?}");
            let [Element::Container(paragraph)] = tree.elements.as_slice() else {
                panic!(
                    "{source} should produce one paragraph: {:#?}",
                    tree.elements
                );
            };
            assert_eq!(
                paragraph.elements(),
                [Element::FileLink {
                    file: cow!(file),
                    label: cow!(label),
                }],
            );
        }
    }

    #[test]
    fn wikidot_file_link_rejects_shapes_without_live_evidence() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        for source in [
            "[[file .]]",
            "[[file ..]]",
            "[[file ../elements.tsv | Download Catalog]]",
            "[[file elements.tsv | label | extra]]",
            "[[file path with spaces/elements.tsv]]",
        ] {
            let tokenization = crate::tokenize(source);
            let (tree, errors) =
                crate::parse(&tokenization, &page_info, &settings).into();

            assert!(
                errors
                    .iter()
                    .any(|error| error.kind() == ParseErrorKind::BlockMalformedArguments),
                "{source} should fail closed: {errors:#?}",
            );
            assert!(
                !tree
                    .elements
                    .iter()
                    .any(|element| matches!(element, Element::FileLink { .. })),
                "{source} must not become a file link",
            );
        }
    }

    #[test]
    fn wikidot_file_link_renders_live_evidenced_target_and_label_shapes() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        for (source, expected) in [
            (
                "[[file elements.tsv]]",
                r#"<p><a href="https://sandbox.wjfiles.com/local--files/some-page/elements.tsv">elements.tsv</a></p>"#,
            ),
            (
                "[[file other-page/elements.tsv | Download Catalog]]",
                r#"<p><a href="https://sandbox.wjfiles.com/local--files/other-page/elements.tsv">Download Catalog</a></p>"#,
            ),
            (
                "[[file elements.tsv|Download Catalog]]",
                r#"<p><a href="https://sandbox.wjfiles.com/local--files/some-page/elements.tsv">Download Catalog</a></p>"#,
            ),
            (
                "[[file elements.tsv | ]]",
                r#"<p><a href="https://sandbox.wjfiles.com/local--files/some-page/elements.tsv">elements.tsv</a></p>"#,
            ),
        ] {
            let tokenization = crate::tokenize(source);
            let (tree, errors) =
                crate::parse(&tokenization, &page_info, &settings).into();

            assert!(errors.is_empty(), "{source} should parse: {errors:#?}");
            assert_eq!(
                HtmlRender.render(&tree, &page_info, &settings).body,
                expected,
            );
        }
    }

    #[test]
    fn wikijump_layout_keeps_the_native_file_link_grammar() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);

        for source in [
            "[[file elements.tsv]]",
            "[[file other-page/elements.tsv | Download Catalog]]",
            "[[file elements.tsv|Download Catalog]]",
            "[[file elements.tsv | ]]",
        ] {
            let tokenization = crate::tokenize(source);
            let (tree, errors) =
                crate::parse(&tokenization, &page_info, &settings).into();

            assert!(
                errors
                    .iter()
                    .any(|error| error.kind() == ParseErrorKind::BlockMalformedArguments),
                "{source} should retain the native grammar: {errors:#?}",
            );
            assert!(
                !tree
                    .elements
                    .iter()
                    .any(|element| matches!(element, Element::FileLink { .. })),
            );
        }
    }

    #[test]
    fn file_link_is_disabled_for_forum_posts() {
        let page_info = PageInfo::dummy();
        let settings =
            WikitextSettings::from_mode(WikitextMode::ForumPost, Layout::Wikidot);
        let tokenization = crate::tokenize("[[file elements.tsv | Download Catalog]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(
            errors
                .iter()
                .any(|error| error.kind() == ParseErrorKind::BlockMalformedArguments),
            "{errors:#?}",
        );
        assert!(
            !tree
                .elements
                .iter()
                .any(|element| matches!(element, Element::FileLink { .. })),
        );
    }
}
