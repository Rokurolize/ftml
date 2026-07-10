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
    canonicalize_unquoted_collapsible_closers(&mut lines);
    canonicalize_unmatched_quoted_tab_closers(&mut lines);
    let mut index = 0;

    while index + 1 < lines.len() {
        let Some((prefix, Marker::CenterOpen)) = marker_line(&lines[index]) else {
            index += 1;
            continue;
        };
        let prefix = prefix.to_owned();
        if marker_line(&lines[index + 1])
            != Some((prefix.as_str(), Marker::CollapsibleOpen))
        {
            index += 1;
            continue;
        }

        let mut early_center_close = None;
        let mut collapsible_close = None;
        for (scan, line) in lines.iter().enumerate().skip(index + 2) {
            let Some((line_prefix, marker)) = marker_line(line) else {
                continue;
            };
            if line_prefix != prefix {
                continue;
            }

            match marker {
                Marker::CenterOpen | Marker::CollapsibleOpen => break,
                Marker::CenterClose if early_center_close.is_none() => {
                    early_center_close = Some(scan);
                }
                Marker::CollapsibleClose if early_center_close.is_some() => {
                    collapsible_close = Some(scan);
                    break;
                }
                _ => {}
            }
        }

        let (Some(early), Some(late)) = (early_center_close, collapsible_close) else {
            index += 1;
            continue;
        };

        let (_, early_ending) = split_line(&lines[early]);
        lines[early] = format!("{prefix}{early_ending}");

        let late_has_ending = lines[late].ends_with('\n');
        if !late_has_ending {
            lines[late].push('\n');
        }
        let inserted_ending = if late_has_ending { "\n" } else { "" };
        lines.insert(late + 1, format!("{prefix}[[/=]]{inserted_ending}"));
        index = late + 2;
    }

    *text = lines.concat();
}

fn canonicalize_unquoted_collapsible_closers(lines: &mut [String]) {
    for open in 0..lines.len() {
        let Some((prefix, Marker::CollapsibleOpen)) = marker_line(&lines[open]) else {
            continue;
        };
        let prefix = prefix.to_owned();
        let quote_depth = prefix.chars().filter(|&ch| ch == '>').count();
        if quote_depth == 0 {
            continue;
        }

        for line in lines.iter_mut().skip(open + 1) {
            let ending = if line.ends_with('\n') { "\n" } else { "" };
            let (body, _) = split_line(line);
            if body.trim().is_empty() {
                break;
            }

            if marker_line(line) == Some(("", Marker::CollapsibleClose)) {
                *line = format!("{prefix}[[/collapsible]]{ending}");
                break;
            }

            let trimmed = body.trim_start();
            let line_quote_depth = trimmed.chars().take_while(|&ch| ch == '>').count();
            if line_quote_depth < quote_depth {
                break;
            }

            if marker_line(line)
                .is_some_and(|(_, marker)| marker == Marker::CollapsibleClose)
            {
                break;
            }
        }
    }
}

fn canonicalize_unmatched_quoted_tab_closers(lines: &mut [String]) {
    let has_tab_open = lines.iter().any(|line| line.contains("[[tab "));
    let has_tabview_open = lines.iter().any(|line| line.contains("[[tabview]]"));

    for line in lines {
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
        let unmatched = (marker == "[[/tab]]" && !has_tab_open)
            || (marker == "[[/tabview]]" && !has_tabview_open);
        if unmatched {
            *line = format!("{prefix}{ending}");
        }
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
}
