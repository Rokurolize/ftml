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
    let Some((file, label)) = value.split_once(" | ") else {
        return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments));
    };
    if file.is_empty()
        || label.is_empty()
        || label.contains('|')
        || matches!(file, "." | "..")
        || !file
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments));
    }

    Ok((cow!(file), cow!(label)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn file_link_rejects_unverified_forms() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        for source in [
            "[[file elements.tsv]]",
            "[[file other-page/elements.tsv | Download Catalog]]",
            "[[file ../elements.tsv | Download Catalog]]",
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
