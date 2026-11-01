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

//! Exhaustive closer-variant check for paired blocks (code, math, footnote,
//! collapsible) in the Wikidot layout.
//!
//! Properties, for a block whose closer is followed only by spaces and tabs
//! to the end of its line (Wikidot accepts that closer exactly like a bare one):
//!
//! * P0 recognition: the block is parsed as a block, as it is with a bare closer.
//! * P1 termination: text after the closer is never inside the block.
//! * P2 closer equivalence: the parse summary equals the bare-closer summary.
//!
//! Every combination of block type, enclosing context, body, trailing blanks
//! and line ending below is parsed on the real parser. Run through
//! `verification/block-end/run.sh`; see the `verify-block-end` skill. The test is
//! `#[ignore]` so ordinary `cargo test` does not run it.

use crate::data::PageInfo;
use crate::layout::Layout;
use crate::settings::{WikitextMode, WikitextSettings};
use serde_json::Value;
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

const SENTINEL: &str = "ZZTAILZZ";

struct Block {
    name: &'static str,
    opener: &'static str,
    closer: &'static str,
    /// The `element` tag of the block in the serialized tree (None for footnotes,
    /// which live in `SyntaxTree::footnotes`).
    tag: Option<&'static str>,
}

const BLOCKS: &[Block] = &[
    Block { name: "code", opener: "[[code]]", closer: "[[/code]]", tag: Some("code") },
    Block { name: "math", opener: "[[math]]", closer: "[[/math]]", tag: Some("math") },
    Block {
        name: "footnote",
        opener: "[[footnote]]",
        closer: "[[/footnote]]",
        tag: None,
    },
    Block {
        name: "bibliography",
        opener: "[[bibliography]]",
        closer: "[[/bibliography]]",
        tag: Some("bibliography-block"),
    },
    Block {
        name: "collapsible",
        opener: "[[collapsible show=\"s\" hide=\"h\"]]",
        closer: "[[/collapsible]]",
        tag: Some("collapsible"),
    },
];

/// Wrapping contexts: text before, text after (the tail goes inside the context).
const CONTEXTS: &[(&str, &str, &str)] = &[
    ("top", "", ""),
    ("div", "[[div]]\n", "\n[[/div]]"),
    ("collapsible", "[[collapsible show=\"o\" hide=\"c\"]]\n", "\n[[/collapsible]]"),
    ("footnote", "[[footnote]]\n", "\n[[/footnote]]"),
];

/// Bodies for bibliographies, whose items are `: label : text` lines.
const BIBLIOGRAPHY_BODIES: &[&str] = &[": a : text", ": a : x\n: b : y", ": a : **b**"];

const BODIES: &[&str] = &["", "x", "x\ny", "[[/cod", "**b**", "a ]] b", "x\n\ny"];
const BLANKS: &[&str] = &["", " ", "\t", "  ", " \t", "\t "];
const LINE_ENDINGS: &[&str] = &["\n", "\r\n", "\n\n"];

fn ancestors_of_sentinel(value: &Value, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match value {
        Value::String(text) if text.contains(SENTINEL) => out.push(path.clone()),
        Value::Array(items) => {
            for item in items {
                ancestors_of_sentinel(item, path, out);
            }
        }
        Value::Object(map) => {
            let tag = map.get("element").and_then(Value::as_str).map(str::to_owned);
            if let Some(tag) = &tag {
                path.push(tag.clone());
            }
            for (key, item) in map {
                ancestors_of_sentinel(item, path, out);
                let _ = key;
            }
            if tag.is_some() {
                path.pop();
            }
        }
        _ => {}
    }
}

fn count_tag(value: &Value, tag: &str) -> usize {
    match value {
        Value::Array(items) => items.iter().map(|item| count_tag(item, tag)).sum(),
        Value::Object(map) => {
            let own = usize::from(map.get("element").and_then(Value::as_str) == Some(tag));
            own + map.values().map(|item| count_tag(item, tag)).sum::<usize>()
        }
        _ => 0,
    }
}

/// What the parser did with a source, reduced to what the properties compare.
#[derive(Debug, PartialEq, Eq, Clone)]
struct Summary {
    /// How many blocks of the target type were recognized.
    recognized: usize,
    /// Registered code block bodies (code only).
    code_bodies: Vec<String>,
    /// Whether the tail sentinel sits inside a block of the target type.
    tail_inside_target: bool,
    /// Distinct element-tag chains above the sentinel.
    sentinel_chains: Vec<Vec<String>>,
}

