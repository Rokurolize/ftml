/*
 * parsing/rule/impls/table.rs
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
use crate::tree::{Alignment, Table, TableCell, TableRow, TableType};
use std::mem;
use std::num::NonZeroU32;

#[derive(Debug, Clone, Copy)]
struct TableCellStart {
    align: Option<Alignment>,
    header: bool,
    column_span: NonZeroU32,
}

pub const RULE_TABLE: Rule = Rule {
    name: "table",
    position: LineRequirement::StartOfLine,
    try_consume_fn,
};

fn take_row<'t>(cells: &mut Vec<TableCell<'t>>) -> TableRow<'t> {
    TableRow {
        cells: mem::take(cells),
        attributes: AttributeMap::new(),
    }
}

fn push_row<'t>(rows: &mut Vec<TableRow<'t>>, cells: &mut Vec<TableCell<'t>>) {
    let row = take_row(cells);
    rows.push(row);
}

fn take_cell<'t>(
    elements: &mut Vec<Element<'t>>,
    cell_start: TableCellStart,
) -> TableCell<'t> {
    let elements = mem::take(elements);
    let attributes = AttributeMap::new();
    TableCell {
        elements,
        header: cell_start.header,
        column_span: cell_start.column_span,
        align: cell_start.align,
        attributes,
    }
}

fn push_cell<'t>(
    cells: &mut Vec<TableCell<'t>>,
    elements: &mut Vec<Element<'t>>,
    cell_start: TableCellStart,
) {
    let cell = take_cell(elements, cell_start);
    cells.push(cell);
}

fn simple_table<'t>(rows: Vec<TableRow<'t>>) -> Element<'t> {
    let attributes = AttributeMap::new();
    let table_type = TableType::Simple;
    let table = Table {
        rows,
        attributes,
        table_type,
    };
    Element::Table(table)
}

fn is_table_column_token(token: Token) -> bool {
    matches!(
        token,
        Token::TableColumn
            | Token::TableColumnTitle
            | Token::TableColumnCenter
            | Token::TableColumnRight
    )
}

enum CellBoundary {
    FinishTable,
    FinishRow,
    ContinueCell,
}

struct CellState<'a, 't> {
    rows: &'a mut Vec<TableRow<'t>>,
    cells: &'a mut Vec<TableCell<'t>>,
    elements: &'a mut Vec<Element<'t>>,
}

impl<'a, 't> CellState<'a, 't> {
    fn new(
        rows: &'a mut Vec<TableRow<'t>>,
        cells: &'a mut Vec<TableCell<'t>>,
        elements: &'a mut Vec<Element<'t>>,
    ) -> Self {
        Self {
            rows,
            cells,
            elements,
        }
    }
}

fn finish_simple_table<'r, 't>(
    rows: Vec<TableRow<'t>>,
    errors: Vec<ParseError>,
) -> ParseResult<'r, 't, Elements<'t>> {
    let table = simple_table(rows);
    ok!(false; table, errors)
}

fn finish_table_or_fail<'r, 't>(
    parser: &Parser<'r, 't>,
    rows: Vec<TableRow<'t>>,
    errors: Vec<ParseError>,
) -> ParseResult<'r, 't, Elements<'t>> {
    let has_rows = !rows.is_empty();
    if has_rows {
        finish_simple_table(rows, errors)
    } else {
        Err(parser.make_err(ParseErrorKind::RuleFailed))
    }
}

fn finish_cell_and_table<'r, 't>(
    parser: &mut Parser<'r, 't>,
    state: &mut CellState<'_, 't>,
    cell_start: TableCellStart,
    steps: usize,
) -> Result<CellBoundary, ParseError> {
    push_cell(state.cells, state.elements, cell_start);
    push_row(state.rows, state.cells);
    parser.step_n(steps)?;
    let boundary = CellBoundary::FinishTable;
    Ok(boundary)
}

fn finish_cell_and_row<'r, 't>(
    parser: &mut Parser<'r, 't>,
    state: &mut CellState<'_, 't>,
    cell_start: TableCellStart,
    steps: usize,
) -> Result<CellBoundary, ParseError> {
    push_cell(state.cells, state.elements, cell_start);
    parser.step_n(steps)?;
    let boundary = CellBoundary::FinishRow;
    Ok(boundary)
}

fn cell_boundary<'r, 't>(
    parser: &mut Parser<'r, 't>,
    state: &mut CellState<'_, 't>,
    cell_start: TableCellStart,
    next: Token,
) -> Result<CellBoundary, ParseError> {
    match next {
        Token::ParagraphBreak | Token::InputEnd => {
            finish_cell_and_table(parser, state, cell_start, 1)
        }
        Token::LineBreak => finish_cell_and_row(parser, state, cell_start, 2),
        Token::Whitespace => match parser.look_ahead(1).map(|t| t.token) {
            Some(Token::ParagraphBreak) | Some(Token::InputEnd) | None => {
                finish_cell_and_table(parser, state, cell_start, 2)
            }
            Some(Token::LineBreak) => finish_cell_and_row(parser, state, cell_start, 3),
            _ => Ok(CellBoundary::ContinueCell),
        },
        _ => Ok(CellBoundary::ContinueCell),
    }
}

fn try_consume_fn<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> ParseResult<'r, 't, Elements<'t>> {
    debug!("Trying to parse simple table");
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    let mut paragraph_break = false;

    loop {
        debug!("Parsing next table row");

        let mut cells = Vec::new();

        // Loop for each cell in the row
        'row: loop {
            debug!("Parsing next table cell");
            let mut elements = Vec::new();
            let cell_start = match parse_cell_start(parser)? {
                Some(cell_start) => cell_start,
                None => return finish_table_or_fail(parser, rows, errors),
            };

            // Loop for each element in the cell
            'cell: loop {
                trace!("Parsing next element (length {})", elements.len());
                match parser.next_two_tokens() {
                    // End the cell or row
                    (current, Some(next)) if is_table_column_token(current) => {
                        trace!("Ending cell, row, or table");
                        let (r, c, e) = (&mut rows, &mut cells, &mut elements);
                        let p = std::convert::identity(&mut *parser);
                        let mut state = CellState::new(r, c, e);
                        let boundary = cell_boundary(p, &mut state, cell_start, next)?;
                        let (finish_row, finish_cell) = match boundary {
                            CellBoundary::FinishTable => {
                                return finish_simple_table(rows, errors);
                            }
                            CellBoundary::FinishRow => (true, false),
                            CellBoundary::ContinueCell => (false, true),
                        };
                        match (finish_row, finish_cell) {
                            (true, _) => break 'row std::convert::identity(()),
                            (_, true) => break 'cell std::convert::identity(()),
                            _ => unreachable!("cell boundary must finish row or cell"),
                        }
                    }

                    // Ignore leading whitespace
                    (Token::Whitespace, _) if elements.is_empty() => {
                        trace!("Ignoring leading whitespace");
                        parser.step()?;
                    }

                    // Ignore trailing whitespace
                    (Token::Whitespace, Some(next)) if is_table_column_token(next) => {
                        trace!("Ignoring trailing whitespace");
                        parser.step()?;
                    }

                    // Invalid tokens
                    (Token::LineBreak | Token::ParagraphBreak | Token::InputEnd, _) => {
                        trace!("Invalid termination tokens in table, ending");
                        return finish_table_or_fail(parser, rows, errors);
                    }

                    // Consume tokens like normal
                    _ => {
                        trace!("Consuming cell contents as elements");

                        let consumed = consume(parser)?;
                        let new_items = consumed.chain(&mut errors, &mut paragraph_break);

                        elements.extend(new_items);
                    }
                }
            }

            push_cell(&mut cells, &mut elements, cell_start);
        }

        push_row(&mut rows, &mut cells);
    }
}

/// Parse out the cell settings from the start.
///
/// Cells have a few settings, such as alignment, and most importantly
/// here, their span, which is specified by having multiple
/// `Token::TableColumn` (`||`) adjacent together.
///
/// If `Ok(None)` is returned, then the end of the input wasn't reached,
/// but this is not a valid cell start.
///
/// This is not an `Err(_)` case, because this may simply signal the end
/// of the table if it already has rows.
fn parse_cell_start(parser: &mut Parser) -> Result<Option<TableCellStart>, ParseError> {
    let mut span = 0;

    macro_rules! increase_span {
        () => {{
            span += 1;
            parser.step()?;
        }};
    }

    let (align, header) = loop {
        match parser.current().token {
            // Style cases, terminal
            // NOTE: There is no TableColumnLeft
            Token::TableColumnTitle => {
                increase_span!();
                break (None, true);
            }
            Token::TableColumnCenter => {
                increase_span!();
                break (Some(Alignment::Center), false);
            }
            Token::TableColumnRight => {
                increase_span!();
                break (Some(Alignment::Right), false);
            }

            // Regular column, iterate to see if it has a span
            Token::TableColumn => increase_span!(),

            // Regular column, terminal
            _ if span > 0 => break (None, false),

            // No span depth, just an invalid token
            _ => return Ok(None),
        }
    };

    let column_span =
        NonZeroU32::new(span).expect("Cell start exited without column span");

    Ok(Some(TableCellStart {
        align,
        header,
        column_span,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::settings::{WikitextMode, WikitextSettings};

    fn with_parser<R>(
        input: &str,
        page_info: &PageInfo<'_>,
        settings: &WikitextSettings,
        run: impl FnOnce(&mut Parser<'_, '_>) -> R,
    ) -> R {
        let tokenization = crate::tokenize(input);
        let mut parser = Parser::new(&tokenization, page_info, settings);
        parser
            .step()
            .expect("first token should follow input start");
        parser.set_rule(RULE_TABLE);
        run(&mut parser)
    }

    #[test]
    fn table_cell_start_parses_alignment_headers_and_colspan() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        let start = with_parser("|||| body", &page_info, &settings, |parser| {
            parse_cell_start(parser)
                .expect("cell start should parse")
                .expect("colspan should produce a cell start")
        });
        assert_eq!(start.align, None);
        assert!(!start.header);
        assert_eq!(start.column_span.get(), 2);

        let start = with_parser("||~ body", &page_info, &settings, |parser| {
            parse_cell_start(parser)
                .expect("header cell start should parse")
                .expect("header should produce a cell start")
        });
        assert!(start.header);
        assert_eq!(start.align, None);
        assert_eq!(start.column_span.get(), 1);

        let start = with_parser("||= body", &page_info, &settings, |parser| {
            parse_cell_start(parser)
                .expect("center cell start should parse")
                .expect("center marker should produce a cell start")
        });
        assert_eq!(start.align, Some(Alignment::Center));
        assert!(!start.header);

        let start = with_parser("||> body", &page_info, &settings, |parser| {
            parse_cell_start(parser)
                .expect("right cell start should parse")
                .expect("right marker should produce a cell start")
        });
        assert_eq!(start.align, Some(Alignment::Right));
        assert!(!start.header);

        with_parser("plain", &page_info, &settings, |parser| {
            assert!(
                parse_cell_start(parser)
                    .expect("invalid start is not a parser error")
                    .is_none()
            );
        });
    }

    #[test]
    fn table_cell_boundary_updates_rows_and_cells() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let cell_start = TableCellStart {
            align: None,
            header: false,
            column_span: NonZeroU32::new(1).unwrap(),
        };

        with_parser("||", &page_info, &settings, |parser| {
            let mut rows = Vec::new();
            let mut cells = Vec::new();
            let mut elements = Vec::new();
            let mut state = CellState::new(&mut rows, &mut cells, &mut elements);
            let boundary = cell_boundary(parser, &mut state, cell_start, Token::InputEnd)
                .expect("input end should finish table");
            assert!(matches!(boundary, CellBoundary::FinishTable));
            assert_eq!(state.rows.len(), 1);
            assert!(state.cells.is_empty());
        });

        with_parser("||\n||", &page_info, &settings, |parser| {
            let mut rows = Vec::new();
            let mut cells = Vec::new();
            let mut elements = Vec::new();
            let mut state = CellState::new(&mut rows, &mut cells, &mut elements);
            let boundary =
                cell_boundary(parser, &mut state, cell_start, Token::LineBreak)
                    .expect("line break should finish row");
            assert!(matches!(boundary, CellBoundary::FinishRow));
            assert!(state.rows.is_empty());
            assert_eq!(state.cells.len(), 1);
        });

        with_parser("|| next", &page_info, &settings, |parser| {
            let mut rows = Vec::new();
            let mut cells = Vec::new();
            let mut elements = Vec::new();
            let mut state = CellState::new(&mut rows, &mut cells, &mut elements);
            let boundary =
                cell_boundary(parser, &mut state, cell_start, Token::Whitespace)
                    .expect("ordinary whitespace should continue cell");
            assert!(matches!(boundary, CellBoundary::ContinueCell));
            assert!(state.rows.is_empty());
            assert!(state.cells.is_empty());
        });
    }

    #[test]
    fn simple_table_consume_loop_finishes_rows_and_cells() {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);

        let tokenization = crate::tokenize("|| A || B ||\n|| C || D ||");
        let mut parser = Parser::new(&tokenization, &page_info, &settings);
        parser
            .step()
            .expect("first token should follow input start");
        parser.set_rule(RULE_TABLE);

        let parsed = try_consume_fn(&mut parser).expect("simple table should parse");
        let (elements, errors, paragraph_safe) = parsed.into();

        assert!(errors.is_empty(), "{errors:?}");
        assert!(!paragraph_safe);
        let Elements::Single(Element::Table(table)) = elements else {
            panic!("expected one simple table, got {elements:?}");
        };
        assert_eq!(table.table_type, TableType::Simple);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells.len(), 2);
        assert_eq!(table.rows[1].cells.len(), 2);
    }
}
