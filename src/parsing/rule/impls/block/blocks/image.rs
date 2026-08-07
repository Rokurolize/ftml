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
use crate::delayed::{DelayedElement, GeneratedImageAttribute, GeneratedKind};
use crate::tree::{FileSource, FloatAlignment, LinkLocation};
use crate::url::is_url;
use std::borrow::Cow;

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

    let generated = parser.generated_until_right_block();
    if generated
        .iter()
        .any(|slot| slot.kind == GeneratedKind::PageLink)
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    let generated_image = match generated.as_slice() {
        [] => None,
        [slot] => {
            let (key, suffix) = delayed_image_attribute(parser.full_text().inner(), slot)
                .ok_or_else(|| parser.make_err(ParseErrorKind::RuleFailed))?;
            let attribute = match key {
                "alt" => GeneratedImageAttribute::Alt,
                "link" => GeneratedImageAttribute::Link,
                _ => return Err(parser.make_err(ParseErrorKind::RuleFailed)),
            };
            Some((slot.clone(), attribute, suffix))
        }
        _ => return Err(parser.make_err(ParseErrorKind::RuleFailed)),
    };

    let (source, source_prefix_is_url, mut arguments) =
        if parser.settings().layout.legacy() {
            let (source, arguments) =
                parser.get_head_field_map_wikidot(&BLOCK_IMAGE, in_head)?;
            let source_prefix_is_url = is_url(source.prefix_before_first_comment());
            (source.into_cow(), source_prefix_is_url, arguments)
        } else {
            let (source, arguments) = parser.get_head_name_map(&BLOCK_IMAGE, in_head)?;
            (Cow::Borrowed(source), is_url(source), arguments)
        };
    let link = if parser.settings().layout.legacy() {
        match arguments.get("link") {
            None => None,
            Some(value) => Some(
                parse_wikidot_image_link_target(value)
                    .ok_or_else(|| parser.make_err(ParseErrorKind::RuleFailed))?,
            ),
        }
    } else {
        arguments.get_with_bare("link").and_then(|(value, bare)| {
            if bare && value == "#" {
                None
            } else {
                Some(LinkLocation::parse(value))
            }
        })
    };
    let alignment = FloatAlignment::parse(name);

    // Parse the image source based on format
    if is_url(&source) && !source_prefix_is_url {
        return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments));
    }
    let source = match if parser.settings().layout.legacy() {
        parse_wikidot_file_source(source)
    } else {
        FileSource::parse(&source).map(|source| source.to_owned())
    } {
        Some(source) => source,
        None => return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments)),
    };

    if let Some((slot, attribute, suffix)) = generated_image {
        match attribute {
            GeneratedImageAttribute::Alt => {
                let _ = arguments.get("alt");
            }
            GeneratedImageAttribute::Link => {
                let _ = arguments.get("link");
            }
        }
        return success_elements(Element::Delayed(DelayedElement::tag_image(
            source,
            link,
            alignment,
            arguments.to_attribute_map(parser.settings()),
            attribute,
            suffix,
            slot.id,
        )));
    }

    // Build image
    let element = Element::Image {
        source,
        link,
        alignment,
        attributes: arguments.to_attribute_map(parser.settings()),
    };

    success_elements(element)
}

fn parse_wikidot_file_source<'t>(source: Cow<'t, str>) -> Option<FileSource<'t>> {
    match source {
        Cow::Borrowed(source) => FileSource::parse_wikidot(source),
        Cow::Owned(source) => {
            FileSource::parse_wikidot(&source).map(|source| source.to_owned())
        }
    }
}

fn parse_wikidot_image_link_target<'t>(target: Cow<'t, str>) -> Option<LinkLocation<'t>> {
    if target.starts_with("https://") {
        return None;
    }

    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let anchor = target.starts_with('#');
    let target = if anchor {
        target.as_ref()
    } else {
        target.trim_start_matches('/')
    };
    let mut encoded = String::with_capacity(target.len() + usize::from(!anchor));
    if !anchor {
        encoded.push('/');
    }
    for byte in target.bytes() {
        if byte == b' ' || !byte.is_ascii() {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        } else {
            encoded.push(char::from(byte));
        }
    }

    Some(LinkLocation::Url(Cow::Owned(encoded)))
}

