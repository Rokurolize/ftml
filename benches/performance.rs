use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
};
use ftml::data::{PageInfo, ScoreValue};
use ftml::layout::Layout;
use ftml::render::Render;
use ftml::render::html::HtmlRender;
use ftml::settings::{WikitextMode, WikitextSettings};
use std::borrow::Cow;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;

const KITCHEN_SINK_TARGET_BYTES: usize = 50 * 1024;
const LARGE_CORPUS_TARGET_BYTES: usize = 500 * 1024;
const PLAINTEXT_TARGET_BYTES: usize = 64 * 1024;
const MARKUP_DENSE_TARGET_BYTES: usize = 64 * 1024;

const KITCHEN_SINK_FIXTURES: &[&str] = &[
    "test/align/basic/input.ftml",
    "test/anchor/basic/input.ftml",
    "test/audio/basic/input.ftml",
    "test/blockquote/native/input.ftml",
    "test/bold/native/input.ftml",
    "test/checkbox/basic/input.ftml",
    "test/code/basic/input.ftml",
    "test/collapsible/basic/input.ftml",
    "test/color/basic/input.ftml",
    "test/date/matrix/input.ftml",
    "test/definition-list/basic/input.ftml",
    "test/div/basic/input.ftml",
    "test/embed/basic/input.ftml",
    "test/footnote/basic/input.ftml",
    "test/heading/basic/input.ftml",
    "test/html/basic/input.ftml",
    "test/iframe/basic/input.ftml",
    "test/image/basic/input.ftml",
    "test/italics/block/input.ftml",
    "test/line-breaks/basic/input.ftml",
    "test/link/single/input.ftml",
    "test/link/triple/input.ftml",
    "test/list/block/input.ftml",
    "test/list/native/input.ftml",
    "test/math/block/input.ftml",
    "test/math/inline/input.ftml",
    "test/misc/bibliography/input.ftml",
    "test/misc/char/input.ftml",
    "test/misc/clear-float/input.ftml",
    "test/misc/email/input.ftml",
    "test/misc/hr/input.ftml",
    "test/module/css/input.ftml",
    "test/module/rate/input.ftml",
    "test/monospace/basic/input.ftml",
    "test/paragraph/basic/input.ftml",
    "test/raw/basic/input.ftml",
    "test/raw/block/input.ftml",
    "test/ruby/basic/input.ftml",
    "test/span/basic/input.ftml",
    "test/table/advanced/input.ftml",
    "test/table/simple/input.ftml",
    "test/tabview/basic/input.ftml",
    "test/toc/basic/input.ftml",
    "test/user/basic/input.ftml",
    "test/video/basic/input.ftml",
];

#[derive(Debug)]
struct Corpus {
    name: &'static str,
    source: String,
}

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("performance-baseline"),
        category: Some(Cow::Borrowed("bench")),
        site: Cow::Borrowed("sandbox"),
        title: Cow::Borrowed("Performance Baseline"),
        alt_title: Some(Cow::Borrowed("FTML Bench")),
        score: ScoreValue::Integer(42),
        tags: vec![
            Cow::Borrowed("bench"),
            Cow::Borrowed("component"),
            Cow::Borrowed("performance"),
        ],
        language: Cow::Borrowed("en"),
    }
}

fn settings() -> WikitextSettings {
    WikitextSettings::from_mode(WikitextMode::Page, Layout::Wikijump)
}

fn fixture_inputs() -> Vec<(PathBuf, String)> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    KITCHEN_SINK_FIXTURES
        .iter()
        .map(|path| {
            let relative = PathBuf::from(path);
            let input = std::fs::read_to_string(manifest_dir.join(path))
                .expect("read fixture input");
            (relative, input)
        })
        .collect()
}

fn kitchen_sink(inputs: &[(PathBuf, String)]) -> String {
    let mut out = String::new();
    for (path, input) in inputs {
        out.push_str("\n\n[[!-- fixture: ");
        out.push_str(&path.display().to_string());
        out.push_str(" --]]\n");
        out.push_str(input);
        out.push('\n');
    }
    out
}

fn repeat_to_at_least(seed: &str, target_bytes: usize) -> String {
    let mut out = String::with_capacity(target_bytes + seed.len());
    while out.len() < target_bytes {
        out.push_str(seed);
        out.push_str("\n\n");
    }
    out
}

fn plaintext_corpus() -> String {
    let paragraph = "This is a plain text paragraph with ordinary words, punctuation, numbers 1234567890, and no intentional FTML markup. It exercises preprocessing, token scanning, paragraph handling, escaping, and HTML string output without table, module, CSS, date, or math syntax.\n\n";
    repeat_to_at_least(paragraph, PLAINTEXT_TARGET_BYTES)
}

