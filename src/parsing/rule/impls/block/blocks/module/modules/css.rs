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
    parse_fn,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    _arguments: Arguments<'t>,
) -> ParseResult<'r, 't, ModuleParseOutput<'t>> {
    debug!("Parsing categories module");
    assert_module_name(&MODULE_CSS, name);

    let css = parser.get_body_text(&BLOCK_MODULE)?;
    let element = Element::Style(cow!(css));
    success_value(element.into(), Vec::new(), false)
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
        assert_eq!(
            output.body,
            "<style>.raw{--literal:\"[[*bold]] [[span]]\"}</style>",
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

        assert_eq!(
            repeated.body,
            format!("{}{}", independent.body, independent.body)
        );
        assert_eq!(
            repeated.styles,
            vec![independent.styles[0].clone(), independent.styles[0].clone()],
        );
    }
}
