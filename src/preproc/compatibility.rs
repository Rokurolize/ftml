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

    *text = lines.concat();
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
        if literal_lines[index] {
            continue;
        }

        let (body, ending) = split_line(line);
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

        let line_quote_depth = body
            .trim_start()
            .chars()
            .take_while(|&character| character == '>')
            .count();
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
    early_center_close: Option<usize>,
}

fn canonicalize_crossed_center_collapsible_closers(
    lines: &mut [String],
    literal_lines: &[bool],
) {
    let mut pending: Option<CrossedClose> = None;
    let mut index = 0;

    while index < lines.len() {
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
                pending = Some(CrossedClose {
                    prefix,
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
    let mut block_close: Option<&'static str> = None;
    let mut in_comment = false;
    let mut in_raw_escape = false;
    let mut mask = Vec::with_capacity(lines.len());

    for line in lines {
        let (body, _) = split_line(line);
        let logical = logical_line_body(body);
        let lower = logical.to_ascii_lowercase();

        if let Some(close) = block_close {
            mask.push(true);
            if lower.contains(close) {
                block_close = None;
            }
            continue;
        }

        if in_comment {
            mask.push(true);
            if logical.contains("--]") {
                in_comment = false;
            }
            continue;
        }

        if in_raw_escape {
            mask.push(true);
            if logical.matches("@@").count() % 2 == 1 {
                in_raw_escape = false;
            }
            continue;
        }

        if let Some(close) = literal_block_close(&lower) {
            mask.push(true);
            if !lower.contains(close) {
                block_close = Some(close);
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
            in_raw_escape = raw_markers % 2 == 1;
            continue;
        }

        mask.push(false);
    }

    mask
}

fn logical_line_body(mut body: &str) -> &str {
    body = body.trim_start_matches([' ', '\t']);
    while let Some(rest) = body.strip_prefix('>') {
        body = rest.trim_start_matches([' ', '\t']);
    }
    body
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
        assert!(html.contains("user example"), "{html}");
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
            concat!("@@\n", "[[/tab]]\n", "@@\n").to_owned(),
            concat!("[!--\n", "[[/tabview]]\n", "--]\n").to_owned(),
        ] {
            let original = source.clone();

            substitute(&mut source);

            assert_eq!(source, original);
        }
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