fn markup_dense_corpus() -> String {
    let block = r#"
+ Heading One

||~ Name ||~ Value ||~ Link ||
|| Alpha || **bold** and //italic// and __underlined__ || [[[target-page|Target Page]]] ||
|| Beta || [[span class="metric" style="color: red;"]]inline span[[/span]] || [https://example.com External] ||

[[div class="panel" style="border: 1px solid #ccc; padding: 0.5rem;"]]
[[module CSS]]
.panel { color: #123456; background: white; }
.panel .metric { font-weight: bold; }
[[/module]]

[[toc]]

* bullet item
 * nested bullet item
 1. numbered text with ##red|color##
 2. [[date 2001-09-11 format="%Y-%m-%d %H:%M:%S"]]

[[collapsible show="+ show" hide="- hide"]]
> quoted text
> more quoted text
[[/collapsible]]
[[/div]]
"#;
    repeat_to_at_least(block, MARKUP_DENSE_TARGET_BYTES)
}

fn corpora() -> Vec<Corpus> {
    let fixtures = fixture_inputs();
    let kitchen = kitchen_sink(&fixtures);
    vec![
        Corpus {
            name: "kitchen_sink",
            source: repeat_to_at_least(&kitchen, KITCHEN_SINK_TARGET_BYTES),
        },
        Corpus {
            name: "large_kitchen_sink",
            source: repeat_to_at_least(&kitchen, LARGE_CORPUS_TARGET_BYTES),
        },
        Corpus {
            name: "plain_text",
            source: plaintext_corpus(),
        },
        Corpus {
            name: "markup_dense",
            source: markup_dense_corpus(),
        },
    ]
}

fn preprocessed(source: &str) -> String {
    let mut text = source.to_owned();
    ftml::preprocess(&mut text);
    text
}

fn bench_stage(c: &mut Criterion) {
    let page_info = page_info();
    let settings = settings();
    let corpora = corpora();
    let mut group = c.benchmark_group("ftml_stage");

    for corpus in &corpora {
        let preprocessed = preprocessed(&corpus.source);
        let tokenization = ftml::tokenize(&preprocessed);
        let (tree, _errors) = ftml::parse(&tokenization, &page_info, &settings).into();

        group.throughput(Throughput::Bytes(corpus.source.len() as u64));
        group.bench_function(BenchmarkId::new("preprocess", corpus.name), |b| {
            b.iter_batched(
                || corpus.source.clone(),
                |mut text| {
                    ftml::preprocess(black_box(&mut text));
                    black_box(text.len())
                },
                BatchSize::SmallInput,
            )
        });

        group.throughput(Throughput::Bytes(preprocessed.len() as u64));
        group.bench_function(BenchmarkId::new("tokenize", corpus.name), |b| {
            b.iter(|| {
                let tokenization = ftml::tokenize(black_box(&preprocessed));
                black_box(tokenization.tokens().len())
            })
        });

        group.bench_function(BenchmarkId::new("parse", corpus.name), |b| {
            b.iter(|| {
                let (tree, errors) =
                    ftml::parse(black_box(&tokenization), &page_info, &settings).into();
                black_box((tree.elements.len(), errors.len()))
            })
        });

        group.bench_function(BenchmarkId::new("render_html", corpus.name), |b| {
            b.iter(|| {
                let output = HtmlRender.render(black_box(&tree), &page_info, &settings);
                black_box((output.body.len(), output.styles.len(), output.meta.len()))
            })
        });
    }

    group.finish();
}

fn bench_full_pipeline(c: &mut Criterion) {
    let page_info = page_info();
    let settings = settings();
    let corpora = corpora();
    let mut group = c.benchmark_group("ftml_full_pipeline");

    for corpus in &corpora {
        group.throughput(Throughput::Bytes(corpus.source.len() as u64));
        group.bench_function(BenchmarkId::from_parameter(corpus.name), |b| {
            b.iter_batched(
                || corpus.source.clone(),
                |mut text| {
                    ftml::preprocess(black_box(&mut text));
                    let tokens = ftml::tokenize(black_box(&text));
                    let (tree, errors) =
                        ftml::parse(black_box(&tokens), &page_info, &settings).into();
                    let output =
                        HtmlRender.render(black_box(&tree), &page_info, &settings);
                    black_box((output.body.len(), output.styles.len(), errors.len()))
                },
                BatchSize::SmallInput,
            )
        });
    }

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1))
        .sample_size(10);
    targets = bench_stage, bench_full_pipeline
}
criterion_main!(benches);
