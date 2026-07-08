use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::Render;
use ftml::render::html::HtmlRender;
use ftml::render::text::TextRender;
use ftml::settings::{WikitextMode, WikitextSettings};
use log::{Level, LevelFilter, Metadata, Record};
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Once;

struct TraceLogger;

impl log::Log for TraceLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record<'_>) {
        if self.enabled(record.metadata()) {
            let _ = record.args().to_string();
        }
    }

    fn flush(&self) {}
}

static LOGGER: TraceLogger = TraceLogger;
static INIT_LOGGER: Once = Once::new();

fn enable_trace_logging() {
    INIT_LOGGER.call_once(|| {
        let _ = log::set_logger(&LOGGER);
        log::set_max_level(LevelFilter::Trace);
    });
}

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("coverage-trace-page"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("coverage"),
        title: Cow::Borrowed("Coverage Trace Page"),
        alt_title: Some(Cow::Borrowed("Trace Alt")),
        score: ScoreValue::Integer(42),
        tags: vec![
            Cow::Borrowed("fruit"),
            Cow::Borrowed("component"),
            Cow::Borrowed("template"),
            Cow::Borrowed("test"),
        ],
        language: Cow::Borrowed("en"),
    }
}

fn collect_fixture_inputs(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(path).expect("read fixture directory") {
        let entry = entry.expect("read fixture directory entry");
        let path = entry.path();
        if path.is_dir() {
            collect_fixture_inputs(&path, files);
        } else if path.file_name().is_some_and(|name| name == "input.ftml") {
            files.push(path);
        }
    }
}

#[test]
fn coverage_trace_logger_exercises_all_tree_fixtures_through_public_api() {
    enable_trace_logging();

    let mut files = Vec::new();
    collect_fixture_inputs(Path::new("test"), &mut files);
    files.sort();

    assert!(
        files.len() > 100,
        "expected the checked-in tree fixture corpus"
    );

    let page_info = page_info();
    let settings = [
        WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump),
        WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikidot),
    ];

    for path in files {
        let mut input = std::fs::read_to_string(&path).expect("read fixture input");
        ftml::preprocess(&mut input);
        let tokens = ftml::tokenize(&input);

        for settings in &settings {
            let result = ftml::parse(&tokens, &page_info, settings);
            let (tree, _errors) = result.into();

            let text = TextRender.render(&tree, &page_info, settings);
            let html = HtmlRender.render(&tree, &page_info, settings);

            assert!(tree.wikitext_len <= input.len());
            assert!(text.len() <= input.len().saturating_mul(20).saturating_add(4096));
            assert!(
                html.body.len() <= input.len().saturating_mul(80).saturating_add(16384)
            );
        }
    }
}
