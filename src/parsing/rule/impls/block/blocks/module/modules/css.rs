/*
 * parsing/rule/impls/block/blocks/module/modules/css.rs
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

pub const MODULE_CSS: ModuleRule = ModuleRule {
    name: "module-css",
    accepts_names: &["CSS"],
    parse_fn: parse_with_wikidot_boundary,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    _arguments: Arguments<'t>,
) -> ParseResult<'r, 't, ModuleParseOutput<'t>> {
    debug!("Parsing categories module");
    assert_module_name(&MODULE_CSS, name);

    if parser.settings().layout.legacy() && parser.current().token == Token::InputEnd {
        return ok!(false; ModuleParseOutput::None);
    }
    let css = parser.get_body_text(&BLOCK_MODULE)?;
    let element = Element::Style(css);
    success_value(element.into(), Vec::new(), false)
}

fn parse_with_wikidot_boundary<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    arguments: Arguments<'t>,
) -> ParseResult<'r, 't, ModuleParseOutput<'t>> {
    assert_module_name(&MODULE_CSS, name);

    if parser.settings().layout.legacy() {
        let mut body_probe = parser.clone();
        if wikidot_css_body_starts_on_new_line(parser)
            && let Ok(body) = body_probe.get_body_text(&BLOCK_MODULE)
            && !wikidot_list_pages_head_offsets(&body).is_empty()
        {
            match wikidot_css_list_pages_boundary(parser) {
                Some(CssListPagesBoundary::Unclosed) => {
                    return ok!(false; ModuleParseOutput::None);
                }
                Some(CssListPagesBoundary::Closed { nested_end_blocks }) => {
                    let css = parser.get_body_text_after_skipping_end_blocks(
                        &BLOCK_MODULE,
                        nested_end_blocks,
                    )?;
                    return success_value(Element::Style(css).into(), Vec::new(), false);
                }
                None => {}
            }
        }
    }

    parse_fn(parser, name, arguments)
}

fn wikidot_css_body_starts_on_new_line(parser: &Parser<'_, '_>) -> bool {
    let source = parser.full_text().inner();
    let source_before_body = &source[..parser.current().span.start];
    let Some(opener_close) = source_before_body.rfind("]]") else {
        return false;
    };
    source[opener_close + 2..].starts_with(['\r', '\n'])
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CssListPagesBoundary {
    Unclosed,
    Closed { nested_end_blocks: usize },
}

fn wikidot_css_list_pages_boundary<'r, 't>(
    parser: &Parser<'r, 't>,
) -> Option<CssListPagesBoundary>
where
    'r: 't,
{
    let mut body_probe = parser.clone();
    let mut first = true;
    let mut open_list_pages = 0;
    let mut nested_end_blocks = 0;

    loop {
        if wikidot_parser_starts_list_pages(&body_probe) {
            open_list_pages += 1;
        }

        if body_probe.consume_body_end_block(first, &BLOCK_MODULE) {
            if open_list_pages > 0 {
                open_list_pages -= 1;
                nested_end_blocks += 1;
                first = false;
                continue;
            }
            return Some(CssListPagesBoundary::Closed { nested_end_blocks });
        }

        if body_probe.step().is_err() {
            return (open_list_pages == 0 && nested_end_blocks > 0)
                .then_some(CssListPagesBoundary::Unclosed);
        }
        first = false;
    }
}

fn wikidot_parser_starts_list_pages(parser: &Parser<'_, '_>) -> bool {
    let source = parser.full_text().inner();
    let start = parser.current().span.start;
    if start > 0 && !source[..start].ends_with('\n') {
        return false;
    }
    let line = source[start..]
        .split_once('\n')
        .map_or(&source[start..], |(line, _)| line);
    wikidot_line_starts_list_pages(line)
}

fn wikidot_list_pages_head_offsets(body: &str) -> Vec<usize> {
    let mut offset = 0;
    let mut offsets = Vec::new();

    for line in body.split_inclusive('\n') {
        if wikidot_line_starts_list_pages(line) {
            offsets.push(offset);
        }
        offset += line.len();
    }

    offsets
}

fn wikidot_line_starts_list_pages(line: &str) -> bool {
    let Some(head) = line.get(.."[[module".len()) else {
        return false;
    };
    if !head.eq_ignore_ascii_case("[[module") {
        return false;
    }
    let head = &line["[[module".len()..];
    if !head.starts_with([' ', '\t']) {
        return false;
    }
    let head = head.trim_start_matches([' ', '\t']);
    let Some(name) = head.get(.."listpages".len()) else {
        return false;
    };
    if !name.eq_ignore_ascii_case("listpages") {
        return false;
    }
    let rest = &head["listpages".len()..];
    rest.starts_with([']', ' ', '\t'])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::Render;
    use crate::render::html::HtmlRender;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn css_module_body_stays_raw_and_disable_argument_is_ignored() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        let tokenization = crate::tokenize(
            "[[module CSS show=\"head\" disable=\"true\"]]\n.raw { --literal: \"[[*bold]] [[span]]\"; }\n[[/module]]",
        );
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(
            tree.elements,
            vec![Element::Style(cow!(
                ".raw { --literal: \"[[*bold]] [[span]]\"; }"
            ))],
        );

        let output = HtmlRender.render(&tree, &page_info, &settings);
        assert_eq!(output.body, "");
        assert_eq!(
            output.styles,
            vec![".raw{--literal:\"[[*bold]] [[span]]\"}".to_owned()],
        );
    }

    #[test]
    fn repeated_css_module_body_renders_like_independent_modules() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        let module = "[[module CSS]]\n.same { color: red; }\n[[/module]]";
        let repeated_source = format!("{module}\n{module}");

        let tokenization = crate::tokenize(&repeated_source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        assert!(errors.is_empty(), "{errors:?}");
        let repeated = HtmlRender.render(&tree, &page_info, &settings);

        let tokenization = crate::tokenize(module);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        assert!(errors.is_empty(), "{errors:?}");
        let independent = HtmlRender.render(&tree, &page_info, &settings);

        assert_eq!(repeated.body, independent.body);
        assert_eq!(
            repeated.styles,
            vec![independent.styles[0].clone(), independent.styles[0].clone()],
        );
    }

    #[test]
    fn wikidot_unclosed_empty_css_module_is_not_displayed() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize("[[module CSS]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let output = HtmlRender.render(&tree, &page_info, &settings);

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(tree.elements.is_empty());
        assert!(output.body.is_empty());
        assert!(output.styles.is_empty());
    }

    #[test]
    fn wikidot_unclosed_css_module_does_not_consume_following_blocks() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(concat!(
            "[[module CSS]]\n\n",
            "[[module ListPages name=\"=\"]]\n",
            "%%title%%\n",
            "[[/module]]",
        ));
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let output = HtmlRender.render(&tree, &page_info, &settings);

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            tree.elements,
            vec![Element::Module(crate::tree::Module::ListPages {
                arguments: vec![crate::tree::RawModuleArgument {
                    name: cow!("name"),
                    value: cow!("="),
                }],
                body: cow!("%%title%%"),
            })],
        );
        assert!(output.styles.is_empty());
    }

    #[test]
    fn wikidot_unclosed_css_yields_to_list_pages_after_a_css_comment_open() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(concat!(
            "[[module CSS]]\n",
            "/*\n",
            "[[module ListPages name=\"=\"]]\n",
            "*/\n",
            "[[/module]]",
        ));
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let output = HtmlRender.render(&tree, &page_info, &settings);

        // Anonymous Wikidot PagePreview executes this exact shape. Its
        // provenance-backed raw HTML SHA-256 is
        // 1c6db6ed75b7c27d758433d4a7157af459157d4e183c342997bef5366d093853.
        assert!(errors.is_empty(), "{errors:#?}");
        assert!(tree.elements.iter().any(|element| {
            matches!(
                element,
                Element::Module(crate::tree::Module::ListPages { body, .. })
                    if body == "*/"
            )
        }));
        assert!(output.styles.is_empty());
    }

    #[test]
    fn wikidot_unclosed_css_yields_to_multiple_complete_list_pages_modules() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(concat!(
            "[[module CSS]]\n",
            "[[module ListPages name=\"missing-one\"]]\n",
            "ONE\n",
            "[[/module]]\n",
            "[[module ListPages name=\"missing-two\"]]\n",
            "TWO\n",
            "[[/module]]",
        ));
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let output = HtmlRender.render(&tree, &page_info, &settings);

        // Anonymous Wikidot PagePreview emits two ListPages wrappers. Its
        // provenance-backed raw HTML SHA-256 is
        // 48a80a2dd6ca8c27d1813e82ed7e0f9a1700214d34095cb81fdefda88344365f.
        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(
            tree.elements
                .iter()
                .filter(|element| {
                    matches!(
                        element,
                        Element::Module(crate::tree::Module::ListPages { .. })
                    )
                })
                .count(),
            2,
        );
        assert!(output.styles.is_empty());
    }

    #[test]
    fn wikidot_unclosed_css_does_not_consume_a_live_malformed_list_pages_head() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(concat!(
            "[[module CSS]]\n",
            "[[module ListPages name=\"=\n",
            "%%title%%\n",
            "[[/module]]",
        ));
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let output = HtmlRender.render(&tree, &page_info, &settings);

        // Anonymous Wikidot PagePreview executes this as the default
        // ListPages query instead of treating it as CSS. The raw HTML
        // SHA-256 is
        // 5884e46ecf111e5e587c9cd0a911fa7bbeefd31b1a9f74021c14141a71f8d1fd.
        assert!(!errors.is_empty());
        assert!(
            !tree
                .elements
                .iter()
                .any(|element| matches!(element, Element::Style(_)))
        );
        assert!(output.body.contains("[[module ListPages"));
        assert!(output.styles.is_empty());
    }

    #[test]
    fn wikidot_closed_css_keeps_nested_list_pages_markers_raw() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(concat!(
            "[[module CSS]]\n",
            "[[module ListPages name=\"missing-one\"]]\n",
            "ONE\n",
            "[[/module]]\n",
            "[[module ListPages name=\"missing-two\"]]\n",
            "TWO\n",
            "[[/module]]\n",
            "[[/module]]",
        ));
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let output = HtmlRender.render(&tree, &page_info, &settings);

        // The matching anonymous Wikidot PagePreview is empty; no ListPages
        // wrapper executes. Its raw HTML SHA-256 is
        // 545c38b0922de19734fbffde62792c37c2aef6a3216cfa472449173165220f7d.
        assert!(errors.is_empty(), "{errors:#?}");
        assert!(matches!(tree.elements.as_slice(), [Element::Style(_)]));
        assert!(!format!("{tree:?}").contains("Module(ListPages"));
        assert!(output.body.is_empty());
    }

    #[test]
    fn wikidot_closed_css_module_keeps_following_list_pages_independent() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(concat!(
            "[[module CSS]]\n",
            ".closed { color: red; }\n",
            "[[/module]]\n\n",
            "[[module ListPages name=\"=\"]]\n",
            "%%title%%\n",
            "[[/module]]",
        ));
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let output = HtmlRender.render(&tree, &page_info, &settings);

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(matches!(
            tree.elements.as_slice(),
            [
                Element::Style(_),
                Element::Module(crate::tree::Module::ListPages { .. })
            ],
        ));
        assert_eq!(output.styles, [".closed{color:red}"]);
    }

    #[test]
    fn wikidot_unclosed_css_recovery_requires_a_root_list_pages_head() {
        for body in ["[[module ListPages]]", "[[MODULE \t LISTPAGES name=\"=\"]]"] {
            assert_eq!(wikidot_list_pages_head_offsets(body), [0], "{body:?}");
        }
        for body in [
            " [[module ListPages]]",
            "\t[[module ListPages]]",
            "> [[module ListPages]]",
            "text [[module ListPages]]",
            "[[moduleListPages]]",
            "[[module ListPagesExample]]",
        ] {
            assert!(wikidot_list_pages_head_offsets(body).is_empty(), "{body:?}");
        }
    }

    #[test]
    fn wikidot_inline_css_preserves_raw_body_without_nested_module_execution() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        for (source, body) in [
            (
                "[[module CSS]].inline { color: red; }[[/module]]",
                ".inline { color: red; }",
            ),
            (
                "[[module CSS]][[module ListPages name=\"=\"]]%%title%%[[/module]]",
                "[[module ListPages name=\"=\"]]%%title%%",
            ),
        ] {
            let tokenization = crate::tokenize(source);
            let (tree, errors) =
                crate::parse(&tokenization, &page_info, &settings).into();
            let output = HtmlRender.render(&tree, &page_info, &settings);

            assert!(errors.is_empty(), "{source:?}: {errors:#?}");
            assert_eq!(tree.elements, [Element::Style(cow!(body))], "{source:?}");
            assert!(output.body.is_empty(), "{source:?}");
        }
    }

    #[test]
    fn wikijump_unclosed_empty_css_module_remains_literal() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
        let tokenization = crate::tokenize("[[module CSS]]");
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let output = HtmlRender.render(&tree, &page_info, &settings);

        assert!(!errors.is_empty());
        assert_eq!(output.body, "<p>[[module CSS]]</p>");
        assert!(output.styles.is_empty());
    }
}
