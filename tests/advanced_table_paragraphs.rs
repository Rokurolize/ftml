use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::{Render, html::HtmlRender};
use ftml::settings::{WikitextMode, WikitextSettings};
use ftml::tree::{ContainerType, Element};
use std::borrow::Cow;

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("advanced-table-paragraphs"),
        category: None,
        site: Cow::Borrowed("compatibility"),
        title: Cow::Borrowed("Advanced table paragraphs"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![],
        language: Cow::Borrowed("en"),
    }
}

fn render_with_layout(source: &str, layout: Layout) -> String {
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, layout);
    let mut source = source.to_owned();
    ftml::preprocess_for_layout(&mut source, layout);
    let tokenization = ftml::tokenize(&source);
    let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    assert!(errors.is_empty(), "{source:?}: {errors:#?}");
    HtmlRender.render(&tree, &page_info, &settings).body
}

fn render(source: &str) -> String {
    render_with_layout(source, Layout::Wikidot)
}

fn table(cell: &str, body: &str) -> String {
    format!("[[table]]\n[[row]]\n[[{cell}]]\n{body}\n[[/{cell}]]\n[[/row]]\n[[/table]]")
}

fn inner_table(label: &str) -> String {
    table("cell", label)
}

fn assert_block_between_paragraphs(block: &str, marker: &str) {
    for cell in ["cell", "hcell"] {
        let html = render(&table(cell, &format!("A\n\n{block}\n\nB")));
        let before = html.find("<p>A</p>").expect("leading paragraph");
        let block = html[before..].find(marker).expect("block sibling") + before;
        let after = html[block..].find("<p>B</p>").expect("trailing paragraph") + block;
        assert!(before < block && block < after, "{cell}: {html}");
        assert!(!html.contains("<br>\nB"), "{cell}: {html}");
    }
}

#[test]
fn blank_lines_create_independent_cell_and_hcell_paragraphs() {
    for (cell, tag) in [("cell", "td"), ("hcell", "th")] {
        assert_eq!(
            render(&table(cell, "A\n\nB")),
            format!("<table>\n<tr>\n<{tag}><p>A</p><p>B</p></{tag}>\n</tr>\n</table>"),
            "{cell}",
        );
    }
}

#[test]
fn single_lines_and_ordinary_line_breaks_remain_unwrapped() {
    for (cell, tag) in [("cell", "td"), ("hcell", "th")] {
        assert_eq!(
            render(&table(cell, "A")),
            format!("<table>\n<tr>\n<{tag}>A</{tag}>\n</tr>\n</table>"),
            "{cell} single line",
        );
        assert_eq!(
            render(&table(cell, "A\nB")),
            format!("<table>\n<tr>\n<{tag}>A<br>\nB</{tag}>\n</tr>\n</table>"),
            "{cell} ordinary line break",
        );
    }
}

#[test]
fn text_around_nested_tables_gets_independent_sibling_paragraphs() {
    let nested = inner_table("I");
    let nested_html = "<table>\n<tr>\n<td>I</td>\n</tr>\n</table>";

    for (cell, tag) in [("cell", "td"), ("hcell", "th")] {
        assert_eq!(
            render(&table(cell, &format!("A\n{nested}\nB"))),
            format!(
                "<table>\n<tr>\n<{tag}><p>A</p>{nested_html}<p>B</p></{tag}>\n</tr>\n</table>"
            ),
            "{cell}",
        );
    }
}

#[test]
fn nested_only_and_matching_multi_table_controls_stay_unwrapped() {
    let first = inner_table("I");
    let second = inner_table("J");
    let first_html = "<table>\n<tr>\n<td>I</td>\n</tr>\n</table>";
    let second_html = "<table>\n<tr>\n<td>J</td>\n</tr>\n</table>";

    for (cell, tag) in [("cell", "td"), ("hcell", "th")] {
        assert_eq!(
            render(&table(cell, &first)),
            format!("<table>\n<tr>\n<{tag}>{first_html}</{tag}>\n</tr>\n</table>"),
            "{cell} nested only",
        );
        assert_eq!(
            render(&table(cell, &format!("{first}\n{second}"))),
            format!(
                "<table>\n<tr>\n<{tag}>{first_html}{second_html}</{tag}>\n</tr>\n</table>"
            ),
            "{cell} two nested tables",
        );
    }
}

#[test]
fn block_siblings_preserve_paragraphs_on_both_sides() {
    for (block, marker) in [
        ("* X", "<ul>"),
        ("> X", "<blockquote>"),
        ("[[code]]\nX\n[[/code]]", "<div class=\"code\">"),
        ("[[div]]\nX\n[[/div]]", "<div>"),
        ("|| X ||", "<table class=\"wiki-content-table\">"),
    ] {
        assert_block_between_paragraphs(block, marker);
    }
}

