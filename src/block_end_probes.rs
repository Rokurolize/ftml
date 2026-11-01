/*
 * block_end_closers.rs
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

//! Corpus-dependent probes for the block-end machinery. None of these run in
//! `verification/block-end/run.sh`: they read the shared Wikidot corpus and are
//! driven by environment variables. See the `verify-block-end` skill for the
//! procedures that use them.
//!
//! Always preprocess with `preprocess_for_layout(.., Layout::Wikidot)`; plain
//! `preprocess` is the native layout and skips every Wikidot compatibility rule.

use crate::data::PageInfo;
use crate::layout::Layout;
use crate::settings::{WikitextMode, WikitextSettings};
use serde_json::Value;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Probe: render the file named by `BLOCK_END_PROBE` and print math/bibliography markers.
#[test]
#[ignore = "probe"]
fn probe_render_file() {
    use crate::render::{Render, html::HtmlRender};
    let Ok(path) = std::env::var("BLOCK_END_PROBE") else {
        return;
    };
    let mut source = std::fs::read_to_string(path).expect("probe file");
    crate::preprocess_for_layout(&mut source, Layout::Wikidot);
    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let tokenization = crate::tokenize(&source);
    let (tree, _errors) = crate::parse(&tokenization, &page_info, &settings).into();
    let html = HtmlRender.render(&tree, &page_info, &settings).body;
    if std::env::var("BLOCK_END_HTML").is_ok() {
        println!("PROBE-HTML-BEGIN\n{html}\nPROBE-HTML-END");
    }
    for (i, c) in tree.code_blocks.iter().enumerate() {
        println!(
            "PROBE-CODE {i} {:?}",
            c.contents
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("")
        );
    }
    println!(
        "PROBE code_blocks={} div-code={}",
        tree.code_blocks.len(),
        html.matches("<div class=\"code\">").count()
    );
    println!(
        "PROBE math-equation={} bibitems={} literal-math-open={} literal-bib-open={}",
        html.matches("math-equation").count(),
        html.matches("bibitems").count(),
        html.matches("[[math").count(),
        html.matches("[[bibliography").count()
    );
}

/// Differential: Deepwell's raw `[[code]]` scan against the blocks FTML recognizes,
/// over the page list named by `BLOCK_END_PAGES` (one source path per line).
#[test]
#[ignore = "probe"]
fn raw_scan_vs_ftml_code_blocks() {
    let Ok(list) = std::env::var("BLOCK_END_PAGES") else {
        return;
    };
    let corpus = std::env::var("BLOCK_END_CORPUS").expect("corpus root");
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let (mut agree, mut scan_failed, mut disagree, mut total) = (0, 0, 0, 0);
    for path in std::fs::read_to_string(list).expect("list").lines() {
        let Ok(raw) = std::fs::read_to_string(format!("{corpus}/{path}")) else {
            continue;
        };
        total += 1;
        let mut page_info = PageInfo::dummy();
        let meta_path = format!(
            "{corpus}/{}",
            path.replace("source.wikidot.txt", "meta.json")
        );
        if let Ok(meta) = std::fs::read_to_string(meta_path) {
            if let Ok(meta) = serde_json::from_str::<Value>(&meta) {
                if let Some(tags) = meta.get("tags").and_then(Value::as_array) {
                    page_info.tags = tags
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|t| t.to_owned().into())
                        .collect();
                }
            }
        }
        let scan = crate::compat_scan_probe::scan_compat_code_blocks(&raw);
        let mut source = raw.clone();
        crate::preprocess_for_layout(&mut source, Layout::Wikidot);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let tokenization = crate::tokenize(&source);
            let (tree, _e) = crate::parse(&tokenization, &page_info, &settings).into();
            tree.code_blocks
                .iter()
                .map(|c| c.contents.to_string())
                .collect::<Vec<_>>()
        }));
        let Ok(ftml_blocks) = result else {
            println!("PANIC {path}");
            continue;
        };
        match scan {
            Err(_) => scan_failed += 1,
            Ok(blocks) if blocks.len() == ftml_blocks.len() => agree += 1,
            Ok(blocks) => {
                disagree += 1;
                println!(
                    "COUNT-MISMATCH {path} raw={} ftml={}",
                    blocks.len(),
                    ftml_blocks.len()
                );
            }
        }
    }
    println!(
        "SCAN-DIFF pages={total} agree={agree} scan_failed_closed={scan_failed} disagree={disagree}"
    );
}

/// Probe: for the file named by `BLOCK_END_PROBE`, find the nearest preceding line whose
/// presence makes FTML recognize fewer code blocks than the suffix starting after it.
#[test]
#[ignore = "probe"]
fn probe_code_suffix_bisect() {
    let Ok(path) = std::env::var("BLOCK_END_PROBE") else {
        return;
    };
    let raw = std::fs::read_to_string(path).expect("probe file");
    let lines: Vec<&str> = raw.split('\n').collect();
    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let count = |start: usize| {
        let mut source = lines[start..].join("\n");
        crate::preprocess_for_layout(&mut source, Layout::Wikidot);
        let tokenization = crate::tokenize(&source);
        let (tree, _e) = crate::parse(&tokenization, &page_info, &settings).into();
        tree.code_blocks.len()
    };
    let first_code = lines
        .iter()
        .position(|l| l.trim_start().to_ascii_lowercase().starts_with("[[code"))
        .unwrap();
    let want = count(first_code);
    let mut k = first_code;
    while k > 0 {
        k -= 1;
        if count(k) < want {
            println!(
                "BISECT line {} (1-based) breaks the block: {:?}",
                k + 1,
                lines[k]
            );
            let first = k.saturating_sub(1);
            for (i, l) in lines.iter().enumerate().skip(first).take(4) {
                println!("   {}: {:?}", i + 1, l);
            }
            return;
        }
    }
    println!(
        "BISECT no preceding line breaks it (want {want}, from-start {})",
        count(0)
    );
}

/// Probe: greedily delete lines of `BLOCK_END_PROBE` while FTML still recognizes fewer code
/// blocks than the number of `[[code` line-start openers remaining.
#[test]
#[ignore = "probe"]
fn probe_code_minimize() {
    let Ok(path) = std::env::var("BLOCK_END_PROBE") else {
        return;
    };
    let raw = std::fs::read_to_string(path).expect("probe file");
    let mut lines: Vec<String> = raw.split('\n').map(str::to_owned).collect();
    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let failing = |lines: &[String]| {
        let openers = lines
            .iter()
            .filter(|l| l.trim_start().to_ascii_lowercase().starts_with("[[code"))
            .count();
        let closers = lines
            .iter()
            .filter(|l| l.trim().to_ascii_lowercase() == "[[/code]]")
            .count();
        if openers == 0 || openers != closers {
            return false;
        }
        if std::env::var("BLOCK_END_BALANCE_FOOTNOTE").is_ok() {
            let text = lines.join("\n").to_ascii_lowercase();
            if text.matches("[[footnote]]").count()
                != text.matches("[[/footnote]]").count()
            {
                return false;
            }
        }
        if std::env::var("BLOCK_END_BALANCE_MATH").is_ok() {
            let mo = lines
                .iter()
                .filter(|l| l.trim_start().to_ascii_lowercase().starts_with("[[math"))
                .count();
            let mc = lines
                .iter()
                .filter(|l| l.trim().to_ascii_lowercase() == "[[/math]]")
                .count();
            if mo != mc {
                return false;
            }
        }
        let mut source = lines.join("\n");
        crate::preprocess_for_layout(&mut source, Layout::Wikidot);
        let tokenization = crate::tokenize(&source);
        let (tree, _e) = crate::parse(&tokenization, &page_info, &settings).into();
        tree.code_blocks.len() < openers
    };
    if !failing(&lines) {
        println!(
            "MINIMIZE page does not fail (needs equal opener/closer line counts and fewer recognized)"
        );
        return;
    }
    let mut chunk = lines.len() / 2;
    while chunk >= 1 {
        let mut i = 0;
        while i < lines.len() {
            let upto = (i + chunk).min(lines.len());
            let mut trial = lines.clone();
            trial.drain(i..upto);
            if failing(&trial) {
                lines = trial;
            } else {
                i += chunk;
            }
        }
        chunk /= 2;
    }
    println!("MINIMIZE-BEGIN\n{}\nMINIMIZE-END", lines.join("\n"));
}

/// Differential against retained Wikidot html: for each page path in `BLOCK_END_PAGES`
/// (`<lang>/pages/<slug>/source.wikidot.txt`) with an `html.xmlrpc.html` sibling, print the
/// Wikidot count and FTML count of code blocks and collapsibles.
#[test]
#[ignore = "probe"]
fn wikidot_html_counts() {
    use crate::render::{Render, html::HtmlRender};
    let Ok(list) = std::env::var("BLOCK_END_PAGES") else {
        return;
    };
    let corpus = std::env::var("BLOCK_END_CORPUS").expect("corpus root");
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let code_div =
        regex::Regex::new(r#"<div class="code">\s*(?:<div class="hl-main">\s*)?<pre"#)
            .unwrap();
    for path in std::fs::read_to_string(list).expect("list").lines() {
        let html_path = format!(
            "{corpus}/{}",
            path.replace("source.wikidot.txt", "html.xmlrpc.html")
        );
        let Ok(wikidot) = std::fs::read_to_string(&html_path) else {
            continue;
        };
        let Ok(raw) = std::fs::read_to_string(format!("{corpus}/{path}")) else {
            continue;
        };
        let mut page_info = PageInfo::dummy();
        if let Ok(meta) = std::fs::read_to_string(format!(
            "{corpus}/{}",
            path.replace("source.wikidot.txt", "meta.json")
        )) {
            if let Ok(meta) = serde_json::from_str::<Value>(&meta) {
                if let Some(tags) = meta.get("tags").and_then(Value::as_array) {
                    page_info.tags = tags
                        .iter()
                        .filter_map(Value::as_str)
                        .map(|t| t.to_owned().into())
                        .collect();
                }
            }
        }
        let mut source = raw;
        crate::preprocess_for_layout(&mut source, Layout::Wikidot);
        let result = catch_unwind(AssertUnwindSafe(|| {
            let tokenization = crate::tokenize(&source);
            let (tree, _e) = crate::parse(&tokenization, &page_info, &settings).into();
            HtmlRender.render(&tree, &page_info, &settings).body
        }));
        let Ok(html) = result else {
            println!("HTMLCOUNT {path} PANIC");
            continue;
        };
        println!(
            "HTMLCOUNT {path} code_w={} code_f={} coll_w={} coll_f={}",
            code_div.find_iter(&wikidot).count(),
            code_div.find_iter(&html).count(),
            wikidot.matches("collapsible-block\"").count(),
            html.matches("collapsible-block\"").count(),
        );
    }
}

/// Expected collapsible count under Wikidot's pure-nesting rule: the first closer closes the
/// open block and openers seen while one is open stay literal. Line-start markers only.
fn expected_collapsibles(lines: &[String]) -> Option<usize> {
    let (mut open, mut count) = (false, 0usize);
    for line in lines {
        let marker = line.trim().to_ascii_lowercase();
        let marker = marker.trim_start_matches(['>', ' ']);
        if marker.starts_with("[[collapsible") && marker.ends_with("]]") {
            open = true;
        } else if marker == "[[/collapsible]]" && open {
            open = false;
            count += 1;
        }
    }
    (!open).then_some(count)
}

/// Probe: delta-minimize `BLOCK_END_PROBE` while FTML renders fewer collapsibles than the
/// pure-nesting rule expects. A reduction that only differs by nesting is not a defect.
#[test]
#[ignore = "probe"]
fn probe_collapsible_minimize() {
    use crate::render::{Render, html::HtmlRender};
    let Ok(path) = std::env::var("BLOCK_END_PROBE") else {
        return;
    };
    let raw = std::fs::read_to_string(path).expect("probe file");
    let mut lines: Vec<String> = raw.split('\n').map(str::to_owned).collect();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let failing = |lines: &[String]| {
        let Some(expected) = expected_collapsibles(lines) else {
            return false;
        };
        if expected == 0 {
            return false;
        }
        let mut source = lines.join("\n");
        crate::preprocess_for_layout(&mut source, Layout::Wikidot);
        let page_info = PageInfo::dummy();
        let tokenization = crate::tokenize(&source);
        let (tree, _e) = crate::parse(&tokenization, &page_info, &settings).into();
        let html = HtmlRender.render(&tree, &page_info, &settings).body;
        html.matches("collapsible-block\"").count() < expected
    };
    if !failing(&lines) {
        println!("MINIMIZE-COLL page does not fail under the pure-nesting expectation");
        return;
    }
    let mut chunk = lines.len() / 2;
    while chunk >= 1 {
        let mut i = 0;
        while i < lines.len() {
            let upto = (i + chunk).min(lines.len());
            let mut trial = lines.clone();
            trial.drain(i..upto);
            if failing(&trial) {
                lines = trial;
            } else {
                i += chunk;
            }
        }
        chunk /= 2;
    }
    println!("MINIMIZE-BEGIN\n{}\nMINIMIZE-END", lines.join("\n"));
}
