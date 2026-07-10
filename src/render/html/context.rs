/*
 * render/html/context.rs
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

use super::builder::HtmlBuilder;
use super::escape::escape;
use super::meta::{HtmlMeta, HtmlMetaType};
use super::output::HtmlOutput;
use super::random::Random;
use crate::data::PageRef;
use crate::data::{Backlinks, PageInfo};
use crate::info;
use crate::layout::Layout;
use crate::next_index::{Incrementer, NextIndex, TableOfContentsIndex};
use crate::render::Handle;
use crate::settings::WikitextSettings;
use crate::tree::{
    Bibliography, BibliographyList, Element, LinkLocation, VariableScopes,
};
use crate::url::{HrefKind, classify_href, normalize_href};
use std::collections::HashMap;
use std::fmt::{self, Write};
use std::num::NonZeroUsize;
use std::ops::Range;

#[derive(Debug)]
pub struct HtmlContext<'i, 'h, 'e, 't>
where
    'e: 't,
{
    body: String,
    meta: Vec<HtmlMeta>,
    styles: Vec<String>,
    backlinks: Backlinks<'static>,
    info: &'i PageInfo<'i>,
    handle: &'h Handle,
    settings: &'e WikitextSettings,
    random: Random,

    //
    // Included page scopes
    //
    variables: VariableScopes,

    //
    // Fields from syntax tree
    //
    table_of_contents: &'e [Element<'t>],
    footnotes: &'e [Vec<Element<'t>>],
    bibliographies: &'e BibliographyList<'t>,

    //
    // Cached data
    //
    pages_exists: HashMap<PageRef, bool>,
    style_css: HashMap<(String, bool), String>,
    table_of_contents_html_range: Option<Range<usize>>,

    //
    // Other fields to track
    //
    code_snippet_index: NonZeroUsize,
    table_of_contents_index: Incrementer,
    equation_index: NonZeroUsize,
    footnote_index: NonZeroUsize,
    bibliography_render_stack: Vec<String>,
}

impl<'i, 'h, 'e, 't> HtmlContext<'i, 'h, 'e, 't> {
    #[inline]
    pub fn new(
        info: &'i PageInfo<'i>,
        handle: &'h Handle,
        settings: &'e WikitextSettings,
        table_of_contents: &'e [Element<'t>],
        footnotes: &'e [Vec<Element<'t>>],
        bibliographies: &'e BibliographyList<'t>,
        wikitext_len: usize,
    ) -> Self {
        // Heuristic for improving rendering performance by avoiding reallocating.
        //
        // Rendered HTML is commonly larger than source wikitext because each
        // syntax element expands into tags and escaped text. Keep the estimate
        // conservative enough to avoid repeated growth on mixed markup pages.
        let capacity = wikitext_len.saturating_mul(4).max(4096);

        // Build and return
        HtmlContext {
            body: String::with_capacity(capacity),
            meta: Self::initial_metadata(info, settings.layout),
            styles: Vec::new(),
            backlinks: Backlinks::new(),
            info,
            handle,
            settings,
            random: Random::default(),
            variables: VariableScopes::new(),
            table_of_contents,
            footnotes,
            bibliographies,
            pages_exists: HashMap::new(),
            style_css: HashMap::new(),
            table_of_contents_html_range: None,
            code_snippet_index: NonZeroUsize::new(1).unwrap(),
            table_of_contents_index: settings.id_indexer(),
            equation_index: NonZeroUsize::new(1).unwrap(),
            footnote_index: NonZeroUsize::new(1).unwrap(),
            bibliography_render_stack: Vec::new(),
        }
    }

    fn initial_metadata(info: &PageInfo<'i>, layout: Layout) -> Vec<HtmlMeta> {
        // Initial version, we can tune how the metadata is generated later.

        let content_type = HtmlMeta {
            tag_type: HtmlMetaType::HttpEquiv,
            name: str!("Content-Type"),
            value: str!("text/html"),
        };
        let generator = HtmlMeta {
            tag_type: HtmlMetaType::Name,
            name: str!("generator"),
            value: format!("{} {}", *info::VERSION, layout.description()),
        };
        let mut description_value = str!(info.title);
        if let Some(ref alt_title) = info.alt_title {
            description_value.push_str(" - ");
            description_value.push_str(alt_title);
        }
        let description = HtmlMeta {
            tag_type: HtmlMetaType::Name,
            name: str!("description"),
            value: description_value,
        };
        let keywords = HtmlMeta {
            tag_type: HtmlMetaType::Name,
            name: str!("keywords"),
            value: info.tags.join(","),
        };

        vec![content_type, generator, description, keywords]
    }

    // Field access
    #[inline]
    pub fn info(&self) -> &PageInfo<'i> {
        self.info
    }

    #[inline]
    pub fn settings(&self) -> &WikitextSettings {
        self.settings
    }

    #[inline]
    pub fn layout(&self) -> Layout {
        self.settings.layout
    }

    #[inline]
    pub fn handle(&self) -> &'h Handle {
        self.handle
    }

    #[inline]
    pub fn random(&mut self) -> &mut Random {
        &mut self.random
    }

    #[inline]
    pub fn language(&self) -> &str {
        &self.info.language
    }

    #[inline]
    pub fn variables(&self) -> &VariableScopes {
        &self.variables
    }

    #[inline]
    pub fn variables_mut(&mut self) -> &mut VariableScopes {
        &mut self.variables
    }

    #[inline]
    pub fn table_of_contents(&self) -> &'e [Element<'t>] {
        self.table_of_contents
    }

    #[inline]
    pub fn footnotes(&self) -> &'e [Vec<Element<'t>>] {
        self.footnotes
    }

    #[inline]
    pub fn get_bibliography(&self, index: usize) -> Option<&'e Bibliography<'t>> {
        self.bibliographies.get_bibliography_opt(index)
    }

    pub fn get_bibliography_ref(
        &self,
        label: &str,
    ) -> Option<(usize, &'e [Element<'t>])> {
        self.bibliographies.get_reference(label)
    }

    pub fn enter_bibliography_ref(&mut self, label: &str) -> bool {
        let render_stack = &self.bibliography_render_stack;
        let already_rendering = render_stack.iter().any(|item| item == label);

        if already_rendering {
            false
        } else {
            self.bibliography_render_stack.push(str!(label));
            true
        }
    }

    pub fn exit_bibliography_ref(&mut self, label: &str) {
        let popped = self.bibliography_render_stack.pop();
        debug_assert_eq!(popped.as_deref(), Some(label));
    }

    pub fn next_code_snippet_index(&mut self) -> NonZeroUsize {
        let next = NonZeroUsize::new(self.code_snippet_index.get() + 1).unwrap();
        std::mem::replace(&mut self.code_snippet_index, next)
    }

    #[inline]
    pub fn next_table_of_contents_index(&mut self) -> Option<usize> {
        self.table_of_contents_index.next()
    }

    pub fn next_equation_index(&mut self) -> NonZeroUsize {
        let next = NonZeroUsize::new(self.equation_index.get() + 1).unwrap();
        std::mem::replace(&mut self.equation_index, next)
    }

    pub fn next_footnote_index(&mut self) -> NonZeroUsize {
        let next = NonZeroUsize::new(self.footnote_index.get() + 1).unwrap();
        std::mem::replace(&mut self.footnote_index, next)
    }

    #[inline]
    pub fn get_footnote(&self, index_one: NonZeroUsize) -> Option<&'e [Element<'t>]> {
        self.footnotes
            .get(usize::from(index_one) - 1)
            .map(|elements| elements.as_slice())
    }

    // Backlinks
    #[inline]
    pub fn add_link(&mut self, link: &LinkLocation) {
        // TODO: set to internal link if domain matches site
        // See https://scuttle.atlassian.net/browse/WJ-24

        match link {
            LinkLocation::Page(page) => {
                self.backlinks.internal_links.push(page.to_owned());
            }
            LinkLocation::Url(link) => {
                let href_kind = classify_href(link);

                match href_kind {
                    HrefKind::NoOp | HrefKind::Invalid | HrefKind::Anchor => {
                        trace!("Ignoring href that does not record a backlink");
                    }
                    HrefKind::External => {
                        let normalized = normalize_href(link, None).into_owned();
                        let external_links = &mut self.backlinks.external_links;
                        external_links.push(normalized.into());
                    }
                    HrefKind::AbsolutePath => {
                        let normalized = normalize_href(link, None);
                        let link = &normalized[1..];
                        if !link.is_empty() {
                            let page_ref = PageRef::page_only(cow!(link));
                            self.backlinks.internal_links.push(page_ref.to_owned());
                        }
                    }
                    HrefKind::Relative => {
                        let link = link.as_ref();
                        if !link.is_empty() {
                            let page_ref = PageRef::page_only(cow!(link));
                            self.backlinks.internal_links.push(page_ref.to_owned());
                        }
                    }
                }
            }
        }
    }

    pub fn page_exists(&mut self, page_ref: &PageRef) -> bool {
        let (site, page, _) = page_ref.fields_or(&self.info.site);

        // Get from cache, or fetch and add
        match self.pages_exists.get(page_ref) {
            Some(exists) => *exists,
            None => {
                let exists = self.handle.get_page_exists(site, page);
                self.pages_exists.insert(page_ref.to_owned(), exists);
                exists
            }
        }
    }

    // TODO
    #[allow(dead_code)]
    #[inline]
    pub fn add_include(&mut self, page: PageRef) {
        self.backlinks.included_pages.push(page.to_owned());
    }

    // Buffer management
    #[inline]
    pub fn buffer(&mut self) -> &mut String {
        &mut self.body
    }

    #[inline]
    pub fn push_raw(&mut self, ch: char) {
        self.buffer().push(ch);
    }

    #[inline]
    pub fn push_raw_str(&mut self, s: &str) {
        self.buffer().push_str(s);
    }

    #[inline]
    pub fn push_escaped(&mut self, s: &str) {
        escape(self.buffer(), s);
    }

    pub fn push_cached_table_of_contents<F>(&mut self, render: F)
    where
        F: FnOnce(&mut Self),
    {
        if let Some(range) = self.table_of_contents_html_range.clone() {
            let table_of_contents_html = self.body[range].to_owned();
            self.push_raw_str(&table_of_contents_html);
        } else {
            let start = self.body.len();
            render(self);
            let end = self.body.len();
            self.table_of_contents_html_range = Some(start..end);
        }
    }

    #[inline]
    pub fn html(&mut self) -> HtmlBuilder<'_, 'i, 'h, 'e, 't> {
        HtmlBuilder::new(self)
    }

    #[inline]
    pub fn add_style(&mut self, css: String) {
        self.styles.push(css);
    }

    pub fn get_cached_style_css(&self, input_css: &str, minify: bool) -> Option<String> {
        self.style_css.get(&(input_css.to_owned(), minify)).cloned()
    }

    pub fn cached_style_css_len(&self) -> usize {
        self.style_css.len()
    }

    pub fn clear_cached_style_css(&mut self) {
        self.style_css.clear();
    }

    pub fn insert_cached_style_css(
        &mut self,
        input_css: String,
        minify: bool,
        output_css: String,
    ) {
        self.style_css.insert((input_css, minify), output_css);
    }
}

impl<'i, 'h, 'e, 't> From<HtmlContext<'i, 'h, 'e, 't>> for HtmlOutput {
    #[inline]
    fn from(ctx: HtmlContext<'i, 'h, 'e, 't>) -> HtmlOutput {
        HtmlOutput {
            body: ctx.body,
            meta: ctx.meta,
            styles: ctx.styles,
            backlinks: ctx.backlinks,
        }
    }
}

impl Write for HtmlContext<'_, '_, '_, '_> {
    #[inline]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.buffer().write_str(s)
    }
}

impl NextIndex<TableOfContentsIndex> for HtmlContext<'_, '_, '_, '_> {
    #[inline]
    fn next(&mut self) -> Option<usize> {
        self.next_table_of_contents_index()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_utils::context;
    use super::*;
    use crate::data::PageInfo;
    use crate::settings::{WikitextMode, WikitextSettings};
    use crate::tree::{Bibliography, BibliographyList, VariableMap};

    #[test]
    fn initial_metadata_includes_alt_title_in_description() {
        let mut info = PageInfo::dummy();
        info.alt_title = Some(cow!("Alternate"));

        let output = HtmlOutput::from(context(&info));
        let description = output
            .meta
            .iter()
            .find(|meta| meta.name == "description")
            .expect("description metadata should be present");

        assert_eq!(description.value, "A page for the age - Alternate");
    }

    #[test]
    fn html_context_add_include_records_backlink() {
        let info = PageInfo::dummy();
        let mut ctx = context(&info);
        let page_ref = PageRef::page_only("component:box");

        ctx.add_include(page_ref.clone());

        let output = HtmlOutput::from(ctx);
        assert_eq!(output.backlinks.included_pages, vec![page_ref]);
        assert!(output.backlinks.internal_links.is_empty());
        assert!(output.backlinks.external_links.is_empty());
        assert!(output.styles.is_empty());
    }

    #[test]
    fn html_context_tracks_indices_and_link_backlinks() {
        let info = PageInfo::dummy();
        let mut ctx = context(&info);

        assert_eq!(ctx.next_code_snippet_index().get(), 1);
        assert_eq!(ctx.next_code_snippet_index().get(), 2);
        assert_eq!(ctx.next_equation_index().get(), 1);
        assert_eq!(ctx.next_equation_index().get(), 2);
        assert_eq!(ctx.next_footnote_index().get(), 1);
        assert_eq!(ctx.next_footnote_index().get(), 2);

        ctx.add_link(&LinkLocation::Url(cow!("javascript:;")));
        ctx.add_link(&LinkLocation::Url(cow!("javascript:alert(1)")));
        ctx.add_link(&LinkLocation::Url(cow!("#local-anchor")));
        ctx.add_link(&LinkLocation::Url(cow!("/")));
        ctx.add_link(&LinkLocation::Url(cow!("/local-page")));
        ctx.add_link(&LinkLocation::Url(cow!("")));
        ctx.add_link(&LinkLocation::Url(cow!("plain-page")));
        ctx.add_link(&LinkLocation::Url(cow!("https://example.com/path")));
        ctx.add_link(&LinkLocation::Url(cow!("//example.com/protocol-relative")));

        let direct_page = PageRef::page_only("direct-page");
        ctx.add_link(&LinkLocation::Page(direct_page.clone()));

        let output = HtmlOutput::from(ctx);
        assert_eq!(
            output.backlinks.internal_links,
            vec![
                PageRef::page_only("local-page"),
                PageRef::page_only("plain-page"),
                direct_page,
            ],
        );
        assert_eq!(
            output.backlinks.external_links,
            vec![
                cow!("https://example.com/path"),
                cow!("//example.com/protocol-relative"),
            ],
        );
        assert!(output.backlinks.included_pages.is_empty());
        assert!(output.styles.is_empty());
    }

    #[test]
    fn html_context_bibliography_render_stack_detects_cycles() {
        let info = PageInfo::dummy();
        let mut ctx = context(&info);

        assert!(ctx.enter_bibliography_ref("alpha"));
        assert!(!ctx.enter_bibliography_ref("alpha"));
        assert!(ctx.enter_bibliography_ref("beta"));

        ctx.exit_bibliography_ref("beta");
        assert!(!ctx.enter_bibliography_ref("alpha"));
        ctx.exit_bibliography_ref("alpha");
        assert!(ctx.enter_bibliography_ref("alpha"));
        ctx.exit_bibliography_ref("alpha");
    }

    #[test]
    fn html_context_accessors_cache_and_buffers() {
        let info = PageInfo::dummy();
        let handle = Handle;
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        let table_of_contents = [Element::Text(cow!("toc"))];
        let footnotes = [vec![Element::Text(cow!("footnote"))]];
        let mut bibliography = Bibliography::new();
        bibliography.add(cow!("alpha"), vec![Element::Text(cow!("reference"))]);
        let mut bibliographies = BibliographyList::new();
        bibliographies.push(bibliography);
        let mut ctx = HtmlContext::new(
            &info,
            &handle,
            &settings,
            &table_of_contents,
            &footnotes,
            &bibliographies,
            10,
        );

        assert_eq!(ctx.info().page, info.page);
        assert_eq!(ctx.settings().layout, Layout::Wikijump);
        assert_eq!(ctx.layout(), Layout::Wikijump);
        assert!(ctx.handle().get_page_exists(&info.site, "present"));
        assert_eq!(ctx.language(), info.language);
        assert_eq!(ctx.table_of_contents(), &table_of_contents);
        assert_eq!(ctx.footnotes(), &footnotes);
        assert_eq!(
            ctx.get_footnote(NonZeroUsize::new(1).unwrap()),
            Some(footnotes[0].as_slice()),
        );
        assert_eq!(ctx.get_footnote(NonZeroUsize::new(2).unwrap()), None);
        assert_eq!(ctx.get_bibliography(0).unwrap().slice().len(), 1);
        assert!(ctx.get_bibliography(1).is_none());
        assert_eq!(ctx.get_bibliography_ref("alpha").unwrap().0, 1);

        let mut variables = VariableMap::new();
        variables.insert(cow!("name"), cow!("value"));
        ctx.variables_mut().push_scope(&variables);
        assert_eq!(ctx.variables().get("name"), Some("value"));
        ctx.variables_mut().pop_scope();

        let missing = PageRef::page_only("missing");
        assert!(!ctx.page_exists(&missing));
        assert!(!ctx.page_exists(&missing));
        let present = PageRef::page_only("present");
        assert!(ctx.page_exists(&present));

        write!(ctx, "raw").expect("writing to HTML context should succeed");
        ctx.push_raw('!');
        ctx.push_raw_str(" ");
        ctx.push_escaped("<tag>");
        ctx.add_style(".collected{color:red}".to_owned());

        let output = HtmlOutput::from(ctx);
        assert_eq!(output.body, "raw! &lt;tag&gt;");
        assert_eq!(output.styles, vec![".collected{color:red}".to_owned()]);
    }

    #[test]
    fn html_context_replays_cached_table_of_contents_inner_html() {
        let info = PageInfo::dummy();
        let mut ctx = context(&info);

        ctx.push_raw_str("before");
        ctx.push_cached_table_of_contents(|ctx| ctx.push_raw_str("<ul><li>A</li></ul>"));
        ctx.push_raw_str("middle");
        ctx.push_cached_table_of_contents(|ctx| ctx.push_raw_str("different"));
        ctx.push_raw_str("after");

        let output = HtmlOutput::from(ctx);
        assert_eq!(
            output.body,
            "before<ul><li>A</li></ul>middle<ul><li>A</li></ul>after"
        );
    }
}
