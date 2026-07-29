/*
 * parsing/rule/impls/block/blocks/image.rs
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
use crate::tree::{FileSource, FloatAlignment, LinkLocation};

pub const BLOCK_IMAGE: BlockRule = BlockRule {
    name: "block-image",
    accepts_names: &[
        "image", "=image", "<image", ">image", "f<image", "f=image", "f>image",
    ],
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
    debug!("Parsing image block (name {name}, in-head {in_head})");
    assert!(!flag_star, "Image doesn't allow star flag");
    assert!(!flag_score, "Image doesn't allow score flag");
    assert_block_name(&BLOCK_IMAGE, name);

    let (source, mut arguments) =
        parser.get_head_name_map_wikidot(&BLOCK_IMAGE, in_head)?;
    let link = arguments.get_with_bare("link").and_then(|(value, bare)| {
        if bare && value == "#" {
            None
        } else {
            Some(LinkLocation::parse(value))
        }
    });
    let alignment = FloatAlignment::parse(name);

    // Parse the image source based on format
    let source = match if parser.settings().layout.legacy() {
        FileSource::parse_wikidot(source)
    } else {
        FileSource::parse(source)
    } {
        Some(source) => source,
        None => return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments)),
    };

    // Build image
    let element = Element::Image {
        source,
        link,
        alignment,
        attributes: arguments.to_attribute_map(parser.settings()),
    };

    success_elements(element)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn wikidot_image_accepts_multi_segment_local_source() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[image a/b/c/d.png]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let [Element::Image { source, .. }] = tree.elements.as_slice() else {
            panic!("expected direct image element, got {:?}", tree.elements);
        };
        assert_eq!(
            source,
            &FileSource::File2 {
                page: cow!("a/b/c"),
                file: cow!("d.png"),
            },
        );
    }

    #[test]
    fn wikidot_root_file_source_has_one_path_separator() {
        let mut page_info = PageInfo::dummy();
        page_info.site = cow!("sandbox");
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[image /my-picture.png]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = crate::render::html::HtmlRender
            .render(&tree, &page_info, &settings)
            .body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            "<img src=\"https://sandbox.wjfiles.com/local--files/my-picture.png\" class=\"image\" alt=\"my-picture.png\">",
        );
    }

    #[test]
    fn wikidot_unsaved_preview_links_relative_images_to_medium_resizes() {
        let mut page_info = PageInfo::dummy();
        page_info.site = cow!("sandbox-for-codex");
        page_info.page = cow!("");
        page_info.category = None;
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[image fog-green.svg]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = crate::render::html::HtmlRender
            .render(&tree, &page_info, &settings)
            .body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            concat!(
                "<a href=\"https://sandbox-for-codex.wjfiles.com/local--files//fog-green.svg\">",
                "<img src=\"https://sandbox-for-codex.wjfiles.com/local--resized-images//fog-green.svg/medium.jpg\" alt=\"fog-green.svg\" class=\"image\">",
                "</a>",
            ),
        );
    }

    #[test]
    fn wikidot_quoted_absolute_image_sources_keep_their_quotes() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        for (source, expected) in [
            (
                r#"[[image "https://example.com/a.png"]]"#,
                r#"<img src="&quot;https://example.com/a.png&quot;" class="image" alt="a.png&quot;">"#,
            ),
            (
                "[[image 'https://example.com/a.png']]",
                r#"<img src="&#39;https://example.com/a.png&#39;" class="image" alt="a.png&#39;">"#,
            ),
        ] {
            let tokenization = crate::tokenize(source);
            let (tree, errors) =
                crate::parse(&tokenization, &page_info, &settings).into();
            let html = crate::render::html::HtmlRender
                .render(&tree, &page_info, &settings)
                .body;

            assert!(errors.is_empty(), "{source}: {errors:#?}");
            assert_eq!(html, expected, "{source}");
        }
    }

    #[test]
    fn wikidot_quoted_local_paths_remain_single_local_filenames() {
        let mut page_info = PageInfo::dummy();
        page_info.site = cow!("sandbox-for-codex");
        page_info.page = cow!("");
        page_info.category = None;
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let source = r#"[[image "/local--files/source-page/image.png"]]"#;
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = crate::render::html::HtmlRender
            .render(&tree, &page_info, &settings)
            .body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            concat!(
                "<a href=\"https://sandbox-for-codex.wjfiles.com/local--files//&quot;/local--files/source-page/image.png&quot;\">",
                "<img src=\"https://sandbox-for-codex.wjfiles.com/local--resized-images//&quot;/local--files/source-page/image.png&quot;/medium.jpg\" ",
                "alt=\"image.png&quot;\" class=\"image\">",
                "</a>",
            ),
        );
    }

    #[test]
    fn image_block_preserves_canonical_wikidot_local_files_path() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(
            "[[image /local--files/source-page/assets/charts/image.png]]",
        );
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        let [Element::Image { source, .. }] = tree.elements.as_slice() else {
            panic!("expected direct image element, got {:?}", tree.elements);
        };

        assert_eq!(
            source,
            &FileSource::Url(cow!("/local--files/source-page/assets/charts/image.png")),
        );
    }

    #[test]
    fn unaligned_image_suppresses_the_contiguous_paragraph_wrapper() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization =
            crate::tokenize("BASIC [[image /local--files/source-page/filename.png]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = crate::render::html::HtmlRender
            .render(&tree, &page_info, &settings)
            .body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            "\n\nBASIC <img src=\"/local--files/source-page/filename.png\" class=\"image\" alt=\"filename.png\">",
        );
    }

    #[test]
    fn blank_line_after_unaligned_image_preserves_wikidot_separator() {
        let mut page_info = PageInfo::dummy();
        page_info.site = cow!("sandbox");
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("BASIC [[image filename.png]]\n\nLEFT");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = crate::render::html::HtmlRender
            .render(&tree, &page_info, &settings)
            .body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            "\n\nBASIC <img src=\"https://sandbox.wjfiles.com/local--files/some-page/filename.png\" class=\"image\" alt=\"filename.png\"> <p>LEFT</p>",
        );
    }

    #[test]
    fn image_breaks_a_contiguous_div_paragraph_like_wikidot() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = concat!(
            "[[div class=\"name\"]]\n",
            "NFSI\n",
            "[[image /local--files/scp-9506/NFSI.png]]\n",
            "[[span style=\"font-size:2rem\"]]National Fog Safety Initiative[[/span]]\n",
            "[[/div]]\n",
        );
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = crate::render::html::HtmlRender
            .render(&tree, &page_info, &settings)
            .body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains("<div class=\"name\">\n\nNFSI<br>"), "{html}");
        assert!(
            html.contains("NFSI.png\" class=\"image\" alt=\"NFSI.png\"><br>\n<span"),
            "{html}",
        );
        assert!(!html.contains("<div class=\"name\"><p>"), "{html}");
    }

    #[test]
    fn blank_line_keeps_text_paragraph_separate_from_naked_image() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let input = concat!(
            "[[div class=\"picture\"]]\n",
            "[[span class=\"heading2\"]]BREAKING[[/span]]\n",
            "\n",
            "[[image /local--files/scp-9506/fog.jpg]]\n",
            "[[/div]]\n",
        );
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = crate::render::html::HtmlRender
            .render(&tree, &page_info, &settings)
            .body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            "<div class=\"picture\"><p><span class=\"heading2\">BREAKING</span></p><img src=\"/local--files/scp-9506/fog.jpg\" class=\"image\" alt=\"fog.jpg\"></div>",
        );
    }

    #[test]
    fn wikidot_float_center_image_uses_plain_image_container() {
        let mut page_info = PageInfo::dummy();
        page_info.site = cow!("sandbox-for-codex");
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("FLOAT CENTER [[f=image landscape.jpg]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = crate::render::html::HtmlRender
            .render(&tree, &page_info, &settings)
            .body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            html,
            concat!(
                "<p>FLOAT CENTER</p>",
                "<div class=\"image-container\">",
                "<img src=\"https://sandbox-for-codex.wjfiles.com/local--files/some-page/landscape.jpg\" class=\"image\" alt=\"landscape.jpg\">",
                "</div>",
            ),
        );
    }
}
