/*
 * render/html/element/style.rs
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
use lightningcss::stylesheet::{ParserOptions, PrinterOptions, StyleSheet};
use std::fmt::Debug;

pub fn render_style(ctx: &mut HtmlContext, input_css: &str) {
    let minify = ctx.settings().minify_css;
    if let Some(output_css) =
        build_style_css(input_css, minify, |stylesheet, print_options| {
            stylesheet
                .to_css(print_options)
                .map(|output| output.code)
                .map_err(|error| error.to_string())
        })
    {
        let output_css = escape_style_end_tags(&output_css);
        ctx.html().style().inner(|ctx| {
            ctx.push_raw_str(&output_css);
        });
    }
}

fn escape_style_end_tags(css: &str) -> String {
    let lower = css.to_ascii_lowercase();
    let mut out = String::with_capacity(css.len());
    let mut last = 0;
    let mut search = 0;

    while let Some(offset) = lower[search..].find("</style") {
        let start = search + offset;
        out.push_str(&css[last..start]);
        out.push_str(r#"<\/style"#);
        last = start + "</style".len();
        search = last;
    }

    out.push_str(&css[last..]);
    out
}

fn build_style_css<F>(input_css: &str, minify: bool, print: F) -> Option<String>
where
    F: FnOnce(&StyleSheet<'_, '_>, PrinterOptions) -> Result<String, String>,
{
    let parser_options = ParserOptions {
        error_recovery: true,
        ..Default::default()
    };

    let print_options = PrinterOptions {
        minify,
        ..Default::default()
    };

    debug!("Parsing input CSS ({} bytes)", input_css.len());
    let stylesheet = handle_style_parse_result(
        input_css,
        StyleSheet::parse(input_css, parser_options),
    )?;

    trace!("Rendering CSS into HTML (minify: {minify})");
    match print(&stylesheet, print_options) {
        Ok(output_css) => Some(output_css),
        Err(error) => {
            log_css_output_error(input_css, &stylesheet, &error);
            None
        }
    }
}

fn handle_style_parse_result<T, E>(input_css: &str, result: Result<T, E>) -> Option<T>
where
    E: Debug,
{
    match result {
        Ok(stylesheet) => Some(stylesheet),
        Err(error) => {
            log_css_parse_error(input_css, &error);
            None
        }
    }
}

fn log_css_output_error(input_css: &str, stylesheet: &impl Debug, error: &str) {
    error!("Problem outputting CSS from stylesheet: {error}");
    trace!("Input CSS:\n{input_css}");
    trace!("Parsed stylesheet:\n{stylesheet:#?}");
}

fn log_css_parse_error(input_css: &str, error: &impl Debug) {
    error!("Problem parsing CSS stylesheet: {error:?}");
    trace!("Input CSS:\n{input_css}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_css_helper_covers_printer_success_and_error_paths() {
        let css = build_style_css(
            "body { color: red; }",
            false,
            |stylesheet, print_options| {
                stylesheet
                    .to_css(print_options)
                    .map(|output| output.code)
                    .map_err(|error| error.to_string())
            },
        );
        assert!(css.as_deref().is_some_and(|css| css.contains("color: red")));

        let css = build_style_css(
            "body { color: red; }",
            false,
            |_stylesheet, _print_options| Err("synthetic printer failure".to_owned()),
        );
        assert!(css.is_none());

        assert_eq!(
            handle_style_parse_result("bad css", Err::<(), _>("synthetic parse failure")),
            None,
        );

        assert_eq!(
            escape_style_end_tags(r#"a { content: "</StYlE><script>" }"#),
            r#"a { content: "<\/style><script>" }"#,
        );
    }
}
