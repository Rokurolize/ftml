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
use crate::parsing::paragraph::ParagraphStack;
use crate::parsing::rule::impls::block::RULE_BLOCK;
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

// Wikidot closes header cells with `[[/cell]]`, while FTML has historically
// also accepted the symmetric `[[/hcell]]` spelling. Keep the registered
// opener rule restricted to `hcell` so `[[cell]]` still dispatches to the
// regular-cell parser, and use this body-only rule for the two valid closers.
const BLOCK_TABLE_CELL_HEADER_BODY: BlockRule = BlockRule {
    name: "block-table-cell-header-body",
    accepts_names: &["hcell", "cell"],
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
    has_arguments: bool,
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
    assert!(!flag_star, "{description} block doesn't allow star flag");
    assert!(!flag_score, "{description} block doesn't allow score flag");
    assert_block_name(block_rule, name);

    // Get attributes
    let (arguments, body_start) =
        parser.get_head_map_with_body_start_wikidot(block_rule, in_head)?;
    let has_arguments = arguments.has_source();
    let attributes = if parser.settings().layout.legacy() {
        arguments.to_attribute_map_without_bare(parser.settings())
    } else {
        arguments.to_attribute_map(parser.settings())
    };

    // Get body elements
    let body = parser.get_body_elements_with_context(block_rule, false, body_start)?;
    let (elements, errors, _) = body.into();

    // Return result
    let parsed = ParsedBlock {
        elements,
        attributes,
        errors,
        has_arguments,
    };
    Ok(parsed)
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

fn recover_legacy_attributed_table<'r, 't>(
    parser: &mut Parser<'r, 't>,
) -> Option<(Vec<TableRow<'t>>, Vec<ParseError>)>
where
    'r: 't,
{
    let mut scan = parser.clone();
    let mut rows = Vec::new();
    let mut errors = Vec::new();

    loop {
        if scan.current().token == Token::InputEnd {
            return None;
        }

        if !rows.is_empty() && scan.current().token == Token::LeftBlockEnd {
            let mut end = scan.clone();
            if end
                .get_end_block()
                .is_ok_and(|name| name.eq_ignore_ascii_case("table"))
            {
                parser.update(&end);
                return Some((rows, errors));
            }
        } else if scan.current().token == Token::LeftBlock {
            let mut head = scan.clone();
            let is_row = head
                .get_block_name(false)
                .is_ok_and(|(name, _)| name.eq_ignore_ascii_case("row"));

            if is_row {
                let mut candidate = scan.clone();
                if let Ok(success) = RULE_BLOCK.try_consume(&mut candidate)
                    && let Elements::Single(Element::Partial(PartialElement::TableRow(
                        row,
                    ))) = success.item
                {
                    rows.push(row);
                    errors.extend(success.errors);
                    scan.update(&candidate);
                    continue;
                }
            }
        }

        scan.step()
            .expect("tokenization always ends with input-end");
    }
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

    let mut rows = extract_table_rows(parser, parsed.elements)?;
    let mut errors = parsed.errors;
    if rows.is_empty()
        && parsed.has_arguments
        && parser.settings().layout.legacy()
        && let Some((recovered_rows, mut recovered_errors)) =
            recover_legacy_attributed_table(parser)
    {
        rows = recovered_rows;
        errors.append(&mut recovered_errors);
    }
    if rows.is_empty() && parser.settings().layout.legacy() {
        return Err(parser.make_err(ParseErrorKind::TableContainsNonRow));
    }
    let attributes = parsed.attributes;

    // Build and return table element
    let table_type = TableType::Advanced;
    let table = Table {
        rows,
        attributes,
        table_type,
    };
    let element = Element::Table(table);
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
    if cells.is_empty() && parser.settings().layout.legacy() {
        return Err(parser.make_err(ParseErrorKind::TableRowContainsNonCell));
    }
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
    let legacy = parser.settings().layout.legacy();
    let source_start = parser.current().span.start;

    // Get block contents.
    let parsed = parse_block(parser, name, flag_star, flag_score, in_head, block)?;
    let source_end = parser.current().span.start;
    let wrap_paragraph =
        legacy && parser.full_text().inner()[source_start..source_end].contains("\n\n");

    parse_cell(
        parsed.elements,
        parsed.attributes,
        parsed.errors,
        false,
        wrap_paragraph,
    )
}

fn parse_cell_header<'r, 't>(
    parser: &mut Parser<'r, 't>,
    name: &'t str,
    flag_star: bool,
    flag_score: bool,
    in_head: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    let parser = &mut ParserWrap::new(parser, AcceptsPartial::TableCell);
    let block = (&BLOCK_TABLE_CELL_HEADER_BODY, "table cell (header)");
    let legacy = parser.settings().layout.legacy();
    let source_start = parser.current().span.start;

    // Get block contents.
    let parsed = parse_block(parser, name, flag_star, flag_score, in_head, block)?;
    let source_end = parser.current().span.start;
    let source =
        parser.full_text().inner()[source_start..source_end].to_ascii_lowercase();
    let wrap_paragraph = legacy && source.contains("\n\n");
    let header =
        source.rfind("[[/hcell").unwrap_or(0) > source.rfind("[[/cell").unwrap_or(0);

    parse_cell(
        parsed.elements,
        parsed.attributes,
        parsed.errors,
        header,
        wrap_paragraph,
    )
}

fn parse_cell<'r, 't>(
    mut elements: Vec<Element<'t>>,
    mut attributes: AttributeMap<'t>,
    errors: Vec<ParseError>,
    header: bool,
    wrap_paragraph: bool,
) -> ParseResult<'r, 't, Elements<'t>> {
    // Remove leading and trailing whitespace
    strip_whitespace(&mut elements);
    if wrap_paragraph {
        wrap_cell_paragraph(&mut elements);
    }

    // Extract column-span if specified via attributes.
    // If not specified, then the default.
    let column_span = match attributes.remove("colspan") {
        Some(value) => match value.parse() {
            Ok(value) => value,
            Err(_) if value == "0" => NonZeroU32::new(1).unwrap(),
            Err(_) => {
                assert!(attributes.insert("colspan", value));
                NonZeroU32::new(1).unwrap()
            }
        },
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

fn wrap_cell_paragraph(elements: &mut Vec<Element<'_>>) {
    let table_index = elements
        .iter()
        .position(|element| matches!(element, Element::Table(_)))
        .unwrap_or(elements.len());

    let mut paragraph_elements: Vec<_> = elements.drain(..table_index).collect();
    strip_whitespace(&mut paragraph_elements);
    paragraph_elements.retain(|element| *element != Element::LineBreak);
    if paragraph_elements.is_empty() {
        return;
    }
    let mut paragraphs = ParagraphStack::new_wikidot();
    for element in paragraph_elements {
        let paragraph_safe = element.paragraph_safe();
        paragraphs.push_element(element, paragraph_safe);
    }
    elements.splice(0..0, paragraphs.into_elements());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::parsing::ParseError;
    use crate::render::{Render, html::HtmlRender};
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

    fn render(source: &str) -> String {
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(source);
        let (tree, _) = crate::parse(&tokenization, &page_info, &settings).into();
        HtmlRender.render(&tree, &page_info, &settings).body
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
    fn wikidot_quoted_simple_table_keeps_adjacent_rows_in_one_table() {
        // Live full-page oracle: sandbox-oracle-20260805-p5-r2,
        // frozen-captures.json (SHA-256
        // 3d175d8ea69434a214c252954be8fc65e80632973dba3668e528bd2801d2765f).
        let source = concat!(
            "> Quote lead\n",
            ">\n",
            "> ||~ H || V ||\n",
            "> || A || //B// ||\n",
            ">\n",
            "> trailing quote\n",
            "\n",
            "Intro [[footnote]]note[[/footnote]] tail.",
        );

        let html = render(source);

        assert_eq!(
            html.matches("<table class=\"wiki-content-table\">").count(),
            1,
            "{html}"
        );
        assert_eq!(html.matches("<tr>").count(), 2, "{html}");
    }

    #[test]
    fn advanced_header_cell_accepts_wikidot_cell_closer() {
        with_parse(
            "[[table]][[row]][[hcell]]Heading[[/cell]][[/row]][[/table]]",
            |tree, errors| {
                assert!(errors.is_empty(), "{errors:?}");
                let [Element::Table(table)] = tree.as_slice() else {
                    panic!("expected one advanced table, got {tree:?}");
                };
                let [row] = table.rows.as_slice() else {
                    panic!("expected one row, got {:?}", table.rows);
                };
                let [cell] = row.cells.as_slice() else {
                    panic!("expected one cell, got {:?}", row.cells);
                };

                assert!(!cell.header);
                assert_eq!(element_text(&cell.elements), "Heading");
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
    fn malformed_table_closes_reenter_wikidot_single_link_lexing() {
        assert_eq!(
            render("[[table]] [[cell]] Also no row [[/cell]] [[/table]]"),
            concat!(
                "<p>[[table]] [[cell]] Also no row ",
                "[<a href=\"/cell]]\">[[/table</a>]</p>",
            ),
        );
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
    fn attributed_empty_table_recovers_at_later_valid_table() {
        let source = concat!(
            "[[table class=\"outer\"]]\n",
            "[[/table]]\n\n",
            "[[table]]\n",
            "[[row bad=1]]\n",
            "[[/row]]\n",
            "[[/table]]\n\n",
            "[[table class=\"inner\"]]",
            "[[row]][[cell]]x[[/cell]][[/row]]",
            "[[/table]]",
        );

        assert_eq!(
            render(source),
            "<table class=\"outer\">\n<tr>\n<td>x</td>\n</tr>\n</table>",
        );
    }

    #[test]
    fn bare_attributed_empty_table_recovers_without_rendering_bare_attribute() {
        let source = concat!(
            "[[table bad=1]]\n",
            "[[/table]]\n\n",
            "[[table]][[row]][[cell]]x[[/cell]][[/row]][[/table]]",
        );

        assert_eq!(render(source), "<table>\n<tr>\n<td>x</td>\n</tr>\n</table>",);
    }

    #[test]
    fn attributed_empty_table_without_later_rows_remains_literal() {
        assert_eq!(
            render("[[table bad=1]]\n[[/table]]"),
            "<p>[[table bad=1]]<br>\n[[/table]]</p>",
        );
    }

    #[test]
    fn unadorned_empty_table_does_not_consume_later_valid_table() {
        let source = concat!(
            "[[table]]\n",
            "[[/table]]\n\n",
            "[[table]][[row]][[cell]]x[[/cell]][[/row]][[/table]]",
        );

        assert_eq!(
            render(source),
            concat!(
                "<p>[[table]]<br>\n[[/table]]</p>",
                "<table>\n<tr>\n<td>x</td>\n</tr>\n</table>",
            ),
        );
    }

    #[test]
    fn wikidot_nested_table_wraps_blank_line_text_in_paragraph() {
        let source = concat!(
            "[[table]][[row]][[cell]]\n\n",
            "1\n\n",
            "[[table]][[row]][[cell]]2[[/cell]][[/row]][[/table]]",
            "[[/cell]][[/row]][[/table]]",
        );

        assert_eq!(
            render(source),
            concat!(
                "<table>\n<tr>\n<td><p>1</p>",
                "<table>\n<tr>\n<td>2</td>\n</tr>\n</table>",
                "</td>\n</tr>\n</table>",
            ),
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
        let success = parse_cell(elements, attributes, Vec::new(), false, false).unwrap();
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
