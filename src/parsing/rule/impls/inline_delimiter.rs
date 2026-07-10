/*
 * parsing/rule/impls/inline_delimiter.rs
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

/// Consume an inline formatting opener that may not be followed by whitespace.
///
/// The formatting collectors already reject whitespace before a closing
/// delimiter. Rejecting padding on the opening side before collection also
/// prevents a malformed marker from searching later list items for a partner.
pub(super) fn assert_unpadded_open<'r, 't>(
    parser: &mut Parser<'r, 't>,
    token: Token,
) -> Result<(), ParseError> {
    assert_step(parser, token)?;
    if parser.current().token == Token::Whitespace {
        Err(parser.make_err(ParseErrorKind::RuleFailed))
    } else {
        Ok(())
    }
}
