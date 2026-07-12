/*
 * parsing/rule/impls/block/blocks/module/rule.rs
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

use super::mapping::get_module_rule_with_name;
use super::prelude::*;

pub const BLOCK_MODULE: BlockRule = BlockRule {
    name: "block-module",
    accepts_names: &["module", "module654"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: true,
    parse_fn,
};

fn parse_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Parsing module block (in-head {in_head})");
    parser.check_page_syntax()?;
    assert!(!flag_star, "Module doesn't allow star flag");
    assert!(!flag_score, "Module doesn't allow score flag");
    assert_block_name(&BLOCK_MODULE, name);

    if parser.native_blockquote_depth().is_some() {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    // Get module name and arguments
    let (subname, arguments) = parser.get_head_name_map(&BLOCK_MODULE, in_head)?;

    // Get the module rule for this name
    let module_rule = match get_module_rule_with_name(subname) {
        Some(rule) => rule,
        None => return Err(parser.make_err(ParseErrorKind::NoSuchModule)),
    };

    // Prepare to run the module's parsing function
    parser.set_module(module_rule);

    // Run the parse function until the end.
    // This starts after the head and its newline.
    //
    // If the module accepts a body, it should consume it,
    // then the tail. Otherwise it shouldn't move the token pointer.
    let output = (module_rule.parse_fn)(parser, subname, arguments)?;
    let (elements, errors, paragraph_safe) = output.into();

    success_elements_with_paragraph_safety(paragraph_safe, elements, errors)
}

#[cfg(test)]
mod tests {
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn quoted_module_markers_remain_literal_like_wikidot() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        // Quoted CSS module examples occur on theme:minimalist-bhl,
        // theme:jakstyle, and manna-charitable-foundation-hub.
        for input in [
            "> [[module CSS]]\n> .OMEGA_CSS_DEPTH_ONE { color: red; }\n> [[/module]]",
            ">> [[module CSS]]\n>> .OMEGA_CSS_DEPTH_TWO { color: red; }\n>> [[/module]]",
            "> > [[module CSS]]\n> > .OMEGA_CSS_SPACED_INNER { color: red; }\n> > [[/module]]",
            ">> [[module CSS]]\n>> .OMEGA_CSS_SHALLOW_CLOSE { color: red; }\n> [[/module]]\n> OMEGA_AFTER_SHALLOW",
            "> [[module CSS]]\n> .OMEGA_CSS_DEEP_CLOSE { color: red; }\n>> [[/module]]\n> OMEGA_AFTER_DEEP",
            "> [[module CSS]]\n> .OMEGA_CSS_UNCLOSED { color: red; }\n> OMEGA_QUOTED_AFTER_UNCLOSED\nOMEGA_OUTSIDE_AFTER_UNCLOSED",
            "> [[module Rate show=\"OMEGA_RATE_DEPTH_ONE\"]]",
            ">> [[module Rate show=\"OMEGA_RATE_DEPTH_TWO\"]]",
            "> [[module CountPages category=\"OMEGA_COUNT_DEPTH_ONE\"]]",
        ] {
            let tokenization = crate::tokenize(input);
            let (tree, _errors) =
                crate::parse(&tokenization, &page_info, &settings).into();

            let debug = format!("{tree:?}");
            assert!(!debug.contains("Style("), "{input:?}: {debug}");
            assert!(!debug.contains("Module("), "{input:?}: {debug}");
            assert!(debug.contains("Text(\"module\")"), "{input:?}: {debug}");
        }
    }
}
