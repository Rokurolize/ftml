/*
 * parsing/rule/impls/block/rule.rs
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

use super::super::prelude::*;
use super::mapping::{get_block_rule_with_name, get_block_rule_with_name_for_layout};
use super::{BlockRule, blocks::BLOCK_HTML};
use crate::settings::WikitextMode;

pub const RULE_BLOCK: Rule = Rule {
    name: "block",
    position: LineRequirement::Any,
    try_consume_fn: block_regular,
};

pub const RULE_BLOCK_STAR: Rule = Rule {
    name: "block-star",
    position: LineRequirement::Any,
    try_consume_fn: block_star,
};

pub const RULE_BLOCK_SKIP_NEWLINE: Rule = Rule {
    name: "block-skip",
    position: LineRequirement::Any, // this rule happens *on* a newline, not after one
    try_consume_fn: block_skip,
};

// Rule implementations

fn block_regular<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    parse_block(parser, false)
}

fn block_star<'r, 't>(parser: &mut Parser<'r, 't>) -> ParseResult<'r, 't, Elements<'t>> {
    parse_block(parser, true)
}

fn block_skip<'r, 't>(parser: &mut Parser<'r, 't>) -> ParseResult<'r, 't, Elements<'t>> {
    let current = parser.step()?;
    let flag_star = current.token == Token::LeftBlockStar;

    // See if there's a block upcoming
    let result = parser.evaluate_fn(|parser| {
        // Make sure this is the start of a block
        if ![Token::LeftBlock, Token::LeftBlockStar].contains(&current.token) {
            return Ok(false);
        }

        // Get the block's name
        let (name, _) = parser.get_block_name(false)?;

        // Get the block rule: if it accepts newlines, then we consume here
        let block = if parser.discarding_hidden_body() {
            get_block_rule_with_name(name)
        } else {
            get_block_rule_with_name_for_layout(name, parser.settings().layout)
        };
        match block {
            Some(block_rule) => {
                Ok(block_rule.accepts_newlines && block_rule_enabled(parser, block_rule))
            }
            None => Ok(false),
        }
    });

    if result {
        let success = parse_block(parser, flag_star)?;
        if success.paragraph_safe && wikidot_literal_css_module(&success.item) {
            Err(parser.make_err(ParseErrorKind::RuleFailed))
        } else {
            Ok(success)
        }
    } else {
        Err(parser.make_err(ParseErrorKind::RuleFailed))
    }
}

fn wikidot_literal_css_module(elements: &Elements<'_>) -> bool {
    let Elements::Single(Element::Text(text)) = elements else {
        return false;
    };
    let Some(prefix) = text.get(.."[[module CSS".len()) else {
        return false;
    };
    prefix.eq_ignore_ascii_case("[[module CSS")
}

fn block_rule_enabled(parser: &Parser<'_, '_>, block_rule: &BlockRule) -> bool {
    // Preview callers disable hosted HTML execution, but the block still owns
    // its complete body so nested module syntax remains literal as Wikidot
    // renders it. `BLOCK_HTML::parse_fn` performs the escaped-literal branch.
    block_rule.name != BLOCK_HTML.name
        || parser.settings().enable_html_blocks
        || parser.settings().layout.legacy()
}

// Block parsing implementation

fn parse_block<'r, 't>(
    parser: &mut Parser<'r, 't>,
    flag_star: bool,
) -> ParseResult<'r, 't, Elements<'t>>
where
    'r: 't,
{
    let parent_rule = parser.rule();
    let opener_start = parser.current().span.start;

    // Set general rule based on presence of star flag
    parser.set_rule(if flag_star {
        RULE_BLOCK_STAR
    } else {
        RULE_BLOCK
    });

    // Get block name
    let spaced_name = parser
        .look_ahead(0)
        .is_some_and(|token| token.token == Token::Whitespace);
    parser.get_optional_space()?;

    let (name, in_head) = parser.get_block_name(flag_star)?;

    let (name, flag_score) = match name.strip_suffix('_') {
        Some(name) => (name, true),
        None => (name, false),
    };

    // Get the block rule for this name
    let block = match if parser.discarding_hidden_body() {
        get_block_rule_with_name(name)
    } else {
        get_block_rule_with_name_for_layout(name, parser.settings().layout)
    } {
        Some(block) => block,
        None => return Err(parser.make_err(ParseErrorKind::NoSuchBlock)),
    };
    if !block_rule_enabled(parser, block) {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    if block.name == "block-embedvideo" && spaced_name {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    if parser.settings().layout.legacy()
        && !parser.discarding_hidden_body()
        && block.name == "block-tab"
        && !parser.accepts_partial_here(crate::tree::AcceptsPartial::Tab)
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    if parser.settings().layout.legacy()
        && !parser.discarding_hidden_body()
        && !wikidot_block_has_physical_line_ownership(
            parser,
            parent_rule,
            block,
            opener_start,
        )
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    if parser.settings().layout.legacy()
        && !parser.discarding_hidden_body()
        && parent_rule.name() == "list"
        && matches!(block.name, "block-div" | "block-collapsible")
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    if parser.settings().layout.legacy()
        && !parser.discarding_hidden_body()
        && block.name == "block-collapsible"
        && (spaced_name
            || parent_rule.name() == "block-collapsible"
            || parser.in_wikidot_collapsible())
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }
    if parser.settings().layout.legacy()
        && !parser.discarding_hidden_body()
        && block.name == "block-user"
        && spaced_name
    {
        return Err(parser.make_err(ParseErrorKind::RuleFailed));
    }

    // Set block rule for better errors
    parser.set_block(block);

    // Check if this block allows star invocation (the '[[*' token)
    if !block.accepts_star && flag_star {
        return Err(parser.make_err(ParseErrorKind::BlockDisallowsStar));
    }

    // Check if this block allows score invocation ('_' after name)
    if !block.accepts_score && flag_score {
        return Err(parser.make_err(ParseErrorKind::BlockDisallowsScore));
    }

    parser.get_optional_space()?;

    // Run the parse function until the end.
    //
    // This is responsible for parsing any arguments,
    // and terminating the block (the ']]' token),
    // then processing the body (if any) and tail block.
    (block.parse_fn)(parser, name, flag_star, flag_score, in_head)
}

fn wikidot_block_has_physical_line_ownership(
    parser: &Parser<'_, '_>,
    parent_rule: Rule,
    block: &BlockRule,
    opener_start: usize,
) -> bool {
    if parser.settings().mode == WikitextMode::List
        || parser.in_native_blockquote_line()
        || parser.in_footnote()
        || parent_rule.name() == "block-footnote"
    {
        return true;
    }

    let needs_line_owner = matches!(
        block.name,
        "block-code" | "block-math" | "block-module" | "block-bibliography" | "block-toc"
    );
    if !needs_line_owner {
        return true;
    }

    let source = parser.full_text().inner();
    let line_start = source[..opener_start]
        .rfind('\n')
        .map_or(0, |index| index + 1);
    let prefix = &source[line_start..opener_start];

    if block.name == "block-module" {
        prefix.is_empty()
    } else {
        prefix
            .bytes()
            .all(|byte| matches!(byte, b' ' | b'\t' | b'\0'))
    }
}