#[test]
fn inline_owners_survive_blank_line_segmentation() {
    for (body, expected) in [
        ("A[!--hidden--]\n\nB", "<p>A</p><p>B</p>"),
        ("[[#if 1 | A | X ]]\n\nB", "<p>A</p><p>B</p>"),
        (
            "@@A@@\n\nB",
            concat!(
                "<p><span style=\"white-space: pre-wrap;\">A</span></p>",
                "<p>B</p>",
            ),
        ),
    ] {
        for (cell, tag) in [("cell", "td"), ("hcell", "th")] {
            let html = render(&table(cell, body));
            assert!(
                html.contains(&format!("<{tag}>{expected}</{tag}>")),
                "{cell}: {body:?}: {html}",
            );
        }
    }

    let html = render(&table("cell", "A[[footnote]]N[[/footnote]]\n\nB"));
    assert!(
        html.contains("<td><p>A<sup class=\"footnoteref\">"),
        "{html}"
    );
    assert!(html.contains("</sup></p><p>B</p></td>"), "{html}");
    assert_eq!(html.matches("class=\"footnote-footer\"").count(), 1);
}

#[test]
fn captured_cell_body_controls_keep_their_existing_wrapper_shape() {
    for (body, expected) in [
        ("**A**", "<td><strong>A</strong></td>"),
        (
            "[https://example.com A]",
            "<td><a href=\"https://example.com\">A</a></td>",
        ),
        ("A[!--hidden--]B", "<td>AB</td>"),
        ("[[#if 1 | A | X ]]", "<td>A</td>"),
        (
            "@@A@@",
            "<td><span style=\"white-space: pre-wrap;\">A</span></td>",
        ),
        ("* A\n* B", "<td><ul>\n<li>A</li>\n<li>B</li>\n</ul></td>"),
        ("> A", "<td><blockquote><p>A</p></blockquote></td>"),
        (
            "[[code]]\nA\n[[/code]]",
            "<td><div class=\"code\"><pre><code>A</code></pre></div></td>",
        ),
        ("[[div]]\nA\n[[/div]]", "<td><div><p>A</p></div></td>"),
        (
            "|| A || B ||",
            concat!(
                "<td><table class=\"wiki-content-table\">\n<tr>\n",
                "<td>A</td>\n<td>B</td>\n</tr>\n</table></td>",
            ),
        ),
    ] {
        let html = render(&table("cell", body));
        assert!(html.contains(expected), "{body:?}: {html}");
    }

    assert_eq!(
        render(&table("cell", "[[module CSS]]\nx{}\n[[/module]]")),
        "<table>\n<tr>\n<td></td>\n</tr>\n</table>",
    );
}

#[test]
fn cell_paragraph_parsing_commits_metadata_once() {
    let mut source = table(
        "cell",
        "A\n\n[[code]]\nX\n[[/code]]\n\nB[[footnote]]N[[/footnote]]",
    );
    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    ftml::preprocess_for_layout(&mut source, settings.layout);
    let tokenization = ftml::tokenize(&source);
    let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();

    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(tree.code_blocks.len(), 1, "{tree:#?}");
    assert_eq!(tree.footnotes.len(), 1, "{tree:#?}");
    let [Element::Table(table)] = tree.elements.as_slice() else {
        panic!("expected one table: {tree:#?}");
    };
    let elements = &table.rows[0].cells[0].elements;
    assert!(
        matches!(
            elements.as_slice(),
            [
                Element::Container(first),
                Element::Code(_),
                Element::Container(last),
            ] if first.ctype() == ContainerType::Paragraph
                && last.ctype() == ContainerType::Paragraph
        ),
        "{elements:#?}",
    );
}

#[test]
fn crlf_and_unicode_paragraph_boundaries_keep_their_content() {
    let source = table("cell", "日本語 α\n\nβ 終端");
    let expected =
        "<table>\n<tr>\n<td><p>日本語 α</p><p>β 終端</p></td>\n</tr>\n</table>";
    assert_eq!(render(&source), expected);
    assert_eq!(render(&source.replace('\n', "\r\n")), expected);
}

#[test]
fn outer_whitespace_and_inner_runs_follow_wikidot_normalization() {
    let source = table("cell", " \t A  B \t\n\n \tC D \t");
    assert_eq!(
        render(&source),
        "<table>\n<tr>\n<td><p>A B</p><p>C D</p></td>\n</tr>\n</table>",
    );
}

#[test]
fn wikijump_layout_keeps_its_existing_cell_body_shape() {
    let source = table("cell", "A\n\nB");
    assert_eq!(
        render_with_layout(&source, Layout::Wikijump),
        concat!(
            "<table class=\"wj-table wj-table-advanced\"><tbody>",
            "<tr><td>A<br>B</td></tr></tbody></table>",
        ),
    );
}
