/*
 * render/html/element/social.rs
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
use crate::tree::SocialButtons;

pub fn render_social_buttons(ctx: &mut HtmlContext, social: &SocialButtons) {
    let id = ctx.random().generate_social_id();
    ctx.require_social(id.clone(), social);

    let mut marker = ctx.html().span();
    marker.attr(attr!(
        "class" => "wj-social",
        "id" => &id,
    ));
}
