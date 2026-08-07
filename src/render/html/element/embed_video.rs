/*
 * render/html/element/embed_video.rs
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
use crate::tree::EmbedVideo;

pub fn render_embed_video(ctx: &mut HtmlContext, embed_video: &EmbedVideo<'_>) {
    let id = ctx.random().generate_embed_video_id();
    ctx.require_embed_video(id.clone(), embed_video);

    let mut marker = ctx.html().div();
    marker.attr(attr!(
        "class" => "wj-embed-video",
        "id" => &id,
    ));
}
