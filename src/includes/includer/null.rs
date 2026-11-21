/*
 * includes/includer/null.rs
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
use std::convert::Infallible;

/// An [`Includer`] that replaces include blocks with nothing.
#[derive(Debug)]
pub struct NullIncluder;

impl<'t> Includer<'t> for NullIncluder {
    type Error = Infallible;

    #[inline]
    fn include_pages(
        &mut self,
        includes: &[IncludeRef<'t>],
    ) -> Result<Vec<FetchedPage<'t>>, Infallible> {
        Ok(includes
            .iter()
            .map(|include| FetchedPage {
                page_ref: include.page_ref().clone(),
                content: None,
            })
            .collect())
    }

    #[inline]
    fn no_such_include(
        &mut self,
        _page_ref: &PageRef,
    ) -> Result<Cow<'t, str>, Infallible> {
        Ok(Cow::Borrowed(""))
    }
}

#[cfg(test)]
mod r11_tests {
    use super::NullIncluder;
    use crate::data::{PageInfo, ScoreValue};
    use crate::includes::Includer;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};
    use std::borrow::Cow;

    #[test]
    fn null_includer_handles_include_blocks() {
        let info = PageInfo {
            page: Cow::Borrowed("r11"),
            category: None,
            site: Cow::Borrowed("s"),
            title: Cow::Borrowed("t"),
            alt_title: None,
            score: ScoreValue::Integer(0),
            tags: vec![],
            language: Cow::Borrowed("default"),
        };
        let mut settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        settings.enable_page_syntax = true;
        let (text, _) = crate::include(
            "Before\n[[include component:example]]\nafter",
            &settings,
            NullIncluder,
            || panic!("invalid include return"),
        )
        .expect("NullIncluder must handle include blocks without an invalid-return error");
        assert_eq!(text.trim(), "Before\n\nafter");
    }
}
