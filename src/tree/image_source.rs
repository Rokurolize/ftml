/*
 * tree/image_source.rs
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

use super::FileSource;
use super::clone::string_to_owned;
use std::borrow::Cow;

/// The image source and the parser-owned disposition used to render it.
#[derive(Serialize, Deserialize, Debug, Hash, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", tag = "type", content = "data")]
pub enum ImageSource<'t> {
    /// A source that renders directly without an implicit attachment link.
    Direct(FileSource<'t>),

    /// A filename inferred as an attachment on the current page.
    ///
    /// `file` retains the exact attachment filename for caller-side lookup and
    /// authorization. `alt` retains authored spelling when URL materialization
    /// normalizes characters such as backslashes.
    ImplicitAttachment {
        file: Cow<'t, str>,
        alt: Cow<'t, str>,
        size: ImageSize,
    },
}

/// A typed Wikidot image resize variant.
#[derive(Serialize, Deserialize, Debug, Hash, Copy, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ImageSize {
    Medium,
}

impl ImageSize {
    pub(crate) const fn file_name(self) -> &'static str {
        match self {
            ImageSize::Medium => "medium.jpg",
        }
    }
}

impl<'t> ImageSource<'t> {
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            ImageSource::Direct(source) => source.name(),
            ImageSource::ImplicitAttachment { .. } => "implicit-attachment",
        }
    }

    pub fn to_owned(&self) -> ImageSource<'static> {
        match self {
            ImageSource::Direct(source) => ImageSource::Direct(source.to_owned()),
            ImageSource::ImplicitAttachment { file, alt, size } => {
                ImageSource::ImplicitAttachment {
                    file: string_to_owned(file),
                    alt: string_to_owned(alt),
                    size: *size,
                }
            }
        }
    }
}
