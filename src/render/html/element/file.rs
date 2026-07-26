/*
 * render/html/element/file.rs
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
use crate::tree::FileSource;

pub fn render_file_link(ctx: &mut HtmlContext, file: &str, label: &str) {
    let source = if ctx.layout().legacy() {
        parse_wikidot_file_link_source(file)
    } else {
        Some(FileSource::File1 {
            file: std::borrow::Cow::Borrowed(file),
        })
    };
    let Some(source) = source else {
        ctx.push_escaped(label);
        return;
    };
    let Some(url) = ctx
        .handle()
        .get_file_link(&source, ctx.info(), ctx.settings())
    else {
        ctx.push_escaped(label);
        return;
    };

    let layout = ctx.layout();
    let mut anchor = ctx.html().a();
    match layout {
        Layout::Wikidot => anchor.attr(attr!("href" => &url)),
        Layout::Wikijump => anchor.attr(attr!(
            "class" => "wj-link wj-link-internal",
            "data-link-type" => "file",
            "href" => &url,
        )),
    };
    anchor.contents(label);
}

fn parse_wikidot_file_link_source(file: &str) -> Option<FileSource<'_>> {
    if file.split('/').any(|part| matches!(part, "." | "..")) {
        return None;
    }
    FileSource::parse_wikidot(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wikidot_file_link_source_rejects_path_traversal() {
        assert_eq!(parse_wikidot_file_link_source("../elements.tsv"), None);
        assert_eq!(parse_wikidot_file_link_source("page/../elements.tsv"), None);
        assert_eq!(
            parse_wikidot_file_link_source("other-page/elements.tsv"),
            Some(FileSource::File2 {
                page: std::borrow::Cow::Borrowed("other-page"),
                file: std::borrow::Cow::Borrowed("elements.tsv"),
            }),
        );
    }
}
