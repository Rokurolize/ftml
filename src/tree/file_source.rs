/*
 * tree/file_source.rs
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

use super::clone::string_to_owned;
use crate::url::is_url;
use std::borrow::Cow;
use strum_macros::IntoStaticStr;

#[derive(Serialize, Deserialize, IntoStaticStr, Debug, Hash, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", tag = "type", content = "data")]
pub enum FileSource<'a> {
    /// File is sourced from an arbitrary URL.
    Url(Cow<'a, str>),

    /// File is attached the current page.
    File1 { file: Cow<'a, str> },

    /// File is attached to another page on the site.
    File2 {
        page: Cow<'a, str>,
        file: Cow<'a, str>,
    },

    /// File is attached to another page on another site.
    File3 {
        site: Cow<'a, str>,
        page: Cow<'a, str>,
        file: Cow<'a, str>,
    },
}

impl<'t> FileSource<'t> {
    pub fn parse(source: &'t str) -> Option<FileSource<'t>> {
        if is_url(source) || source.starts_with("/local--files/") {
            return Some(FileSource::Url(cow!(source)));
        }

        // Strip leading / if present
        let source = source.strip_prefix('/').unwrap_or(source);

        // Get parts for path
        let parts: Vec<&str> = source.split('/').collect();

        // Depending on the number of parts, determine the file variant
        let source = match parts.len() {
            1 => FileSource::File1 {
                file: cow!(parts[0]),
            },
            2 => FileSource::File2 {
                page: cow!(parts[0]),
                file: cow!(parts[1]),
            },
            3 => FileSource::File3 {
                site: cow!(parts[0]),
                page: cow!(parts[1]),
                file: cow!(parts[2]),
            },
            _ => return None,
        };

        Some(source)
    }

    #[inline]
    pub fn name(&self) -> &'static str {
        self.into()
    }

    pub fn to_owned(&self) -> FileSource<'static> {
        match self {
            FileSource::Url(url) => FileSource::Url(string_to_owned(url)),
            FileSource::File1 { file } => FileSource::File1 {
                file: string_to_owned(file),
            },
            FileSource::File2 { page, file } => FileSource::File2 {
                page: string_to_owned(page),
                file: string_to_owned(file),
            },
            FileSource::File3 { site, page, file } => FileSource::File3 {
                site: string_to_owned(site),
                page: string_to_owned(page),
                file: string_to_owned(file),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_file_source_variants() {
        assert_eq!(
            FileSource::parse("https://example.com/image.png"),
            Some(FileSource::Url(cow!("https://example.com/image.png"))),
        );
        assert_eq!(
            FileSource::parse("/image.png"),
            Some(FileSource::File1 {
                file: cow!("image.png"),
            }),
        );
        assert_eq!(
            FileSource::parse("/page/image.png"),
            Some(FileSource::File2 {
                page: cow!("page"),
                file: cow!("image.png"),
            }),
        );
        assert_eq!(
            FileSource::parse("/site/page/image.png"),
            Some(FileSource::File3 {
                site: cow!("site"),
                page: cow!("page"),
                file: cow!("image.png"),
            }),
        );
    }

    #[test]
    fn parses_canonical_wikidot_local_files_path_as_literal_url() {
        assert_eq!(
            FileSource::parse("/local--files/page/assets/charts/image.png"),
            Some(FileSource::Url(cow!(
                "/local--files/page/assets/charts/image.png"
            ))),
        );
    }

    #[test]
    fn rejects_unknown_deeper_path_shapes() {
        assert_eq!(FileSource::parse("site/page/assets/image.png"), None);
    }
}
