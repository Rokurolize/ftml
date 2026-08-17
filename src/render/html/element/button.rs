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
use crate::tree::{
    StandaloneButton, StandaloneButtonAction, TagAlteration,
    is_safe_standalone_button_style,
};

pub fn render_standalone_button(ctx: &mut HtmlContext, button: &StandaloneButton<'_>) {
    let id = ctx.random().generate_standalone_button_id();
    ctx.require_standalone_button(id.clone(), &button.action);

    if ctx.layout().legacy() {
        let class = button.class.as_deref().unwrap_or("wiki-standalone-button");
        let onclick = wikidot_onclick(&button.action);
        let mut anchor = ctx.html().a();
        anchor.attr(attr!(
            "class" => class,
            "href" => "javascript:;",
            "onclick" => &onclick,
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

fn wikidot_onclick(action: &StandaloneButtonAction<'_>) -> String {
    match action {
        StandaloneButtonAction::Edit => {
            "WIKIDOT.page.listeners.editClick(event)".to_owned()
        }
        StandaloneButtonAction::History => {
            "WIKIDOT.page.listeners.historyClick(event)".to_owned()
        }
        StandaloneButtonAction::Source => {
            "WIKIDOT.page.listeners.viewSourceClick(event)".to_owned()
        }
        StandaloneButtonAction::Print => {
            "WIKIDOT.page.listeners.printClick(event)".to_owned()
        }
        StandaloneButtonAction::SetTags(alterations) => {
            let alterations = alterations
                .iter()
                .map(|alteration| match alteration {
                    TagAlteration::Add(tag) => {
                        format!("+{}", escape_javascript_string(tag))
                    }
                    TagAlteration::Remove(tag) => {
                        format!("-{}", escape_javascript_string(tag))
                    }
                    TagAlteration::ClearVisible => "-*".to_owned(),
                    TagAlteration::ClearHidden => "-_*".to_owned(),
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("WIKIDOT.page.listeners.updateTagsByButton(event, '{alterations}')")
        }
    }
}

fn escape_javascript_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push('\\'),
            '\'' => escaped.push_str("\\x27"),
            '\u{2028}' => escaped.push_str("\\u2028"),
            '\u{2029}' => escaped.push_str("\\u2029"),
            character => escaped.push(character),
        }
    }
    escaped
}