fn summarize(block: &Block, source: &str) -> Result<Summary, String> {
    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let result = catch_unwind(AssertUnwindSafe(|| {
        let tokenization = crate::tokenize(source);
        let (tree, _errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let json = serde_json::to_value(&tree.elements).expect("tree serializes");
        let footnotes = serde_json::to_value(&tree.footnotes).expect("footnotes serialize");
        let mut chains = Vec::new();
        ancestors_of_sentinel(&json, &mut Vec::new(), &mut chains);
        let mut footnote_hits = Vec::new();
        ancestors_of_sentinel(&footnotes, &mut vec!["footnote".to_owned()], &mut footnote_hits);
        let (recognized, tail_inside_target) = match block.tag {
            Some(tag) => (
                count_tag(&json, tag),
                chains.iter().any(|chain| chain.iter().any(|t| t == tag)),
            ),
            None => (tree.footnotes.len(), !footnote_hits.is_empty()),
        };
        chains.extend(footnote_hits);
        chains.sort();
        chains.dedup();
        Summary {
            recognized,
            code_bodies: tree.code_blocks.iter().map(|c| c.contents.to_string()).collect(),
            tail_inside_target,
            sentinel_chains: chains,
        }
    }));
    result.map_err(|_| "panic".to_owned())
}

fn build(block: &Block, context: usize, body: &str, blanks: &str, eol: &str) -> String {
    let (before, after) = (CONTEXTS[context].1, CONTEXTS[context].2);
    // The closer owns its line, so that code (which requires line ownership)
    // is a valid candidate in every enumerated shape.
    format!(
        "{before}{opener}\n{body}\n{closer}{blanks}{eol}{SENTINEL}{after}",
        opener = block.opener,
        closer = block.closer,
    )
}

#[test]
#[ignore = "exhaustive check; run through verification/block-end/run.sh"]
fn closer_blanks_recognize_terminate_and_match_bare() {
    // message -> (shortest source, count)
    let mut findings: BTreeMap<String, (String, usize)> = BTreeMap::new();
    let mut parses = 0usize;
    for block in BLOCKS {
        for (context, (context_name, _, _)) in CONTEXTS.iter().enumerate() {
            if *context_name == block.name {
                continue; // a footnote or collapsible cannot be tested inside itself
            }
            let bodies = if block.name == "bibliography" { BIBLIOGRAPHY_BODIES } else { BODIES };
            for body in bodies {
                for eol in LINE_ENDINGS {
                    let bare_source = build(block, context, body, "", eol);
                    let bare = summarize(block, &bare_source);
                    parses += 1;
                    for blanks in BLANKS.iter().filter(|b| !b.is_empty()) {
                        let source = build(block, context, body, blanks, eol);
                        let variant = summarize(block, &source);
                        parses += 1;
                        let mut report = |class: &str| {
                            let message = format!(
                                "{} in {} body={:?}: {}",
                                block.name,
                                context_name,
                                body,
                                class
                            );
                            let entry = findings
                                .entry(message)
                                .or_insert_with(|| (source.clone(), 0));
                            entry.1 += 1;
                            if source.len() < entry.0.len() {
                                entry.0 = source.clone();
                            }
                        };
                        match (&bare, &variant) {
                            (Err(_), _) | (_, Err(_)) => report("P0 panic"),
                            (Ok(bare), Ok(variant)) => {
                                if bare.recognized > 0 && variant.recognized == 0 {
                                    report("P0 block not recognized with trailing blanks");
                                }
                                if variant.tail_inside_target && !bare.tail_inside_target {
                                    report("P1 tail swallowed by the block");
                                }
                                if bare.recognized > 0
                                    && (bare.recognized != variant.recognized
                                        || bare.code_bodies != variant.code_bodies
                                        || bare.sentinel_chains != variant.sentinel_chains)
                                {
                                    report("P2 summary differs from the bare closer");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    println!("ORDERS-BOUND {parses} parses over {} block types", BLOCKS.len());
    for (message, (source, count)) in &findings {
        println!("MINIMAL-FAILURE message={message} count={count} source={source:?}");
    }
}


/// Element-tag chain above each of the single-letter sentinels A..D.
fn sentinel_chain(value: &Value, letter: &str, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match value {
        Value::String(text) if text.split_whitespace().any(|w| w == letter) => {
            out.push(path.clone())
        }
        Value::Array(items) => {
            for item in items {
                sentinel_chain(item, letter, path, out);
            }
        }
        Value::Object(map) => {
            let tag = map.get("element").and_then(Value::as_str).map(str::to_owned);
            if let Some(tag) = &tag {
                path.push(tag.clone());
            }
            for item in map.values() {
                sentinel_chain(item, letter, path, out);
            }
            if tag.is_some() {
                path.pop();
            }
        }
        _ => {}
    }
}

/// P3 pairing: `OPEN A OPEN B CLOSE C CLOSE D`. Wikidot's first-opener ->
/// first-closer counting would close the outer block at the first closer,
/// leaving B inside it as literal text and C, D outside. A stack pairing nests
/// the inner block, so B is two deep and C is still inside the outer block.
/// For each block type this prints how many target ancestors each letter has.
#[test]
#[ignore = "exhaustive check; run through verification/block-end/run.sh"]
fn nested_same_name_pairing_depths() {
    for block in BLOCKS {
        let Some(tag) = block.tag else { continue };
        let source = format!(
            "{o}\nA\n{o}\nB\n{c}\nC\n{c}\nD",
            o = block.opener,
            c = block.closer
        );
        let page_info = PageInfo::dummy();
        let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
        let tokenization = crate::tokenize(&source);
        let (tree, _errors) = crate::parse(&tokenization, &page_info, &settings).into();
        let json = serde_json::to_value(&tree.elements).expect("tree serializes");
        let mut depths = Vec::new();
        for letter in ["A", "B", "C", "D"] {
            let mut chains = Vec::new();
            sentinel_chain(&json, letter, &mut Vec::new(), &mut chains);
            let d: Vec<usize> = chains
                .iter()
                .map(|c| c.iter().filter(|t| t.as_str() == tag).count())
                .collect();
            depths.push(format!("{letter}={d:?}"));
        }
        println!(
            "PAIRING {} depths {} blocks={} codes={}",
            block.name,
            depths.join(" "),
            count_tag(&json, tag),
            tree.code_blocks.len()
        );
    }
}

/// Probe: render the file named by `BLOCK_END_PROBE` and print math/bibliography markers.
#[test]
#[ignore = "probe"]
fn probe_render_file() {
    use crate::render::{Render, html::HtmlRender};
    let Ok(path) = std::env::var("BLOCK_END_PROBE") else { return };
    let mut source = std::fs::read_to_string(path).expect("probe file");
    crate::preprocess_for_layout(&mut source, Layout::Wikidot);
    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let tokenization = crate::tokenize(&source);
    let (tree, _errors) = crate::parse(&tokenization, &page_info, &settings).into();
    let html = HtmlRender.render(&tree, &page_info, &settings).body;
    if std::env::var("BLOCK_END_HTML").is_ok() { println!("PROBE-HTML-BEGIN\n{html}\nPROBE-HTML-END"); }
    for (i, c) in tree.code_blocks.iter().enumerate() { println!("PROBE-CODE {i} {:?}", c.contents.lines().find(|l| !l.trim().is_empty()).unwrap_or("")); }
    println!("PROBE code_blocks={} div-code={}", tree.code_blocks.len(), html.matches("<div class=\"code\">").count());
    println!("PROBE math-equation={} bibitems={} literal-math-open={} literal-bib-open={}",
        html.matches("math-equation").count(), html.matches("bibitems").count(),
        html.matches("[[math").count(), html.matches("[[bibliography").count());
}


/// Differential: Deepwell's raw `[[code]]` scan against the blocks FTML recognizes,
/// over the page list named by `BLOCK_END_PAGES` (one source path per line).
#[test]
#[ignore = "probe"]
fn raw_scan_vs_ftml_code_blocks() {
    let Ok(list) = std::env::var("BLOCK_END_PAGES") else { return };
    let corpus = std::env::var("BLOCK_END_CORPUS").expect("corpus root");
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let (mut agree, mut scan_failed, mut disagree, mut total) = (0, 0, 0, 0);
    for path in std::fs::read_to_string(list).expect("list").lines() {
        let Ok(raw) = std::fs::read_to_string(format!("{corpus}/{path}")) else { continue };
        total += 1;
        let mut page_info = PageInfo::dummy();
        let meta_path = format!("{corpus}/{}", path.replace("source.wikidot.txt", "meta.json"));
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
            tree.code_blocks.iter().map(|c| c.contents.to_string()).collect::<Vec<_>>()
        }));
        let Ok(ftml_blocks) = result else { println!("PANIC {path}"); continue };
        match scan {
            Err(_) => scan_failed += 1,
            Ok(blocks) if blocks.len() == ftml_blocks.len() => agree += 1,
            Ok(blocks) => {
                disagree += 1;
                println!("COUNT-MISMATCH {path} raw={} ftml={}", blocks.len(), ftml_blocks.len());
            }
        }
    }
    println!("SCAN-DIFF pages={total} agree={agree} scan_failed_closed={scan_failed} disagree={disagree}");
}

/// Probe: for the file named by `BLOCK_END_PROBE`, find the nearest preceding line whose
/// presence makes FTML recognize fewer code blocks than the suffix starting after it.
#[test]
#[ignore = "probe"]
fn probe_code_suffix_bisect() {
    let Ok(path) = std::env::var("BLOCK_END_PROBE") else { return };
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
    let first_code = lines.iter().position(|l| l.trim_start().to_ascii_lowercase().starts_with("[[code")).unwrap();
    let want = count(first_code);
    let mut k = first_code;
    while k > 0 {
        k -= 1;
        if count(k) < want {
            println!("BISECT line {} (1-based) breaks the block: {:?}", k + 1, lines[k]);
            for (i, l) in lines[k.saturating_sub(1)..=(k + 2).min(lines.len() - 1)].iter().enumerate() {
                println!("   {}: {:?}", k.saturating_sub(1) + i + 1, l);
            }
            return;
        }
    }
    println!("BISECT no preceding line breaks it (want {want}, from-start {})", count(0));
}

/// Probe: greedily delete lines of `BLOCK_END_PROBE` while FTML still recognizes fewer code
/// blocks than the number of `[[code` line-start openers remaining.
#[test]
#[ignore = "probe"]
fn probe_code_minimize() {
    let Ok(path) = std::env::var("BLOCK_END_PROBE") else { return };
    let raw = std::fs::read_to_string(path).expect("probe file");
    let mut lines: Vec<String> = raw.split('\n').map(str::to_owned).collect();
    let page_info = PageInfo::dummy();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let failing = |lines: &[String]| {
        let openers = lines.iter().filter(|l| l.trim_start().to_ascii_lowercase().starts_with("[[code")).count();
        let closers = lines.iter().filter(|l| l.trim().to_ascii_lowercase() == "[[/code]]").count();
        if openers == 0 || openers != closers { return false; }
        if std::env::var("BLOCK_END_BALANCE_FOOTNOTE").is_ok() {
            let text = lines.join("\n").to_ascii_lowercase();
            if text.matches("[[footnote]]").count() != text.matches("[[/footnote]]").count() { return false; }
        }
        if std::env::var("BLOCK_END_BALANCE_MATH").is_ok() {
            let mo = lines.iter().filter(|l| l.trim_start().to_ascii_lowercase().starts_with("[[math")).count();
            let mc = lines.iter().filter(|l| l.trim().to_ascii_lowercase() == "[[/math]]").count();
            if mo != mc { return false; }
        }
        let mut source = lines.join("\n");
        crate::preprocess_for_layout(&mut source, Layout::Wikidot);
        let tokenization = crate::tokenize(&source);
        let (tree, _e) = crate::parse(&tokenization, &page_info, &settings).into();
        tree.code_blocks.len() < openers
    };
    if !failing(&lines) { println!("MINIMIZE page does not fail (needs equal opener/closer line counts and fewer recognized)"); return; }
    let mut chunk = lines.len() / 2;
    while chunk >= 1 {
        let mut i = 0;
        while i < lines.len() {
            let upto = (i + chunk).min(lines.len());
            let mut trial = lines.clone();
            trial.drain(i..upto);
            if failing(&trial) { lines = trial; } else { i += chunk; }
        }
        chunk /= 2;
    }
    println!("MINIMIZE-BEGIN\n{}\nMINIMIZE-END", lines.join("\n"));
}
