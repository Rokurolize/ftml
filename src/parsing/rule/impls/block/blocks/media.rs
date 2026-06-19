/*
 * parsing/rule/impls/block/blocks/media.rs
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
use crate::tree::{Alignment, FileSource, FloatAlignment};

pub const BLOCK_AUDIO: BlockRule = BlockRule {
    name: "block-audio",
    accepts_names: &["audio"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: false,
    parse_fn: parse_audio,
};

pub const BLOCK_VIDEO: BlockRule = BlockRule {
    name: "block-video",
    accepts_names: &["video"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: false,
    parse_fn: parse_video,
};

fn parse_audio<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    parse_media_block(
        parser,
        &BLOCK_AUDIO,
        name,
        flag_star,
        flag_score,
        in_head,
        |source, alignment, attributes| Element::Audio {
            source,
            alignment,
            attributes,
        },
    )
}

fn parse_video<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    parse_media_block(
        parser,
        &BLOCK_VIDEO,
        name,
        flag_star,
        flag_score,
        in_head,
        |source, alignment, attributes| Element::Video {
            source,
            alignment,
            attributes,
        },
    )
}

fn parse_media_block<'r, 't, F>(
    parser: &mut Parser<'r, 't>,
    block_rule: &BlockRule,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
    build_element: F,
) -> ParseResult<'r, 't, Elements<'t>>
where
    F: FnOnce(
        FileSource<'t>,
        Option<FloatAlignment>,
        crate::tree::AttributeMap<'t>,
    ) -> Element<'t>,
{
    debug!(
        "Parsing media block (rule {}, name {name}, in-head {in_head})",
        block_rule.name
    );
    assert!(!flag_star, "Media blocks don't allow star flag");
    assert!(!flag_score, "Media blocks don't allow score flag");
    assert_block_name(block_rule, name);

    let (source, mut arguments) = parser.get_head_name_map(block_rule, in_head)?;
    let alignment = parse_media_alignment(parser, &mut arguments)?;

    let source = match FileSource::parse(source) {
        Some(source) => source,
        None => return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments)),
    };

    if arguments.get("src").is_some() {
        return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments));
    }

    // TODO: html render settings to allow this?
    let _autoplay = arguments.get("autoplay");

    ok!(build_element(
        source,
        alignment,
        arguments.to_attribute_map(parser.settings()),
    ))
}

fn parse_media_alignment<'r, 't>(
    parser: &mut Parser<'r, 't>,
    arguments: &mut Arguments<'t>,
) -> Result<Option<FloatAlignment>, ParseError> {
    let Some(value) = arguments.get("align") else {
        return Ok(None);
    };

    let align = match value.as_ref() {
        "left" => Alignment::Left,
        "right" => Alignment::Right,
        "center" => Alignment::Center,
        _ => return Err(parser.make_err(ParseErrorKind::BlockMalformedArguments)),
    };

    Ok(Some(FloatAlignment {
        align,
        float: false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    fn parse_single_media(input: &str) -> Element<'static> {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        let tokenization = crate::tokenize(input);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "media block should parse cleanly");
        assert_eq!(tree.elements.len(), 1);

        let element = tree.elements.into_iter().next().unwrap().to_owned();
        match element {
            Element::Container(container) => {
                assert_eq!(container.elements().len(), 1);
                container.elements()[0].to_owned()
            }
            other => other,
        }
    }

    #[test]
    fn media_blocks_parse_audio_and_video_with_alignment() {
        let audio = parse_single_media(
            r#"[[audio filename.mp3 align="left" class="custom-audio"]]"#,
        );
        let video = parse_single_media(
            r#"[[video filename.mp4 align="right" class="custom-video"]]"#,
        );

        match audio {
            Element::Audio {
                source,
                alignment,
                attributes,
            } => {
                assert_eq!(
                    source,
                    FileSource::File1 {
                        file: cow!("filename.mp3"),
                    },
                );
                assert_eq!(
                    alignment,
                    Some(FloatAlignment {
                        align: Alignment::Left,
                        float: false,
                    }),
                );
                assert_eq!(
                    attributes.get().get("class").map(|value| value.as_ref()),
                    Some("custom-audio"),
                );
            }
            other => panic!("expected audio element, got {other:?}"),
        }

        match video {
            Element::Video {
                source,
                alignment,
                attributes,
            } => {
                assert_eq!(
                    source,
                    FileSource::File1 {
                        file: cow!("filename.mp4"),
                    },
                );
                assert_eq!(
                    alignment,
                    Some(FloatAlignment {
                        align: Alignment::Right,
                        float: false,
                    }),
                );
                assert_eq!(
                    attributes.get().get("class").map(|value| value.as_ref()),
                    Some("custom-video"),
                );
            }
            other => panic!("expected video element, got {other:?}"),
        }
    }
}
