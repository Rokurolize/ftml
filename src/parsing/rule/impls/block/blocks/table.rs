/*
 * parsing/rule/impls/block/blocks/table.rs
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
use crate::parsing::{ParserWrap, strip_whitespace};
use crate::tree::{
    AcceptsPartial, AttributeMap, PartialElement, Table, TableCell, TableRow, TableType,
};
use std::num::NonZeroU32;

pub const BLOCK_TABLE: BlockRule = BlockRule {
    name: "block-table",
    accepts_names: &["table"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: true,
    parse_fn: parse_table,
};

pub const BLOCK_TABLE_ROW: BlockRule = BlockRule {
    name: "block-table-row",
    accepts_names: &["row"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: true,
    parse_fn: parse_row,
};

pub const BLOCK_TABLE_CELL_REGULAR: BlockRule = BlockRule {
    name: "block-table-cell-regular",
    accepts_names: &["cell"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: true,
    parse_fn: parse_cell_regular,
};

pub const BLOCK_TABLE_CELL_HEADER: BlockRule = BlockRule {
    name: "block-table-cell-header",
    accepts_names: &["hcell"],
    accepts_star: false,
    accepts_score: false,
    accepts_newlines: true,
    parse_fn: parse_cell_header,
};

// Helper functions and macros

#[derive(Debug)]
struct ParsedBlock<'t> {
    elements: Vec<Element<'t>>,
    attributes: AttributeMap<'t>,
    errors: Vec<ParseError>,
}

fn parse_block<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
    (block_rule, description): (&BlockRule, &str),
) -> Result<ParsedBlock<'t>, ParseError>
where
    'r: 't,
    ParsedBlock<'t>: 't,
{
    debug!("Parsing {description} block (name '{name}', in-head {in_head})");
    assert!(
        !flag_star,
        "Block for {description} doesn't allow star flag"
    );
    assert!(
        !flag_score,
        "Block for {description} doesn't allow score flag"
    );
    assert_block_name(block_rule, name);

    // Get attributes
    let arguments = parser.get_head_map(block_rule, in_head)?;
    let attributes = arguments.to_attribute_map(parser.settings());

    // Get body elements
    let body = parser.get_body_elements(block_rule, false)?;
    let (elements, errors, _) = body.into();

    // Return result
    Ok(ParsedBlock {
        elements,
        attributes,
        errors,
    })
}

fn extract_table_rows<'r, 't>(
    parser: &Parser<'r, 't>,
    elements: Vec<Element<'t>>,
) -> Result<Vec<TableRow<'t>>, ParseError> {
    let mut rows = Vec::new();

    for element in elements {
        match element {
            // Append the next table row.
            Element::Partial(PartialElement::TableRow(row)) => {
                rows.push(row);
            }

            // Ignore internal whitespace.
            element if element.is_whitespace() => {}

            // Return an error for anything else.
            _ => return Err(parser.make_err(ParseErrorKind::TableContainsNonRow)),
        }
    }

    Ok(rows)
}

fn extract_table_cells<'r, 't>(
    parser: &Parser<'r, 't>,
    elements: Vec<Element<'t>>,
) -> Result<Vec<TableCell<'t>>, ParseError> {
    let mut cells = Vec::new();

    for element in elements {
        match element {
            // Append the next table cell.
            Element::Partial(PartialElement::TableCell(cell)) => {
                cells.push(cell);
            }

            // Ignore internal whitespace.
            element if element.is_whitespace() => {}

            // Return an error for anything else.
            _ => return Err(parser.make_err(ParseErrorKind::TableRowContainsNonCell)),
        }
    }

    Ok(cells)
}

// Table block

fn parse_table<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    let parser = &mut ParserWrap::new(parser, AcceptsPartial::TableRow);
    let block = (&BLOCK_TABLE, "table block");

    // Get block contents.
    let parsed = parse_block(parser, name, flag_star, flag_score, in_head, block)?;

    let rows = extract_table_rows(parser, parsed.elements)?;
    let attributes = parsed.attributes;
    let errors = parsed.errors;

    // Build and return table element
    let element = Element::Table(Table {
        rows,
        attributes,
        table_type: TableType::Advanced,
    });
    ok!(false; element, errors)
}

// Table row

fn parse_row<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    let parser = &mut ParserWrap::new(parser, AcceptsPartial::TableCell);
    let block = (&BLOCK_TABLE_ROW, "table row");

    // Get block contents.
    let parsed = parse_block(parser, name, flag_star, flag_score, in_head, block)?;

    let cells = extract_table_cells(parser, parsed.elements)?;
    let attributes = parsed.attributes;
    let errors = parsed.errors;

    // Build and return table row
    let row = TableRow { cells, attributes };
    let element = Element::Partial(PartialElement::TableRow(row));

    ok!(false; element, errors)
}

// Table cell

fn parse_cell_regular<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    let block = (&BLOCK_TABLE_CELL_REGULAR, "table cell (regular)");

    // Get block contents.
    let parsed = parse_block(parser, name, flag_star, flag_score, in_head, block)?;

    parse_cell(parsed.elements, parsed.attributes, parsed.errors, false)
}

fn parse_cell_header<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    let parser = &mut ParserWrap::new(parser, AcceptsPartial::TableCell);
    let block = (&BLOCK_TABLE_CELL_HEADER, "table cell (header)");

    // Get block contents.
    let parsed = parse_block(parser, name, flag_star, flag_score, in_head, block)?;

    parse_cell(parsed.elements, parsed.attributes, parsed.errors, true)
}

fn parse_cell<'r, 't>(
    mut elements: Vec<Element<'t>>,
    mut attributes: AttributeMap<'t>,
    errors: Vec<ParseError>,
    header: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    // Remove leading and trailing whitespace
    strip_whitespace(&mut elements);

    // Extract column-span if specified via attributes.
    // If not specified, then the default.
    let column_span = match attributes.remove("colspan") {
        Some(value) => value.parse().unwrap_or(NonZeroU32::new(1).unwrap()),
        None => NonZeroU32::new(1).unwrap(),
    };

    let cell = TableCell {
        header,
        column_span,
        align: None,
        elements,
        attributes,
    };
    let element = Element::Partial(PartialElement::TableCell(cell));

    ok!(false; element, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::parsing::ParseError;
    use crate::settings::{WikitextMode, WikitextSettings};
    use std::panic::catch_unwind;

    fn with_parse<R>(
        source: &str,
        check: impl for<'t> FnOnce(Vec<Element<'t>>, Vec<ParseError>) -> R,
    ) -> R {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();

        check(tree.elements, errors)
    }

    fn element_text(elements: &[Element]) -> String {
        elements
            .iter()
            .filter_map(|element| match element {
                Element::Text(text) => Some(text.as_ref()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn table_parse_block_rejects_disallowed_flags() {
        let parse_with_flags = |flag_star, flag_score| {
            let page_info = PageInfo::dummy();
            let settings =
                WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
            let tokenization = crate::tokenize("[[table]]\n[[/table]]");
            let mut parser = Parser::new(&tokenization, &page_info, &settings);
            let _ = parse_block(
                &mut parser,
                "table",
                flag_star,
                flag_score,
                false,
                (&BLOCK_TABLE, "table block"),
            );
        };

        let star = catch_unwind(|| parse_with_flags(true, false));
        assert!(star.is_err());

        let score = catch_unwind(|| parse_with_flags(false, true));
        assert!(score.is_err());
    }

    #[test]
    fn advanced_table_preserves_rows_cells_headers_and_colspan() {
        with_parse(
            r#"[[table class="grid"]]
[[row class="top"]]
[[hcell colspan="2" class="heading"]]
 Heading 
[[/hcell]]
[[/row]]
[[row]]
[[cell class="body"]]Content[[/cell]]
[[/row]]
[[/table]]"#,
            |tree, errors| {
                assert!(errors.is_empty(), "{errors:?}");
                let [Element::Table(table)] = tree.as_slice() else {
                    panic!("expected one advanced table, got {tree:?}");
                };

                assert_eq!(table.table_type, TableType::Advanced);
                assert_eq!(
                    table
                        .attributes
                        .get()
                        .get("class")
                        .map(|value| value.as_ref()),
                    Some("grid")
                );
                assert_eq!(table.rows.len(), 2);

                let header_row = &table.rows[0];
                assert_eq!(
                    header_row
                        .attributes
                        .get()
                        .get("class")
                        .map(|value| value.as_ref()),
                    Some("top")
                );
                let [header_cell] = header_row.cells.as_slice() else {
                    panic!("expected one header cell, got {:?}", header_row.cells);
                };
                assert!(header_cell.header);
                assert_eq!(header_cell.column_span.get(), 2);
                assert_eq!(
                    header_cell
                        .attributes
                        .get()
                        .get("class")
                        .map(|value| value.as_ref()),
                    Some("heading")
                );
                assert_eq!(element_text(&header_cell.elements), "Heading");

                let body_row = &table.rows[1];
                let [body_cell] = body_row.cells.as_slice() else {
                    panic!("expected one body cell, got {:?}", body_row.cells);
                };
                assert!(!body_cell.header);
                assert_eq!(body_cell.column_span.get(), 1);
                assert_eq!(
                    body_cell
                        .attributes
                        .get()
                        .get("class")
                        .map(|value| value.as_ref()),
                    Some("body")
                );
                assert_eq!(element_text(&body_cell.elements), "Content");
            },
        );
    }

    #[test]
    fn advanced_table_rejects_non_row_body() {
        with_parse("[[table]]plain text[[/table]]", |_tree, errors| {
            assert!(
                errors
                    .iter()
                    .any(|error| error.kind() == ParseErrorKind::TableContainsNonRow)
            );
        });
    }

    #[test]
    fn advanced_table_row_rejects_non_cell_body() {
        with_parse(
            "[[table]][[row]]plain text[[/row]][[/table]]",
            |_tree, errors| {
                assert!(
                    errors
                        .iter()
                        .any(|error| error.kind()
                            == ParseErrorKind::TableRowContainsNonCell)
                );
            },
        );
    }

    #[test]
    fn parse_cell_strips_whitespace_and_defaults_colspan() {
        let mut attributes = AttributeMap::new();
        assert!(attributes.insert("class", cow!("plain")));
        let elements = vec![
            Element::Text(cow!(" ")),
            Element::Text(cow!("Cell")),
            Element::Text(cow!(" ")),
        ];
        let success = parse_cell(elements, attributes, Vec::new(), false).unwrap();
        let Elements::Single(Element::Partial(PartialElement::TableCell(cell))) =
            success.item
        else {
            panic!("expected one table cell, got {:?}", success.item);
        };

        assert!(!cell.header);
        assert_eq!(cell.column_span.get(), 1);
        assert_eq!(
            cell.attributes
                .get()
                .get("class")
                .map(|value| value.as_ref()),
            Some("plain")
        );
        assert_eq!(element_text(&cell.elements), "Cell");
    }
}
