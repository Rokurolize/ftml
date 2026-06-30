use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::Render;
use ftml::render::html::HtmlRender;
use ftml::render::text::TextRender;
use ftml::settings::{WikitextMode, WikitextSettings};
use log::{Level, LevelFilter, Metadata, Record};
use std::borrow::Cow;
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
        page: Cow::Borrowed("coverage-parser-edges"),
        category: Some(Cow::Borrowed("test")),
        site: Cow::Borrowed("coverage"),
        title: Cow::Borrowed("Coverage Parser Edges"),
        alt_title: Some(Cow::Borrowed("Parser Edges")),
        score: ScoreValue::Integer(3),
        tags: vec![Cow::Borrowed("test"), Cow::Borrowed("component")],
        language: Cow::Borrowed("en"),
    }
}

#[test]
fn coverage_parser_edges_exercise_rule_boundaries_with_logging() {
    enable_trace_logging();

    let page_info = page_info();
    let settings = WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump);
    let snippets = [
        "@@@@@@",
        "@@@@@",
        "@@@@",
        "@@ raw @@",
        "@< raw >@",
        "@@\n@@",
        "+ Heading\n++ Subheading\nplain",
        "= centered",
        "> quoted\n>> nested\nplain",
        "* \n* item\nplain",
        "|| A || B ||\n|| C || D ||\nplain",
        "|| A ||\nfollowing",
        "[# anchor-name]",
        "[https://example.com Label]",
        "[*https://example.com New tab]",
        "((bibcite ref-1))",
        "{$variable}",
        "%%math%%",
        "[[iftags +test]]tagged[[/iftags]]",
        "[[iftags -missing]]hidden[[/iftags]]",
        "[[ifcategory test]]category[[/ifcategory]]",
        "[[ifcategory other]]hidden[[/ifcategory]]",
        "[[size 120%]]large[[/size]]",
        "[[span class=\"marker\"]]span[[/span]]",
        "[[div class=\"box\"]]div[[/div]]",
        "[[image local.png]]",
        "[[iframe https://example.com/embed]]",
        "[[audio local.mp3]]",
        "[[video local.mp4]]",
        "[[math]]x^2[[/math]]",
        "[[bibcite ref-2]]",
        "[[user account-name]]",
        "[[module PageTree root=\"start\" depth=\"2\" showRoot=\"true\"]]",
    ];

    for snippet in snippets {
        let mut input = snippet.to_owned();
        ftml::preprocess(&mut input);
        let tokens = ftml::tokenize(&input);
        let result = ftml::parse(&tokens, &page_info, &settings);
        let (tree, _errors) = result.into();

        let text = TextRender.render(&tree, &page_info, &settings);
        let html = HtmlRender.render(&tree, &page_info, &settings);

        assert!(tree.wikitext_len <= input.len());
        assert!(text.len() <= input.len().saturating_mul(20).saturating_add(2048));
        assert!(html.body.len() <= input.len().saturating_mul(80).saturating_add(8192));
    }
}
