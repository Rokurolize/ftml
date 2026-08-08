/*
 * render/html/element/button.rs
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
use crate::tree::{StandaloneButton, is_safe_standalone_button_style};

pub fn render_standalone_button(ctx: &mut HtmlContext, button: &StandaloneButton<'_>) {
    let id = ctx.random().generate_standalone_button_id();
    ctx.require_standalone_button(id.clone(), &button.action);

    if ctx.layout().legacy() {
        let class = button.class.as_deref().unwrap_or("wiki-standalone-button");
        let mut anchor = ctx.html().a();
        anchor.attr(attr!(
            "class" => class,
            "id" => &id,
            "href" => "javascript:;",
        ));
        if let Some(style) = &button.style
            && is_safe_standalone_button_style(style)
        {
            anchor.attr(attr!("style" => style));
        }
        anchor.contents(&button.label);
        return;
    }

    let class = match &button.class {
        Some(class) => format!("wj-standalone-button {class}"),
        None => "wj-standalone-button".to_owned(),
    };
    let mut control = ctx.html().tag("button");
    control.attr(attr!(
        "type" => "button",
        "class" => &class,
        "id" => &id,
    ));
    if let Some(style) = &button.style
        && is_safe_standalone_button_style(style)
    {
        control.attr(attr!("style" => style));
    }
    control.contents(&button.label);
}
