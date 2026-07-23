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
    let source = FileSource::File1 {
        file: std::borrow::Cow::Borrowed(file),
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
