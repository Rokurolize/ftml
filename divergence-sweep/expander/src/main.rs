//! stdin JSONL  {"case_id","source"}  ->  stdout JSONL {"case_id","expanded","missing":[..],"rounds":n}
//! argv[1]: JSON object mapping page name ("component:image-block") -> wikitext.
use ftml::data::PageRef;
use ftml::includes::{FetchedPage, IncludeRef, Includer};
use ftml::layout::Layout;
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::io::BufRead;

struct Fs<'a> {
    pages: &'a HashMap<String, String>,
    missing: &'a mut BTreeSet<String>,
}

fn key(r: &PageRef) -> String {
    r.page.to_lowercase()
}

impl<'t, 'a> Includer<'t> for Fs<'a> {
    type Error = String;
    fn include_pages(&mut self, includes: &[IncludeRef<'t>]) -> Result<Vec<FetchedPage<'t>>, String> {
        Ok(includes
            .iter()
            .map(|i| {
                let page_ref = i.page_ref().clone();
                let foreign = matches!(&page_ref.site, Some(s) if s != "scp-wiki");
                let content = if foreign { None } else { self.pages.get(&key(&page_ref)).cloned().map(Cow::Owned) };
                if content.is_none() {
                    self.missing.insert(if foreign { page_ref.to_string() } else { key(&page_ref) });
                }
                FetchedPage { page_ref, content }
            })
            .collect())
    }
    fn no_such_include(&mut self, r: &PageRef) -> Result<Cow<'t, str>, String> {
        // Placeholder only; the page is reported in `missing` so the caller can fetch it.
        Ok(Cow::Owned(format!("[[div class=\"error-block\"]]Page to be included {r} cannot be found![[/div]]")))
    }
}

fn main() {
    let pages: HashMap<String, String> =
        serde_json::from_str(&std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap()).unwrap();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot);
    for line in std::io::stdin().lock().lines() {
        let v: serde_json::Value = serde_json::from_str(&line.unwrap()).unwrap();
        let mut text = v["source"].as_str().unwrap().to_owned();
        let mut missing = BTreeSet::new();
        let mut rounds = 0;
        for _ in 0..6 {
            let (out, included) = ftml::include(&text, &settings, Fs { pages: &pages, missing: &mut missing }, || "invalid".to_owned()).unwrap();
            rounds += 1;
            let done = included.is_empty();
            text = out;
            if done {
                break;
            }
        }
        println!("{}", serde_json::json!({"case_id": v["case_id"], "expanded": text, "missing": missing, "rounds": rounds}));
    }
}
