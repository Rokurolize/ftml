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
            Some(block_rule) => Ok(block_rule.accepts_newlines),
            None => Ok(false),
        }
    });

    if result {
        parse_block(parser, flag_star)
    } else {
        Err(parser.make_err(ParseErrorKind::RuleFailed))
    }
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
