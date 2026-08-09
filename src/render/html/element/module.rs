/*
 * render/html/element/module.rs
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
use crate::tree::Module;

pub fn render_module(ctx: &mut HtmlContext, module: &Module<'_>) {
    match (ctx.layout(), module) {
        (Layout::Wikidot, Module::Runtime { name, .. }) => {
            ctx.html()
                .div()
                .attr(attr!("class" => "error-block"))
                .inner(|ctx| {
                    ctx.push_escaped("[[module ");
                    ctx.html().em().contents(name);
                    ctx.push_escaped("]] No such module, please ");
                    ctx.html()
                        .a()
                        .attr(attr!(
                            "href" => "https://www.wikidot.com/doc:modules",
                            "target" => "_blank",
                            "rel" => "noopener noreferrer",
                        ))
                        .contents("check available modules");
                    ctx.push_escaped(" and fix this page.");
                });
        }
        _ => ctx.handle().render_module(ctx.buffer(), module),
    }
}