fn delayed_image_attribute<'a>(
    source: &'a str,
    slot: &crate::delayed::GeneratedInput,
) -> Option<(&'a str, &'a str)> {
    let prefix = &source[..slot.source_range.start];
    let equals = prefix.rfind('=')?;
    if !prefix[equals + 1..]
        .chars()
        .all(|character| matches!(character, '"' | '\''))
    {
        return None;
    }
    let key_start = prefix[..equals]
        .rfind(|character: char| {
            !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
        })
        .map_or(0, |position| position + 1);
    let key = &prefix[key_start..equals];
    let quote = *source
        .as_bytes()
        .get(slot.source_range.start.checked_sub(1)?)?;
    if !matches!(quote, b'"' | b'\'') {
        return None;
    }
    let suffix_end = source[slot.source_range.end..]
        .bytes()
        .position(|byte| byte == quote)?;
    let suffix = &source[slot.source_range.end..slot.source_range.end + suffix_end];
    (!key.is_empty()).then_some((key, suffix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::settings::{WikitextMode, WikitextSettings};
    use std::time::{Duration, Instant};

    fn render_image(
        source: &str,
        layout: Layout,
    ) -> (String, Vec<crate::parsing::ParseError>) {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
        let mut source = source.to_owned();
        crate::preprocess_for_layout(&mut source, layout);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = crate::render::html::HtmlRender
            .render(&tree, &page_info, &settings)
            .body;

        (html, errors)
    }

    #[test]
    fn wikidot_double_quoted_image_links_preserve_legacy_internal_targets() {
        // Immutable live preview evidence:
        // /mnt/oracle-store/wjlab/ftml-scout-20260730/image-link-target-live.jsonl
        for (target, expected_href) in [
            ("SCP-002", "/SCP-002"),
            ("scp-002", "/scp-002"),
            ("ScP-002", "/ScP-002"),
            ("Component:Image-Block", "/Component:Image-Block"),
            ("component:image-block", "/component:image-block"),
            ("Foo Bar", "/Foo%20Bar"),
            ("foo_bar", "/foo_bar"),
            ("日本語", "/%E6%97%A5%E6%9C%AC%E8%AA%9E"),
            ("SCP-002/noredirect/true", "/SCP-002/noredirect/true"),
            ("/SCP-002", "/SCP-002"),
            ("#Section", "#Section"),
            (".", "/."),
            ("..", "/.."),
            ("//Example.COM/Path", "/Example.COM/Path"),
            ("mailto:User@Example.COM", "/mailto:User@Example.COM"),
            ("", "/"),
        ] {
            let source =
                format!(r#"[[image https://example.com/x.png link="{target}"]]"#,);
            let (html, errors) = render_image(&source, Layout::Wikidot);

            assert!(errors.is_empty(), "{target:?}: {errors:#?}");
            assert_eq!(
                html,
                format!(
                    r#"<a href="{expected_href}"><img src="https://example.com/x.png" class="image" alt="x.png"></a>"#,
                ),
                "{target:?}",
            );
        }
    }

    #[test]
    fn wikidot_qualified_https_image_link_rejects_the_whole_image_candidate() {
        let source = concat!(
            "BEGIN|[[image https://example.com/x.png ",
            r#"link="https://Example.COM/Path?Q=X#Frag"]]|END"#,
        );
        let (html, errors) = render_image(source, Layout::Wikidot);

        assert!(!errors.is_empty());
        assert_eq!(
            html,
            concat!(
                "<p>BEGIN|[[image ",
                r#"<a href="https://example.com/x.png">https://example.com/x.png</a> "#,
                r#"link=&quot;<a href="https://Example.COM/Path?Q=X#Frag">"#,
                "https://Example.COM/Path?Q=X#Frag</a>&quot;]]|END</p>",
            ),
        );
        assert!(!html.contains("<a href=\"https://Example.COM/Path?Q=X#Frag\"><img"));
    }

    #[test]
    fn wikidot_single_quoted_and_bare_image_links_remain_inert() {
        for target in [
            "SCP-002",
            "scp-002",
            "ScP-002",
            "Component:Image-Block",
            "component:image-block",
            "Foo Bar",
            "foo_bar",
            "日本語",
            "/SCP-002",
            "SCP-002/noredirect/true",
            "#Section",
            ".",
            "..",
            "https://Example.COM/Path?Q=X#Frag",
            "//Example.COM/Path",
            "mailto:User@Example.COM",
            "",
        ] {
            for argument in [format!("link='{target}'"), format!("link={target}")] {
                let source = format!("[[image https://example.com/x.png {argument}]]");
                let (html, errors) = render_image(&source, Layout::Wikidot);

                assert!(errors.is_empty(), "{argument}: {errors:#?}");
                assert_eq!(
                    html,
                    r#"<img src="https://example.com/x.png" class="image" alt="x.png">"#,
                    "{argument}",
                );
            }
        }
    }

    #[test]
    fn malformed_qualified_image_link_target_stays_literal_and_inert() {
        let source = concat!(
            "[[image https://example.com/x.png ",
            r#"link="https://"]]tail"#,
        );
        let (html, errors) = render_image(source, Layout::Wikidot);

        assert!(!errors.is_empty());
        assert!(!html.contains("<img"), "{html}");
        assert!(!html.contains("<a href=\"https://\"><img"));
        assert!(html.contains("[[image "), "{html}");
        assert!(html.contains("]]tail"), "{html}");
    }

    #[test]
    fn repeated_malformed_qualified_image_links_stay_bounded_and_inert() {
        const CANDIDATE_COUNT: usize = 128;
        let candidate = concat!(
            "[[image https://example.com/x.png ",
            r#"link="https://"]]"#,
            "\n",
        );
        let source = candidate.repeat(CANDIDATE_COUNT);
        let started = Instant::now();
        let (html, errors) = render_image(&source, Layout::Wikidot);

        assert!(started.elapsed() < Duration::from_secs(5));
        assert!(!errors.is_empty());
        assert_eq!(html.matches("[[image ").count(), CANDIDATE_COUNT);
        assert_eq!(html.matches("<img").count(), 0);
    }

    #[test]
    fn wikijump_image_link_behavior_is_unchanged() {
        let source = concat!(
            "[[image https://example.com/x.png ",
            r#"link="https://Example.COM/Path?Q=X#Frag"]]"#,
        );
        let (html, errors) = render_image(source, Layout::Wikijump);

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains(r#"href="https://Example.COM/Path?Q=X#Frag""#));
        assert!(html.contains("<img"));
    }

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
    fn wikidot_structural_quoted_local_paths_remain_literal() {
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

        assert!(!errors.is_empty(), "{errors:#?}");
        assert!(!format!("{tree:?}").contains("Image {"), "{tree:#?}");
        assert_eq!(
            html,
            r#"<p>[[image &quot;/local—files/source-page/image.png&quot;]]</p>"#,
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
