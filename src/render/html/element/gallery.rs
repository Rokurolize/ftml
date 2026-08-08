/*
 * render/html/element/gallery.rs
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
use crate::tree::Gallery;

pub fn render_gallery(ctx: &mut HtmlContext, gallery: &Gallery<'_>) {
    let id = ctx.random().generate_gallery_id();
    ctx.require_gallery(id.clone(), gallery);

    let mut marker = ctx.html().div();
    marker.attr(attr!(
        "class" => "wj-gallery",
        "id" => &id,
    ));
}
