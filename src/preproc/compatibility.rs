/*
 * preproc/compatibility.rs
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

//! Canonicalization of malformed-but-real Wikidot syntax shapes.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Marker {
    CenterOpen,
    CenterClose,
    CollapsibleOpen,
    CollapsibleClose,
}

fn split_line(line: &str) -> (&str, &str) {
    line.strip_suffix('\n')
        .map_or((line, ""), |body| (body, "\n"))
}

fn marker_line(line: &str) -> Option<(&str, Marker)> {
    let (body, _) = split_line(line);
    let marker_start = body.find("[[")?;
    let prefix = &body[..marker_start];
    if !prefix.chars().all(|ch| matches!(ch, '>' | ' ' | '\t')) {
        return None;
    }

    let marker = &body[marker_start..];
    let kind = match marker {
        "[[=]]" => Marker::CenterOpen,
        "[[/=]]" => Marker::CenterClose,
        "[[/collapsible]]" => Marker::CollapsibleClose,
        marker if marker.starts_with("[[collapsible ") && marker.ends_with("]]") => {
            Marker::CollapsibleOpen
        }
        _ => return None,
    };

    Some((prefix, kind))
}

/// Move a prematurely crossed center closer behind its collapsible closer.
///
/// Wikidot treats the corpus-backed shape
/// `[[=]][[collapsible]][[/=]]...[[/collapsible]]` as a centered collapsible,
/// effectively canonicalizing the close order. FTML's tree parser needs that
/// nesting made explicit before tokenization.
pub fn substitute(text: &mut String) {
    let mut lines = text
        .split_inclusive('\n')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let literal_lines = literal_line_mask(&lines);
    canonicalize_unquoted_collapsible_closers(&mut lines, &literal_lines);
    canonicalize_unmatched_quoted_tab_closers(&mut lines, &literal_lines);
    canonicalize_crossed_center_collapsible_closers(&mut lines, &literal_lines);
    canonicalize_crossed_bold_size_closers(&mut lines, &literal_lines);
    remove_tight_quote_lines(&mut lines, &literal_lines);

    *text = lines.concat();
}

/// Remove lines whose first native quote marker is not followed by horizontal space.
///
/// Wikidot consumes these lines rather than treating their remainder as quoted
/// content or literal text. Root-level literal regions remain byte-preserving.
fn remove_tight_quote_lines(lines: &mut [String], literal_lines: &[bool]) {
    let mut active_quote_depth = 0;
    for (line, literal) in lines.iter_mut().zip(literal_lines) {
        if *literal {
            let (body, _) = split_line(line);
            let depth = valid_quote_depth(body);
            if depth > 0 {
                active_quote_depth = depth;
            }
            continue;
        }

        let (body, ending) = split_line(line);
        let trimmed = body.trim_start_matches([' ', '\t']);
        let Some(after_marker) = trimmed.strip_prefix('>') else {
            active_quote_depth = 0;
            continue;
        };
        if after_marker.is_empty() || after_marker.starts_with([' ', '\t', '\r']) {
            active_quote_depth = valid_quote_depth(body);
            continue;
        }

        let indentation = &body[..body.len() - trimmed.len()];
        if active_quote_depth == 0 {
            *line = ending.to_owned();
        } else {
            *line = format!("{indentation}{}{ending}", "> ".repeat(active_quote_depth));
        }
    }
}

fn valid_quote_depth(body: &str) -> usize {
    let mut depth = 0;
    let mut rest = body.trim_start_matches([' ', '\t']);
    while let Some(after_marker) = rest.strip_prefix('>') {
        if after_marker.is_empty() || after_marker.starts_with('\r') {
            return depth + 1;
        }
        if !after_marker.starts_with([' ', '\t']) {
            break;
        }
        depth += 1;
        rest = after_marker.trim_start_matches([' ', '\t']);
    }
    depth
}

/// Move a prematurely crossed bold closer behind its size closer.
///
/// Wikidot renders `**[[size 110%]]text**[[/size]]` as properly nested
/// `<strong><span>text</span></strong>`. Canonicalizing this corpus-backed
/// shape also prevents repeated crossed delimiters from causing exponential
/// parser backtracking.
fn canonicalize_crossed_bold_size_closers(lines: &mut [String], literal_lines: &[bool]) {
    const OPEN: &str = "**[[size ";
    const CROSSED_CLOSE: &str = "**[[/size]]";

    for (line, literal) in lines.iter_mut().zip(literal_lines) {
        if *literal {
            continue;
        }

        let mut search_from = 0;
        while let Some(open_start) = line[search_from..].find(OPEN) {
            let open_start = search_from + open_start;
            let Some(open_end) = line[open_start + OPEN.len()..].find("]]") else {
                break;
            };
            let body_start = open_start + OPEN.len() + open_end + 2;
            let Some(close_start) = line[body_start..].find(CROSSED_CLOSE) else {
                break;
            };
            let close_start = body_start + close_start;

            // A second bold delimiter makes the intended pairing ambiguous.
            // Leave such lines to the normal parser.
            if line[body_start..close_start].contains("**") {
                search_from = body_start;
                continue;
            }

            line.replace_range(
                close_start..close_start + CROSSED_CLOSE.len(),
                "[[/size]]**",
            );
            search_from = close_start + "[[/size]]**".len();
        }
    }
}

#[derive(Debug)]
struct QuotedCollapsible {
    prefix: String,
    quote_depth: usize,
}

fn canonicalize_unquoted_collapsible_closers(
    lines: &mut [String],
    literal_lines: &[bool],
) {
    let mut openers: Vec<QuotedCollapsible> = Vec::new();

    for (index, line) in lines.iter_mut().enumerate() {
        let (body, ending) = split_line(line);
        let (line_quote_depth, _) = quote_depth_and_body(body);

        // Literal contents cannot contain compatibility markers, but their
        // physical quote depth still terminates a quoted candidate when the
        // parser has returned to a shallower context.
        if literal_lines[index] {
            while openers
                .last()
                .is_some_and(|opener| opener.quote_depth > line_quote_depth)
            {
                openers.pop();
            }
            continue;
        }

        if body.trim().is_empty() {
            openers.clear();
            continue;
        }

        let marker = marker_line(line);
        if marker == Some(("", Marker::CollapsibleClose)) {
            if let Some(opener) = openers.pop() {
                *line = format!("{}[[/collapsible]]{ending}", opener.prefix);
            }
            continue;
        }

        while openers
            .last()
            .is_some_and(|opener| opener.quote_depth > line_quote_depth)
        {
            openers.pop();
        }

        match marker {
            Some((prefix, Marker::CollapsibleOpen)) => {
                let quote_depth =
                    prefix.chars().filter(|&character| character == '>').count();
                if quote_depth > 0 {
                    openers.push(QuotedCollapsible {
                        prefix: prefix.to_owned(),
                        quote_depth,
                    });
                }
            }
            Some((prefix, Marker::CollapsibleClose)) => {
                while let Some(opener) = openers.last() {
                    if opener.quote_depth < line_quote_depth {
                        break;
                    }
                    let matching_prefix = opener.prefix == prefix;
                    openers.pop();
                    if matching_prefix {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
}

#[derive(Debug)]
struct CrossedClose {
    prefix: String,
    quote_depth: usize,
    early_center_close: Option<usize>,
}

fn canonicalize_crossed_center_collapsible_closers(
    lines: &mut [String],
    literal_lines: &[bool],
) {
    let mut pending: Option<CrossedClose> = None;
    let mut index = 0;

    while index < lines.len() {
        if pending.as_ref().is_some_and(|candidate| {
            let (body, _) = split_line(&lines[index]);
            let (line_quote_depth, _) = quote_depth_and_body(body);
            line_quote_depth < candidate.quote_depth
        }) {
            pending = None;
        }

        if literal_lines[index] {
            index += 1;
            continue;
        }

        let marker = marker_line(&lines[index])
            .map(|(prefix, marker)| (prefix.to_owned(), marker));
        if let (Some(candidate), Some((prefix, marker))) = (&mut pending, &marker)
            && prefix == &candidate.prefix
        {
            match marker {
                Marker::CenterClose if candidate.early_center_close.is_none() => {
                    candidate.early_center_close = Some(index);
                    index += 1;
                    continue;
                }
                Marker::CollapsibleClose if candidate.early_center_close.is_some() => {
                    let early = candidate
                        .early_center_close
                        .expect("crossed close candidate has an early closer");
                    let (_, early_ending) = split_line(&lines[early]);
                    lines[early] = format!("{}{early_ending}", candidate.prefix);

                    let (_, late_ending) = split_line(&lines[index]);
                    lines[index] = format!(
                        "{}[[/collapsible]]\n{}[[/=]]{late_ending}",
                        candidate.prefix, candidate.prefix,
                    );
                    pending = None;
                    index += 1;
                    continue;
                }
                Marker::CenterOpen | Marker::CollapsibleOpen => pending = None,
                _ => {}
            }
        }

        if marker
            .as_ref()
            .is_some_and(|(_, marker)| *marker == Marker::CenterOpen)
            && index + 1 < lines.len()
            && !literal_lines[index + 1]
        {
            let (prefix, _) = marker.expect("center marker exists");
            if marker_line(&lines[index + 1])
                == Some((prefix.as_str(), Marker::CollapsibleOpen))
            {
                let quote_depth = prefix.bytes().filter(|&byte| byte == b'>').count();
                pending = Some(CrossedClose {
                    prefix,
                    quote_depth,
                    early_center_close: None,
                });
                index += 2;
                continue;
            }
        }

        index += 1;
    }
}

fn canonicalize_unmatched_quoted_tab_closers(
    lines: &mut [String],
    literal_lines: &[bool],
) {
    let has_tab_open = lines
        .iter()
        .zip(literal_lines)
        .any(|(line, literal)| !literal && line.to_ascii_lowercase().contains("[[tab "));
    let has_tabview_open = lines.iter().zip(literal_lines).any(|(line, literal)| {
        !literal && line.to_ascii_lowercase().contains("[[tabview]]")
    });

    for (index, line) in lines.iter_mut().enumerate() {
        if literal_lines[index] {
            continue;
        }
        let ending = if line.ends_with('\n') { "\n" } else { "" };
        let (body, _) = split_line(line);
        let Some(marker_start) = body.find("[[") else {
            continue;
        };
        let prefix = body[..marker_start].to_owned();
        if !prefix.chars().all(|ch| matches!(ch, '>' | ' ' | '\t')) {
            continue;
        }

        let marker = &body[marker_start..];
        let unmatched = (marker.eq_ignore_ascii_case("[[/tab]]") && !has_tab_open)
            || (marker.eq_ignore_ascii_case("[[/tabview]]") && !has_tabview_open);
        if unmatched {
            *line = format!("{prefix}{ending}");
        }
    }
}

fn literal_line_mask(lines: &[String]) -> Vec<bool> {
    #[derive(Clone, Copy)]
    struct LiteralBlock {
        close: &'static str,
        quote_depth: usize,
    }

    let mut block: Option<LiteralBlock> = None;
    let mut in_comment = false;
    let mut mask = Vec::with_capacity(lines.len());

    for line in lines {
        let (body, _) = split_line(line);
        let (quote_depth, logical) = quote_depth_and_body(body);
        let lower = logical.to_ascii_lowercase();

        if let Some(literal) = block {
            // A native quoted raw-text collector fails at the first shallower
            // physical line. That boundary belongs to the surrounding page,
            // so process it normally instead of masking it with the stale
            // literal candidate.
            if literal.quote_depth > 0 && quote_depth < literal.quote_depth {
                block = None;
            } else {
                mask.push(true);
                // Root-level collectors see a closer at any quote depth.
                // Quoted collectors only accept one at their exact depth.
                let close_depth_matches =
                    literal.quote_depth == 0 || quote_depth == literal.quote_depth;
                if close_depth_matches && lower.contains(literal.close) {
                    block = None;
                }
                continue;
            }
        }

        if in_comment {
            mask.push(true);
            if logical.contains("--]") {
                in_comment = false;
            }
            continue;
        }

        if is_tight_first_quote(body) {
            mask.push(false);
            continue;
        }

        if let Some(close) = literal_block_close(&lower) {
            mask.push(true);
            if !lower.contains(close) {
                block = Some(LiteralBlock { close, quote_depth });
            }
            continue;
        }

        if let Some(open) = logical.find("[!--") {
            mask.push(true);
            if !logical[open + 4..].contains("--]") {
                in_comment = true;
            }
            continue;
        }

        let raw_markers = logical.matches("@@").count();
        if raw_markers > 0 {
            mask.push(true);
            continue;
        }

        mask.push(false);
    }

    mask
}

fn quote_depth_and_body(mut body: &str) -> (usize, &str) {
    let mut quote_depth = 0;
    body = body.trim_start_matches([' ', '\t']);
    while let Some(rest) = body.strip_prefix('>') {
        quote_depth += 1;
        body = rest.trim_start_matches([' ', '\t']);
    }
    (quote_depth, body)
}

fn is_tight_first_quote(body: &str) -> bool {
    let body = body.trim_start_matches([' ', '\t']);
    let Some(rest) = body.strip_prefix('>') else {
        return false;
    };
    !rest.is_empty() && !rest.starts_with([' ', '\t', '\r'])
}

fn literal_block_close(lower: &str) -> Option<&'static str> {
    let marker = lower.strip_prefix("[[")?.trim_start();
    let head = marker.split_once("]]")?.0.trim_end();
    let mut words = head.split_ascii_whitespace();

    match words.next()? {
        "code" => Some("[[/code]]"),
        "raw" => Some("[[/raw]]"),
        "html" => Some("[[/html]]"),
        "math" => Some("[[/math]]"),
        // Named service embeds are single blocks. Only the empty-head form
        // introduces a literal body terminated by [[/embed]].
        "embed" if words.next().is_none() => Some("[[/embed]]"),
        "module" if words.next() == Some("css") => Some("[[/module]]"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::PageInfo;
    use crate::layout::Layout;
    use crate::render::{Render, html::HtmlRender};
    use crate::settings::{WikitextMode, WikitextSettings};

    #[test]
    fn tight_quote_lines_are_consumed_but_spaced_quotes_render() {
        let mut source = concat!(
            ">ALPHA_PLAIN_TIGHT\n",
            ">**ALPHA_BOLD_TIGHT**\n",
            ">[[[https://example.com | ALPHA_LINK_TIGHT]]]\n",
            ">[[div]]\n",
            ">ALPHA_DIV_TIGHT\n",
            ">[[/div]]\n",
            "> ALPHA_PLAIN_SPACED\n",
            "> **ALPHA_BOLD_SPACED**\n",
            "> [[[https://example.com | ALPHA_LINK_SPACED]]]\n",
            "> [[div]]\n",
            "> ALPHA_DIV_SPACED\n",
            "> [[/div]]\n",
        )
        .to_owned();

        substitute(&mut source);

        assert_eq!(
            source,
            concat!(
                "\n\n\n\n\n\n",
                "> ALPHA_PLAIN_SPACED\n",
                "> **ALPHA_BOLD_SPACED**\n",
                "> [[[https://example.com | ALPHA_LINK_SPACED]]]\n",
                "> [[div]]\n",
                "> ALPHA_DIV_SPACED\n",
                "> [[/div]]\n",
            ),
        );

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(!html.contains("_TIGHT"), "{html}");
        assert!(html.contains("ALPHA_PLAIN_SPACED"), "{html}");
        assert!(html.contains("ALPHA_BOLD_SPACED"), "{html}");
        assert!(html.contains("ALPHA_LINK_SPACED"), "{html}");
        assert!(html.contains("ALPHA_DIV_SPACED"), "{html}");
    }

    #[test]
    fn tight_quote_text_inside_root_literal_blocks_is_preserved() {
        let mut source = concat!(
            "[[code]]\n",
            ">ALPHA_CODE_LITERAL\n",
            "[[/code]]\n",
            "[[raw]]\n",
            ">ALPHA_RAW_LITERAL\n",
            "[[/raw]]\n",
        )
        .to_owned();
        let original = source.clone();

        substitute(&mut source);

        assert_eq!(source, original);
    }

    #[test]
    fn standalone_tight_quote_canonicalizes_to_a_pruned_empty_quote() {
        let mut source = ">ALPHA_TIGHT\n".to_owned();

        substitute(&mut source);
        assert_eq!(source, "\n");

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(!html.contains("ALPHA_TIGHT"), "{html}");
        assert!(html.trim().is_empty(), "{html}");
    }

    #[test]
    fn discarded_tight_quote_retains_a_paragraph_boundary() {
        let mut source = "BEFORE_MARKER\n>DROP_MARKER\nAFTER_MARKER".to_owned();

        substitute(&mut source);
        assert_eq!(source, "BEFORE_MARKER\n\nAFTER_MARKER");

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(html.matches("<p>").count(), 2, "{html}");
        assert!(!html.contains("DROP_MARKER"), "{html}");
    }

    #[test]
    fn nested_spaced_quote_keeps_a_tight_marker_as_literal_content() {
        let mut source = "> >ALPHA_NESTED_LITERAL\n".to_owned();

        substitute(&mut source);

        assert_eq!(source, "> >ALPHA_NESTED_LITERAL\n");
        assert_eq!(quote_depth_and_body(&source), (2, "ALPHA_NESTED_LITERAL\n"));
    }

    #[test]
    fn quoted_crossed_center_and_collapsible_closers_are_canonicalized() {
        // Corpus provenance: scp-wiki/gears-ground-slowly.
        let mut source = concat!(
            "> [[=]]\n",
            "> [[collapsible show=\"poetry\" hide=\"poetry\"]]\n",
            "> [[/=]]\n",
            "> originally written here\n",
            "> [[/collapsible]]\n",
            "outside\n",
        )
        .to_owned();

        substitute(&mut source);
        assert_eq!(
            source,
            concat!(
                "> [[=]]\n",
                "> [[collapsible show=\"poetry\" hide=\"poetry\"]]\n",
                "> \n",
                "> originally written here\n",
                "> [[/collapsible]]\n",
                "> [[/=]]\n",
                "outside\n",
            ),
        );

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains("originally written here"), "{html}");
        assert!(html.contains("text-align: center"), "{html}");
        assert!(html.contains("wj-collapsible"), "{html}");
        assert!(html.contains("outside"), "{html}");
    }

    #[test]
    fn ordinary_center_and_collapsible_blocks_are_unchanged() {
        let mut source = concat!(
            "[[=]]\n",
            "centered\n",
            "[[/=]]\n",
            "[[collapsible show=\"open\" hide=\"close\"]]\n",
            "body\n",
            "[[/collapsible]]\n",
        )
        .to_owned();
        let original = source.clone();

        substitute(&mut source);

        assert_eq!(source, original);
    }

    #[test]
    fn crossed_bold_and_size_closers_are_canonicalized() {
        // Corpus provenance: scp-wiki/scp-007-int.
        let mut source = concat!(
            "**[[size 120%]]SITE PT1[[/size]]**\n",
            "**[[size 110%]]OVERWATCH COUNCIL**[[/size]]\n",
            "**[[size 110%]]CABINET OFFICE**[[/size]]\n",
        )
        .repeat(6);

        substitute(&mut source);

        assert!(!source.contains("COUNCIL**[[/size]]"), "{source}");
        assert!(source.contains("COUNCIL[[/size]]**"), "{source}");

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains("<strong><span style=\"font-size: 110%;\">OVERWATCH COUNCIL</span></strong>"), "{html}");
    }

    #[test]
    fn crossed_bold_and_size_markers_inside_literals_are_unchanged() {
        let mut source = concat!(
            "[[code]]\n",
            "**[[size 110%]]literal**[[/size]]\n",
            "[[/code]]\n",
        )
        .to_owned();
        let original = source.clone();

        substitute(&mut source);

        assert_eq!(source, original);
    }

    #[test]
    fn quoted_collapsible_accepts_corpus_unquoted_standalone_closer() {
        // Corpus provenance: scp-wiki/foundation-missed-connections.
        let mut source = concat!(
            "> [[collapsible show=\"open\" hide=\"close\"]]\n",
            ">user example\n",
            ">------\n",
            "> body\n",
            "[[/collapsible]]\n",
            "outside\n",
        )
        .to_owned();

        substitute(&mut source);
        assert!(source.contains("> [[/collapsible]]\n"), "{source}");

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains("wj-collapsible"), "{html}");
        assert!(!html.contains("user example"), "{html}");
        assert!(html.contains("outside"), "{html}");
    }

    #[test]
    fn unquoted_content_boundary_is_not_absorbed_into_quoted_collapsible() {
        let mut source = concat!(
            "> [[collapsible show=\"open\"]]\n",
            "unquoted body\n",
            "[[/collapsible]]\n",
        )
        .to_owned();
        let original = source.clone();

        substitute(&mut source);

        assert_eq!(source, original);
    }

    #[test]
    fn unmatched_quoted_tab_closers_are_invisible_like_wikidot() {
        // Corpus provenance: scp-wiki/foundation-missed-connections.
        let mut source = concat!(
            "> template body\n",
            ">[[/tab]]\n",
            ">[[/tabview]]\n",
            "outside\n",
        )
        .to_owned();

        substitute(&mut source);
        assert!(!source.contains("[[/tab"), "{source}");

        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(&source);
        let (tree, errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;

        assert!(errors.is_empty(), "{errors:#?}");
        assert!(html.contains("template body"), "{html}");
        assert!(html.contains("outside"), "{html}");
    }

    #[test]
    fn compatibility_markers_inside_literal_regions_are_unchanged() {
        for mut source in [
            concat!(
                "[[code]]\n",
                "[[=]]\n",
                "[[collapsible show=\"sample\"]]\n",
                "[[/=]]\n",
                "[[/collapsible]]\n",
                "[[/tab]]\n",
                "[[/code]]\n",
            )
            .to_owned(),
            concat!("> [[raw]]\n", "> [[/tabview]]\n", "> [[/raw]]\n",).to_owned(),
            concat!("[!--\n", "[[/tabview]]\n", "--]\n").to_owned(),
        ] {
            let original = source.clone();

            substitute(&mut source);

            assert_eq!(source, original);
        }
    }

    #[test]
    fn deeper_quote_closer_does_not_end_a_literal_block() {
        let mut source = concat!(
            "> [[code]]\n",
            ">> [[/code]]\n",
            "> [[/tab]]\n",
            "> [[/code]]\n",
        )
        .to_owned();
        let original = source.clone();

        substitute(&mut source);

        assert_eq!(source, original);
    }

    #[test]
    fn shallower_quote_boundary_ends_a_literal_candidate() {
        let mut source =
            concat!("> [[code]]\n", "[[/tab]]\n", "> [[/code]]\n",).to_owned();

        substitute(&mut source);

        assert_eq!(source, concat!("> [[code]]\n", "\n", "> [[/code]]\n"));
    }

    #[test]
    fn inline_raw_escape_does_not_mask_later_physical_lines() {
        let mut source = concat!("@@\n", "[[/tab]]\n", "@@\n").to_owned();

        substitute(&mut source);

        assert_eq!(source, "@@\n\n@@\n");
    }

    #[test]
    fn literal_line_at_shallower_depth_terminates_quoted_candidates() {
        for mut source in [
            concat!(
                "> [[collapsible show=\"open\"]]\n",
                "[[code]]\n",
                "literal body\n",
                "[[/code]]\n",
                "[[/collapsible]]\n",
            )
            .to_owned(),
            concat!(
                "> [[=]]\n",
                "> [[collapsible show=\"open\"]]\n",
                "[[code]]\n",
                "literal body\n",
                "[[/code]]\n",
                "> [[/=]]\n",
                "> [[/collapsible]]\n",
            )
            .to_owned(),
        ] {
            let original = source.clone();

            substitute(&mut source);

            assert_eq!(source, original);
        }
    }

    #[test]
    fn root_literal_block_accepts_a_quote_prefixed_closer() {
        let lines = concat!("[[code]]\n", "> [[/code]]\n", "[[/tab]]\n",)
            .split_inclusive('\n')
            .map(str::to_owned)
            .collect::<Vec<_>>();

        assert_eq!(literal_line_mask(&lines), [true, true, false]);
    }

    #[test]
    fn literal_block_detection_requires_an_exact_block_name_and_shape() {
        for lookalike in [
            "[[codeexample]]",
            "[[raw-data]]",
            "[[html5]]",
            "[[mathref theorem]]",
            "[[equation theorem]]",
            "[[equationref theorem]]",
            "[[embed youtube video=abc]]",
            "[[module css-reset]]",
        ] {
            assert_eq!(literal_block_close(lookalike), None, "{lookalike}");
        }

        for (opener, closer) in [
            ("[[code]]", "[[/code]]"),
            ("[[code type=rust]]", "[[/code]]"),
            ("[[ raw ]]", "[[/raw]]"),
            ("[[html class=frame]]", "[[/html]]"),
            ("[[math quadratic]]", "[[/math]]"),
            ("[[embed]]", "[[/embed]]"),
            ("[[module css]]", "[[/module]]"),
        ] {
            assert_eq!(literal_block_close(opener), Some(closer), "{opener}");
        }
    }

    #[test]
    fn compatibility_scans_adversarial_marker_runs_in_bounded_time() {
        const MARKERS: usize = 8_192;

        let quoted_open = "> [[collapsible show=\"open\"]]\n";
        let crossed_open = "> [[=]]\n> [[collapsible show=\"open\"]]\n";
        for mut source in [quoted_open.repeat(MARKERS), crossed_open.repeat(MARKERS)] {
            let original = source.clone();
            let started = std::time::Instant::now();

            substitute(&mut source);

            assert!(
                started.elapsed() < std::time::Duration::from_secs(5),
                "compatibility scan took {:?}",
                started.elapsed(),
            );
            assert_eq!(source, original);
        }
    }
}
