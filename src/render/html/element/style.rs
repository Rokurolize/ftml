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
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;

const MAX_CACHED_STYLE_ENTRIES: usize = 32;
const MAX_CACHED_STYLE_BYTES: usize = 64 * 1024;

#[derive(Debug, Hash, PartialEq, Eq)]
struct StyleCacheKey {
    input_css: String,
    minify: bool,
}

thread_local! {
    static STYLE_CSS_CACHE: RefCell<HashMap<StyleCacheKey, String>> =
        RefCell::new(HashMap::new());
}

pub fn render_style(ctx: &mut HtmlContext, input_css: &str) {
    let minify = ctx.settings().minify_css;
    if let Some(output_css) = cached_style_css(input_css, minify) {
        ctx.add_style(output_css.clone());
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

fn cached_style_css(input_css: &str, minify: bool) -> Option<String> {
    if input_css.len() > MAX_CACHED_STYLE_BYTES {
        return render_uncached_style_css(input_css, minify);
    }

    let key = StyleCacheKey {
        input_css: input_css.to_owned(),
        minify,
    };

    if let Some(output_css) =
        STYLE_CSS_CACHE.with(|cache| cache.borrow().get(&key).cloned())
    {
        return Some(output_css);
    }

    let output_css = render_uncached_style_css(input_css, minify)?;
    STYLE_CSS_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= MAX_CACHED_STYLE_ENTRIES {
            cache.clear();
        }
        cache.insert(key, output_css.clone());
    });
    Some(output_css)
}

fn render_uncached_style_css(input_css: &str, minify: bool) -> Option<String> {
    build_style_css(input_css, minify, |stylesheet, print_options| {
        stylesheet
            .to_css(print_options)
            .map(|output| output.code)
            .map_err(|error| error.to_string())
    })
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
fn clear_style_css_cache() {
    STYLE_CSS_CACHE.with(|cache| cache.borrow_mut().clear());
}

#[cfg(test)]
fn style_css_cache_len() -> usize {
    STYLE_CSS_CACHE.with(|cache| cache.borrow().len())
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

    #[test]
    fn style_css_cache_reuses_small_successes_and_skips_large_inputs() {
        clear_style_css_cache();

        let input = "body { color: red; }";
        assert_eq!(
            cached_style_css(input, true).as_deref(),
            Some("body{color:red}")
        );
        assert_eq!(style_css_cache_len(), 1);
        assert_eq!(
            cached_style_css(input, true).as_deref(),
            Some("body{color:red}")
        );
        assert_eq!(style_css_cache_len(), 1);
        assert!(
            cached_style_css(input, false)
                .as_deref()
                .is_some_and(|css| css.contains("color: red"))
        );
        assert_eq!(style_css_cache_len(), 2);

        let large_input =
            format!("/* {} */ body {{ color: red; }}", "x".repeat(64 * 1024));
        assert!(cached_style_css(&large_input, true).is_some());
        assert_eq!(style_css_cache_len(), 2);

        clear_style_css_cache();
    }

    #[test]
    fn style_css_cache_clears_when_full() {
        clear_style_css_cache();

        for index in 0..MAX_CACHED_STYLE_ENTRIES {
            let input = format!(".rule-{index} {{ color: blue; }}");
            assert!(cached_style_css(&input, true).is_some());
        }
        assert_eq!(style_css_cache_len(), MAX_CACHED_STYLE_ENTRIES);

        assert!(cached_style_css("body { color: green; }", true).is_some());
        assert_eq!(style_css_cache_len(), 1);

        clear_style_css_cache();
    }
}
