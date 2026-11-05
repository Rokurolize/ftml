//! Expand `[[include]]` blocks with `ftml::include`, the same entry point a
//! caller such as Wikijump uses, over page sources prefetched by
//! `scripts/fetch.py materialize`.
//!
//! ftml performs one expansion pass per call and leaves recursion to the
//! caller. Wikidot re-runs include expansion over the whole source until it
//! stops changing or a pass cap is reached, so this tool loops the same way.
//!
//! usage: include_expand <includes-dir> <current-site> [max-passes] < source > expanded
//! A JSON report of each pass is written to stderr.

use ftml::data::PageRef;
use ftml::includes::{FetchedPage, IncludeRef, Includer};
use ftml::layout::Layout;
use ftml::settings::{WikitextMode, WikitextSettings};
use serde_json::json;
use std::borrow::Cow;
use std::io::{self, Read, Write};
use std::path::PathBuf;

struct CacheIncluder<'a> {
    root: PathBuf,
    site: &'a str,
    seen: Vec<serde_json::Value>,
}

/// Mirrors `wd.unix_name` in scripts/wd.py (the fetch-side normaliser).
fn unix_name(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    let mut out = String::new();
    let mut dash = false;
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() || ch == ':' || ch == '_' || ch == '-' {
            if ch == '-' {
                if dash {
                    continue;
                }
                dash = true;
            } else {
                dash = false;
            }
            out.push(ch);
        } else if !dash {
            out.push('-');
            dash = true;
        }
    }
    out.split(':')
        .map(|part| part.trim_matches('-'))
        .collect::<Vec<_>>()
        .join(":")
}

impl CacheIncluder<'_> {
    fn location(&self, page_ref: &PageRef) -> (String, String) {
        let site = page_ref.site().unwrap_or(self.site).to_lowercase();
        (site, unix_name(page_ref.page()))
    }
}

impl<'t> Includer<'t> for &mut CacheIncluder<'_> {
    type Error = String;

    fn include_pages(
        &mut self,
        includes: &[IncludeRef<'t>],
    ) -> Result<Vec<FetchedPage<'t>>, String> {
        let mut pages = Vec::new();
        for include in includes {
            let (site, page) = self.location(include.page_ref());
            let path = self
                .root
                .join(&site)
                .join(page.replace(':', "__"))
                .join("source.ftml");
            let content = std::fs::read_to_string(&path).ok().map(Cow::Owned);
            let variables: serde_json::Map<String, serde_json::Value> = include
                .variables()
                .iter()
                .map(|(k, v)| (k.to_string(), json!(v)))
                .collect();
            self.seen.push(json!({
                "site": site,
                "page": page,
                "found": content.is_some(),
                "variables": variables,
            }));
            pages.push(FetchedPage {
                page_ref: include.page_ref().clone(),
                content,
            });
        }
        Ok(pages)
    }

    fn no_such_include(&mut self, page_ref: &PageRef) -> Result<Cow<'t, str>, String> {
        // Caller-owned message. Kept distinctive so a missing include is visible
        // in the diff instead of silently vanishing.
        let (site, page) = self.location(page_ref);
        Ok(Cow::Owned(format!(
            "[[div class=\"error-block\"]]\nSWEEP-MISSING-INCLUDE {site}:{page}\n[[/div]]"
        )))
    }
}

fn main() -> io::Result<()> {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("includes dir"));
    let site = args.next().expect("current site");
    let max_passes: usize = args.next().map_or(5, |n| n.parse().expect("max passes"));

    let mut source = String::new();
    io::stdin().read_to_string(&mut source)?;

    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    let mut report = Vec::new();
    let mut passes = 0;
    while passes < max_passes {
        let mut includer = CacheIncluder {
            root: root.clone(),
            site: &site,
            seen: Vec::new(),
        };
        let (expanded, _pages) =
            ftml::include(&source, &settings, &mut includer, || "invalid".to_owned())
                .map_err(io::Error::other)?;
        passes += 1;
        let changed = expanded != source;
        report.push(json!({"pass": passes, "changed": changed, "includes": includer.seen}));
        source = expanded;
        if !changed {
            break;
        }
    }

    io::stdout().write_all(source.as_bytes())?;
    eprintln!("{}", json!({"passes": report, "max_passes": max_passes}));
    Ok(())
}
