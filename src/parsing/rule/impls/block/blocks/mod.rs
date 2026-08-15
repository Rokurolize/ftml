/*
 * parsing/rule/impls/block/blocks/mod.rs
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

mod prelude {
    pub use super::super::{Arguments, BlockRule};
    use crate::delayed::DelayedElement;
    pub use crate::parsing::ParseError;
    pub use crate::parsing::parser::Parser;
    pub use crate::parsing::prelude::*;
    pub use crate::tree::{Container, ContainerType, Element};

    #[cfg(debug_assertions)]
    pub fn assert_generic_name(
        expected_names: &[&str],
        actual_name: &str,
        name_type: &str,
    ) {
        let matched = expected_names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(actual_name));
        assert!(
            matched,
            "Actual {name_type} name doesn't match any expected: {expected_names:?} (was {actual_name})",
        );
    }

    #[cfg(not(debug_assertions))]
    #[inline]
    pub fn assert_generic_name(_: &[&str], _: &str, _: &str) {}

    #[inline]
    pub fn assert_block_name(block_rule: &BlockRule, actual_name: &str) {
        assert_generic_name(block_rule.accepts_names, actual_name, "block")
    }

    pub fn require_block_argument<'r, 't>(
        parser: &Parser<'r, 't>,
        value: Option<&'t str>,
    ) -> Result<&'t str, ParseError> {
        match value {
            Some(value) => Ok(value),
            None => Err(parser.make_err(ParseErrorKind::BlockMissingArguments)),
        }
    }

    pub fn require_trimmed_block_argument<'r, 't>(
        parser: &Parser<'r, 't>,
        value: Option<&'t str>,
    ) -> Result<&'t str, ParseError> {
        require_block_argument(parser, value).map(str::trim)
    }

    #[cold]
    #[inline(never)]
    pub fn recover_wikidot_empty_key_candidate<'r, 't>(
        parser: &mut Parser<'r, 't>,
        block_rule: &BlockRule,
        literal_start: usize,
        paragraph_safe: bool,
    ) -> ParseResult<'r, 't, Elements<'t>>
    where
        'r: 't,
    {
        let error = parser.make_err(ParseErrorKind::BlockMalformedArguments);
        if !parser.has_body_end_block(block_rule) {
            return Err(error);
        }

        let _ = parser.get_body_text(block_rule)?;
        let owner_end = parser.current().span.start;
        let source = parser.full_text().inner();
        let generated = parser.generated_in_range(literal_start..owner_end);
        let elements = if generated.is_empty() {
            if paragraph_safe {
                literal_block_candidate(&source[literal_start..owner_end])
            } else {
                text!(&source[literal_start..owner_end]).into()
            }
        } else {
            Element::Delayed(DelayedElement::shell(
                source,
                literal_start..owner_end,
                &generated,
            ))
            .into()
        };

        ok!(paragraph_safe; elements, vec![error])
    }

    fn literal_block_candidate(source: &str) -> Elements<'_> {
        let mut elements = Vec::new();
        for chunk in source.split_inclusive('\n') {
            let (text, line_break) = match chunk.strip_suffix('\n') {
                Some(text) => (text, true),
                None => (chunk, false),
            };
            if !text.is_empty() {
                elements.push(text!(text));
            }
            if line_break {
                elements.push(Element::LineBreak);
            }
        }
        Elements::Multiple(elements)
    }
}

#[macro_use]
mod align;

mod align_center;
mod align_justify;
mod align_left;
mod align_right;
mod anchor;
mod bibcite;
mod bibliography;
mod blockquote;
mod bold;
mod button;
mod char;
mod checkbox;
mod code;
mod collapsible;
mod date;
mod del;
mod div;
mod embed;
mod embed_video;
mod equation_ref;
mod file;
mod footnote;
mod gallery;
mod hidden;
mod html;
mod ifcategory;
mod iframe;
mod iftags;
mod image;
mod include_elements;
mod include_wikidot;
mod ins;
mod invisible;
mod italics;
mod later;
mod lines;
mod list;
mod mark;
mod math;
mod media;
mod module;
mod monospace;
mod note;
mod paragraph;
mod radio;
mod raw;
mod ruby;
mod size;
mod span;
mod strikethrough;
mod subscript;
mod superscript;
mod table;
mod tabs;
mod target;
mod toc;
mod underline;
mod user;

pub use self::align_center::BLOCK_ALIGN_CENTER;
pub use self::align_justify::BLOCK_ALIGN_JUSTIFY;
pub use self::align_left::BLOCK_ALIGN_LEFT;
pub use self::align_right::BLOCK_ALIGN_RIGHT;
pub use self::anchor::BLOCK_ANCHOR;
pub use self::bibcite::BLOCK_BIBCITE;
pub use self::bibliography::BLOCK_BIBLIOGRAPHY;
pub use self::blockquote::BLOCK_BLOCKQUOTE;
pub use self::bold::BLOCK_BOLD;
pub use self::button::BLOCK_BUTTON;
pub use self::char::BLOCK_CHAR;
pub use self::checkbox::BLOCK_CHECKBOX;
pub use self::code::BLOCK_CODE;
pub use self::collapsible::BLOCK_COLLAPSIBLE;
pub(crate) use self::collapsible::{CollapsibleHead, parse_collapsible_head};
pub use self::date::BLOCK_DATE;
pub use self::del::BLOCK_DEL;
pub use self::div::BLOCK_DIV;
pub use self::embed::BLOCK_EMBED;
pub use self::embed_video::BLOCK_EMBED_VIDEO;
pub use self::equation_ref::BLOCK_EQUATION_REF;
pub use self::file::BLOCK_FILE;
pub use self::footnote::{BLOCK_FOOTNOTE, BLOCK_FOOTNOTE_BLOCK};
pub use self::gallery::BLOCK_GALLERY;
pub use self::hidden::BLOCK_HIDDEN;
pub use self::html::BLOCK_HTML;
pub use self::ifcategory::BLOCK_IFCATEGORY;
pub use self::iframe::BLOCK_IFRAME;
pub use self::iftags::BLOCK_IFTAGS;
pub use self::image::BLOCK_IMAGE;
pub use self::include_elements::BLOCK_INCLUDE_ELEMENTS;
pub use self::include_wikidot::BLOCK_INCLUDE_WIKIDOT;
pub use self::ins::BLOCK_INS;
pub use self::invisible::BLOCK_INVISIBLE;
pub use self::italics::BLOCK_ITALICS;
pub use self::later::BLOCK_LATER;
pub use self::lines::BLOCK_LINES;
pub use self::list::{BLOCK_LI, BLOCK_OL, BLOCK_UL};
pub use self::mark::BLOCK_MARK;
pub use self::math::BLOCK_MATH;
pub(crate) use self::math::wikidot_math_name;
pub use self::media::{BLOCK_AUDIO, BLOCK_VIDEO};
pub use self::module::BLOCK_MODULE;
pub use self::monospace::BLOCK_MONOSPACE;
pub use self::note::BLOCK_NOTE;
pub use self::paragraph::BLOCK_PARAGRAPH;
pub use self::radio::BLOCK_RADIO;
pub use self::raw::BLOCK_RAW;
pub use self::ruby::{BLOCK_RB, BLOCK_RT, BLOCK_RUBY};
pub use self::size::BLOCK_SIZE;
pub use self::span::BLOCK_SPAN;
pub use self::strikethrough::BLOCK_STRIKETHROUGH;
pub use self::subscript::BLOCK_SUBSCRIPT;
pub use self::superscript::BLOCK_SUPERSCRIPT;
pub use self::table::{
    BLOCK_TABLE, BLOCK_TABLE_CELL_HEADER, BLOCK_TABLE_CELL_REGULAR, BLOCK_TABLE_ROW,
};
pub use self::tabs::{BLOCK_TAB, BLOCK_TABVIEW};
pub use self::target::BLOCK_TARGET;
pub use self::toc::BLOCK_TABLE_OF_CONTENTS;
pub use self::underline::BLOCK_UNDERLINE;
pub use self::user::BLOCK_USER;
