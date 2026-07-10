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

#[test]
fn null_includer_returns_no_pages_for_empty_include_list() {
    let mut includer = NullIncluder;
    let includes: Vec<IncludeRef<'static>> = Vec::new();

    let pages = includer
        .include_pages(&includes)
        .expect("null includer should not fail");

    assert!(pages.is_empty());
}

#[test]
fn null_includer_returns_missing_pages_for_non_empty_include_lists() {
    let mut includer = NullIncluder;
    let page_ref = PageRef::page_only("component:example");
    let includes = vec![IncludeRef::page_only(page_ref.clone())];

    let pages = includer
        .include_pages(&includes)
        .expect("null includer should not fail");

    assert_eq!(
        pages,
        vec![FetchedPage {
            page_ref,
            content: None,
        }],
    );
}

#[test]
fn null_includer_removes_include_blocks() {
    let input = "Before
[[include component:example]]
after";
    let settings = crate::settings::WikitextSettings::from_mode(
        crate::settings::WikitextMode::Page,
        crate::layout::Layout::Wikidot,
    );

    let (output, pages) =
        crate::includes::include(input, &settings, NullIncluder, || {
            unreachable!("null includer should return one page for each include")
        })
        .unwrap_or_else(|error| match error {});

    assert_eq!(
        output,
        "Before

after"
    );
    assert_eq!(pages, vec![PageRef::page_only("component:example")]);
}

#[test]
fn null_includer_renders_missing_includes_as_empty_borrowed_text() {
    let mut includer = NullIncluder;
    let page_ref = PageRef::page_only("missing");

    let replacement = includer
        .no_such_include(&page_ref)
        .expect("null includer should not fail");

    assert_eq!(replacement, Cow::Borrowed(""));
}
