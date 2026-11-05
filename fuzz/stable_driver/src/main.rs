mod r#gen;

use ftml::Utf16IndexMap;
use ftml::data::{PageInfo, PageRef, ScoreValue};
use ftml::delayed::{
    DelayedInput, GeneratedInput, GeneratedKind, GeneratedValue, InputSegment,
    ResolvedTagRef, SlotBindings, SlotId, TextOrigin, parse_delayed_list,
};
use ftml::includes::{DebugIncluder, NullIncluder};
use ftml::layout::Layout;
use ftml::render::Render;
use ftml::render::debug::DebugRender;
use ftml::render::html::HtmlRender;
use ftml::render::text::TextRender;
use ftml::render::PageExistenceResolver;
use ftml::settings::{WikitextMode, WikitextSettings};
use ftml::tree::SyntaxTree;
use ftml::{WikidotParserFunctionOptions, WikidotZeroOperatorPolicy};
use r#gen::Rng;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

static CURRENT: Mutex<String> = Mutex::new(String::new());
static CFG: Mutex<u32> = Mutex::new(0);
static STAGE: Mutex<&'static str> = Mutex::new("");
static OUT: Mutex<Option<PathBuf>> = Mutex::new(None);
static QUIET: AtomicBool = AtomicBool::new(false);
static LAST_PANIC: Mutex<Option<String>> = Mutex::new(None);

fn stage(name: &'static str) {
    *STAGE.lock().unwrap_or_else(|e| e.into_inner()) = name;
}

const MODES: [WikitextMode; 6] = [
    WikitextMode::Page,
    WikitextMode::PageNav,
    WikitextMode::Draft,
    WikitextMode::ForumPost,
    WikitextMode::DirectMessage,
    WikitextMode::List,
];

#[derive(Clone, Copy, Debug)]
struct Cfg(u32);

impl Cfg {
    fn layout(self) -> Layout {
        if self.0 & 1 == 0 { Layout::Wikidot } else { Layout::Wikijump }
    }
    fn mode(self) -> WikitextMode {
        MODES[((self.0 >> 1) % 6) as usize]
    }
    fn include(self) -> u32 {
        (self.0 >> 4) & 3
    }
    fn parser_fns(self) -> bool {
        (self.0 >> 6) & 1 == 1
    }
    fn zero_replace(self) -> bool {
        (self.0 >> 7) & 1 == 1
    }
    fn page_syntax_off(self) -> bool {
        (self.0 >> 8) & 1 == 1
    }
    fn html_off(self) -> bool {
        (self.0 >> 9) & 1 == 1
    }
    fn true_ids(self) -> bool {
        (self.0 >> 10) & 1 == 1
    }
    fn inline(self) -> bool {
        (self.0 >> 11) & 1 == 1
    }
    fn exists(self) -> bool {
        (self.0 >> 12) & 1 == 1
    }
    fn delayed(self) -> u32 {
        (self.0 >> 13) & 7
    }
    fn isolate(self) -> bool {
        (self.0 >> 16) & 1 == 1
    }
    fn local_paths(self) -> bool {
        (self.0 >> 17) & 1 == 1
    }
    fn plain_preprocess(self) -> bool {
        (self.0 >> 18) & 1 == 1
    }
}

fn page_info() -> PageInfo<'static> {
    PageInfo {
        page: Cow::Borrowed("fuzz-page"),
        category: Some(Cow::Borrowed("fuzz")),
        site: Cow::Borrowed("fuzz"),
        title: Cow::Borrowed("Fuzz"),
        alt_title: None,
        score: ScoreValue::Integer(0),
        tags: vec![Cow::Borrowed("a"), Cow::Borrowed("_b")],
        language: Cow::Borrowed("en"),
    }
}

struct Exists;
impl PageExistenceResolver for Exists {
    fn page_exists(&self, _site: &str, page: &str) -> bool {
        page.len() % 2 == 0
    }
    fn page_title(&self, _site: &str, page: &str) -> Option<String> {
        Some(format!("T {page}"))
    }
}

fn settings_for(cfg: Cfg) -> WikitextSettings {
    let mut settings = WikitextSettings::from_mode(cfg.mode(), cfg.layout());
    if cfg.page_syntax_off() {
        settings.enable_page_syntax = false;
    }
    if cfg.html_off() {
        settings.enable_html_blocks = false;
    }
    settings.use_true_ids = cfg.true_ids();
    settings.isolate_user_ids = cfg.isolate();
    settings.list_pages_inline = cfg.inline();
    settings.allow_local_paths = cfg.local_paths();
    settings
}

fn check_tree(tree: &SyntaxTree, page_info: &PageInfo, settings: &WikitextSettings, cfg: Cfg) {
    stage("tree-refs");
    std::hint::black_box(tree.page_references());
    std::hint::black_box(tree.user_references());
    stage("tree-owned");
    let owned = tree.to_owned();
    stage("tree-json");
    let json = serde_json::to_string(&owned);
    if let Ok(json) = json {
        stage("tree-json-roundtrip");
        if let Ok(back) = serde_json::from_str::<SyntaxTree>(&json) {
            std::hint::black_box(back);
        }
    }
    stage("render-html");
    if cfg.exists() {
        std::hint::black_box(HtmlRender.render_with_page_existence(tree, page_info, settings, &Exists));
    } else {
        std::hint::black_box(HtmlRender.render(tree, page_info, settings));
    }
    stage("render-text");
    std::hint::black_box(TextRender.render(tree, page_info, settings));
    stage("render-debug");
    std::hint::black_box(DebugRender.render(tree, page_info, settings));
}

fn exercise(source: &str, cfg: Cfg) {
    let page_info = page_info();
    let settings = settings_for(cfg);

    stage("utf16-map");
    let map = Utf16IndexMap::new(source);
    for i in 0..=source.len().min(64) {
        std::hint::black_box(map.get_index_opt(i));
    }

    let mut text: String = match cfg.include() {
        0 => source.to_owned(),
        1 => {
            stage("include-debug");
            ftml::include(source, &settings, DebugIncluder::all_present(), || unreachable!()).unwrap().0
        }
        2 => {
            stage("include-debug-missing");
            ftml::include(source, &settings, DebugIncluder::first_missing(), || unreachable!()).unwrap().0
        }
        _ => {
            stage("include-null");
            ftml::include(source, &settings, NullIncluder, || unreachable!()).unwrap().0
        }
    };

    if cfg.parser_fns() {
        stage("parser-functions");
        text = if cfg.zero_replace() {
            ftml::resolve_wikidot_parser_functions_with_options(
                &text,
                WikidotParserFunctionOptions {
                    zero_operator_policy: WikidotZeroOperatorPolicy::ReplaceOperationWithZero,
                },
            )
        } else {
            ftml::resolve_wikidot_parser_functions(&text)
        };
    }

    stage("preprocess");
    if cfg.plain_preprocess() && cfg.layout() == Layout::Wikijump {
        ftml::preprocess(&mut text);
    } else {
        ftml::preprocess_for_layout(&mut text, cfg.layout());
    }

    stage("tokenize");
    let tokenization = ftml::tokenize(&text);
    stage("token-utf16");
    let map = Utf16IndexMap::new(&text);
    for token in tokenization.tokens() {
        std::hint::black_box(token.to_utf16_indices(&map));
    }

    if cfg.mode() == WikitextMode::List && cfg.delayed() != 0 {
        exercise_delayed(&text, cfg, &page_info, &settings);
        return;
    }

    stage("parse");
    let (tree, errors) = ftml::parse(&tokenization, &page_info, &settings).into();
    stage("error-utf16");
    for error in &errors {
        std::hint::black_box(error.to_utf16_indices(&map));
    }
    check_tree(&tree, &page_info, &settings, cfg);
}

fn generated_value(kind: GeneratedKind) -> GeneratedValue<'static> {
    match kind {
        GeneratedKind::PageLink => GeneratedValue::PageLink {
            page: PageRef::page_only("fuzz-target"),
            label: Cow::Borrowed("Fuzz Target"),
        },
        GeneratedKind::TagLinks => GeneratedValue::TagLinks {
            tags: Cow::Owned(vec![ResolvedTagRef { tag: Cow::Borrowed("fuzz") }]),
            separator: Cow::Borrowed(", "),
        },
    }
}

fn exercise_delayed(source: &str, cfg: Cfg, page_info: &PageInfo, settings: &WikitextSettings) {
    stage("delayed-input");
    let mut starts: Vec<usize> = source.char_indices().map(|(i, _)| i).collect();
    starts.push(source.len());
    let h = source.bytes().fold(cfg.0 as usize, |a, b| a.wrapping_mul(31).wrapping_add(b as usize));
    let origin = |v: usize| match v % 3 {
        0 => TextOrigin::Authored,
        1 => TextOrigin::RuntimeScalar,
        _ => TextOrigin::RuntimeLiteral,
    };
    let kind = |v: usize| if v & 1 == 0 { GeneratedKind::PageLink } else { GeneratedKind::TagLinks };
    let (segments, values) = match cfg.delayed() {
        1 | 2 => (vec![InputSegment::text(0..source.len(), origin(h))], vec![]),
        3 | 4 if starts.len() >= 2 => {
            let i = h % (starts.len() - 1);
            let (a, b) = (starts[i], starts[i + 1]);
            let k = kind(h >> 3);
            (
                vec![
                    InputSegment::text(0..a, origin(h >> 5)),
                    InputSegment::generated(GeneratedInput { source_range: a..b, id: SlotId::new(1), kind: k, occurrence: 0 }),
                    InputSegment::text(b..source.len(), origin(h >> 7)),
                ],
                vec![(SlotId::new(1), generated_value(k))],
            )
        }
        _ if starts.len() >= 3 => {
            let n = starts.len() - 1;
            let i = h % (n - 1);
            let j = i + 1 + (h >> 9) % (n - i - 1);
            let repeated = cfg.delayed() >= 6;
            let k1 = kind(h >> 3);
            let k2 = if repeated { k1 } else { kind(h >> 4) };
            let mut values = vec![(SlotId::new(1), generated_value(k1))];
            if !repeated {
                values.push((SlotId::new(2), generated_value(k2)));
            }
            (
                vec![
                    InputSegment::text(0..starts[i], origin(h >> 5)),
                    InputSegment::generated(GeneratedInput { source_range: starts[i]..starts[i + 1], id: SlotId::new(1), kind: k1, occurrence: 0 }),
                    InputSegment::text(starts[i + 1]..starts[j], origin(h >> 6)),
                    InputSegment::generated(GeneratedInput {
                        source_range: starts[j]..starts[j + 1],
                        id: if repeated { SlotId::new(1) } else { SlotId::new(2) },
                        kind: k2,
                        occurrence: if repeated { 1 } else { 0 },
                    }),
                    InputSegment::text(starts[j + 1]..source.len(), origin(h >> 7)),
                ],
                values,
            )
        }
        _ => (vec![InputSegment::text(0..source.len(), origin(h))], vec![]),
    };
    let Ok(input) = DelayedInput::new(source, segments) else { return };
    let Ok(bindings) = SlotBindings::new(values) else { return };
    stage("delayed-parse");
    let Ok(delayed) = parse_delayed_list(&input, page_info, settings) else { return };
    stage("delayed-bind");
    let Ok(bound) = delayed.bind(&bindings) else { return };
    stage("delayed-render");
    if cfg.exists() {
        std::hint::black_box(bound.render_html_with_page_existence(page_info, settings, &Exists));
    } else {
        std::hint::black_box(bound.render_html(page_info, settings));
    }
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

fn install_hook() {
    std::panic::set_hook(Box::new(|info| {
        let loc = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_default();
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic>".to_owned()
        };
        let key = loc.rsplit_once(':').map(|(a, _)| a.to_owned()).unwrap_or(loc.clone());
        *LAST_PANIC.lock().unwrap_or_else(|e| e.into_inner()) = Some(key.clone());
        if QUIET.load(Ordering::Relaxed) {
            return;
        }
        let input = CURRENT.lock().map(|g| g.clone()).unwrap_or_else(|e| e.into_inner().clone());
        let cfg = *CFG.lock().unwrap_or_else(|e| e.into_inner());
        let st = *STAGE.lock().unwrap_or_else(|e| e.into_inner());
        let out = OUT.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let Some(out) = out else {
            eprintln!("PANIC at {loc} in stage {st} (cfg {cfg}): {msg}");
            eprintln!("{}", std::backtrace::Backtrace::force_capture());
            return;
        };
        let base = out.join(format!("crash-{}", sanitize(&key)));
        let path = base.with_extension("txt");
        let better = match std::fs::metadata(&path) {
            Ok(m) => (m.len() as usize) > input.len(),
            Err(_) => true,
        };
        if better {
            let _ = std::fs::write(&path, &input);
            let bt = std::backtrace::Backtrace::force_capture();
            let _ = std::fs::write(
                base.with_extension("msg"),
                format!("location: {loc}\nstage: {st}\ncfg: {cfg} {:?}\nmessage: {msg}\n\n{bt}\n", Cfg(cfg)),
            );
            let _ = std::fs::write(base.with_extension("cfg"), cfg.to_string());
        }
    }));
}

fn load_corpus() -> Vec<String> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<String>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.file_name().is_some_and(|n| n == "input.ftml" || n.to_string_lossy().ends_with(".ftml")) {
                if let Ok(s) = std::fs::read_to_string(&p) {
                    if s.len() < 200_000 {
                        out.push(s);
                    }
                }
            }
        }
    }
    for d in ["test", "tests/fixtures", "verification", "misc", "benches", "conf"] {
        walk(&Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).join(d), &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn run_one(input: &str, cfg: Cfg) -> Result<(), ()> {
    *CURRENT.lock().unwrap_or_else(|e| e.into_inner()) = input.to_owned();
    *CFG.lock().unwrap_or_else(|e| e.into_inner()) = cfg.0;
    std::panic::catch_unwind(|| exercise(input, cfg)).map_err(|_| ())
}

fn worker(out: PathBuf, seed: u64, secs: u64) {
    *OUT.lock().unwrap() = Some(out.clone());
    install_hook();
    let corpus = load_corpus();
    eprintln!("worker {seed}: corpus {} entries", corpus.len());
    let mut rng = Rng::new(seed);
    let cur = out.join(format!("cur-{seed}.txt"));
    let start = Instant::now();
    let mut execs = 0u64;
    let mut panics = 0u64;
    let mut last_report = Instant::now();
    while start.elapsed() < Duration::from_secs(secs) {
        let input = match rng.below(10) {
            0..=5 => {
                let mut s = rng.pick(&corpus).clone();
                r#gen::mutate(&mut rng, &mut s, &corpus);
                s
            }
            6 => rng.pick(&corpus).clone(),
            7 | 8 => r#gen::synth(&mut rng),
            _ => {
                let mut s = r#gen::synth(&mut rng);
                r#gen::mutate(&mut rng, &mut s, &corpus);
                s
            }
        };
        let cfg = Cfg(rng.next() as u32);
        let _ = std::fs::write(&cur, format!("{}\n{}", cfg.0, input));
        let t = Instant::now();
        if run_one(&input, cfg).is_err() {
            panics += 1;
        }
        let el = t.elapsed();
        if el > Duration::from_secs(3) {
            let _ = std::fs::write(out.join(format!("slow-{seed}-{execs}.txt")), format!("{}\n{}", cfg.0, input));
            eprintln!("worker {seed}: slow input {:?} ({} bytes)", el, input.len());
        }
        execs += 1;
        if last_report.elapsed() > Duration::from_secs(60) {
            eprintln!("worker {seed}: {execs} execs, {panics} panics, {:.0}/s", execs as f64 / start.elapsed().as_secs_f64());
            last_report = Instant::now();
        }
    }
    eprintln!("worker {seed}: done {execs} execs, {panics} panics");
    let _ = std::fs::remove_file(&cur);
}

fn supervise(out: PathBuf, workers: u64, secs: u64, base_seed: u64) {
    std::fs::create_dir_all(&out).unwrap();
    let exe = std::env::current_exe().unwrap();
    let deadline = Instant::now() + Duration::from_secs(secs);
    let mut next_seed = base_seed;
    let spawn = |seed: u64, remaining: u64| {
        Command::new("sh")
            .arg("-c")
            .arg("ulimit -v 6000000; exec \"$0\" work \"$1\" \"$2\" \"$3\"")
            .arg(&exe)
            .arg(&out)
            .arg(seed.to_string())
            .arg(remaining.to_string())
            .stdin(Stdio::null())
            .spawn()
            .unwrap()
    };
    let mut children: Vec<(u64, std::process::Child)> = Vec::new();
    for _ in 0..workers {
        children.push((next_seed, spawn(next_seed, secs)));
        next_seed += 1;
    }
    let mut aborts = 0;
    loop {
        std::thread::sleep(Duration::from_millis(500));
        let now = Instant::now();
        let mut i = 0;
        while i < children.len() {
            let (seed, child) = &mut children[i];
            let cur = out.join(format!("cur-{seed}.txt"));
            let stale = std::fs::metadata(&cur)
                .and_then(|m| m.modified())
                .map(|t| SystemTime::now().duration_since(t).unwrap_or_default() > Duration::from_secs(60))
                .unwrap_or(false);
            if stale {
                eprintln!("supervisor: worker {seed} hung; killing");
                let _ = child.kill();
                let _ = std::fs::copy(&cur, out.join(format!("hang-{seed}.txt")));
                let _ = std::fs::remove_file(&cur);
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    let old = *seed;
                    if !status.success() && cur.exists() {
                        aborts += 1;
                        eprintln!("supervisor: worker {old} died ({status}); saving input");
                        let _ = std::fs::copy(&cur, out.join(format!("abort-{old}-{aborts}.txt")));
                        let _ = std::fs::remove_file(&cur);
                    }
                    if now < deadline {
                        let remaining = (deadline - now).as_secs().max(1);
                        children[i] = (next_seed, spawn(next_seed, remaining));
                        next_seed += 1;
                        i += 1;
                    } else {
                        children.remove(i);
                    }
                }
                _ => i += 1,
            }
        }
        if children.is_empty() {
            break;
        }
    }
}

fn read_case(path: &str) -> (Option<u32>, String) {
    let data = std::fs::read_to_string(path).unwrap();
    // cur/abort/hang files carry "cfg\ninput"; crash files are raw input.
    if path.contains("cur-") || path.contains("abort-") || path.contains("hang-") || path.contains("slow-") {
        if let Some((c, rest)) = data.split_once('\n') {
            if let Ok(c) = c.parse() {
                return (Some(c), rest.to_owned());
            }
        }
    }
    (None, data)
}

fn all_cfgs() -> Vec<Cfg> {
    let mut v = Vec::new();
    for layout in 0..2u32 {
        for mode in 0..6u32 {
            for include in [0u32, 1] {
                for pf in [0u32, 1] {
                    for delayed in [0u32, 3, 5] {
                        if delayed != 0 && mode != 5 {
                            continue;
                        }
                        v.push(Cfg(layout | (mode << 1) | (include << 4) | (pf << 6) | (delayed << 13)));
                    }
                }
            }
        }
    }
    v
}

/// Returns the panic key (file:line) for the input under cfg, if it panics.
fn panic_key(input: &str, cfgs: &[Cfg]) -> Option<String> {
    for &cfg in cfgs {
        *LAST_PANIC.lock().unwrap() = None;
        if run_one(input, cfg).is_err() {
            return LAST_PANIC.lock().unwrap().clone();
        }
    }
    None
}

fn minimize(path: &str, cfg_arg: Option<u32>) {
    QUIET.store(true, Ordering::Relaxed);
    install_hook();
    let (file_cfg, mut input) = read_case(path);
    let cfgs: Vec<Cfg> = match cfg_arg.or(file_cfg) {
        Some(c) => vec![Cfg(c)],
        None => all_cfgs(),
    };
    let Some(key) = panic_key(&input, &cfgs) else {
        println!("no panic");
        return;
    };
    let cfg = *cfgs
        .iter()
        .find(|&&c| {
            *LAST_PANIC.lock().unwrap() = None;
            run_one(&input, c).is_err() && LAST_PANIC.lock().unwrap().as_deref() == Some(&key)
        })
        .unwrap();
    let cfgs = vec![cfg];
    eprintln!("target panic: {key} (cfg {})", cfg.0);
    let still = |s: &str| panic_key(s, &cfgs).as_deref() == Some(key.as_str());
    let mut chunk = input.len() / 2;
    while chunk >= 1 {
        let mut changed = false;
        let mut i = 0;
        while i < input.len() {
            let mut a = i;
            while !input.is_char_boundary(a) {
                a += 1;
            }
            let mut b = (a + chunk).min(input.len());
            while !input.is_char_boundary(b) {
                b += 1;
            }
            if a >= input.len() {
                break;
            }
            let mut cand = input.clone();
            cand.replace_range(a..b, "");
            if still(&cand) {
                input = cand;
                changed = true;
            } else {
                i = b;
            }
        }
        if !changed {
            chunk /= 2;
        }
    }
    // Try simplifying characters to 'a'.
    let chars: Vec<(usize, char)> = input.char_indices().collect();
    for (i, c) in chars.into_iter().rev() {
        if c == 'a' || !input.is_char_boundary(i) {
            continue;
        }
        let mut cand = input.clone();
        cand.replace_range(i..i + c.len_utf8(), "a");
        if still(&cand) {
            input = cand;
        }
    }
    println!("cfg {} {:?}\nkey {key}\n{:?}", cfg.0, cfg, input);
}

fn run_file(path: &str, cfg_arg: Option<u32>) {
    install_hook();
    let (file_cfg, input) = read_case(path);
    let cfgs: Vec<Cfg> = match cfg_arg.or(file_cfg) {
        Some(c) => vec![Cfg(c)],
        None => all_cfgs(),
    };
    for cfg in cfgs {
        let t = Instant::now();
        let r = run_one(&input, cfg);
        let el = t.elapsed();
        if r.is_err() || el > Duration::from_secs(1) {
            println!("cfg {} {:?}: {} in {:?}", cfg.0, cfg, if r.is_err() { "PANIC" } else { "ok" }, el);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("supervise") => supervise(
            PathBuf::from(&args[2]),
            args[3].parse().unwrap(),
            args[4].parse().unwrap(),
            args.get(5).map(|s| s.parse().unwrap()).unwrap_or(1),
        ),
        Some("work") => worker(PathBuf::from(&args[2]), args[3].parse().unwrap(), args[4].parse().unwrap()),
        Some("run") => run_file(&args[2], args.get(3).map(|s| s.parse().unwrap())),
        Some("min") => minimize(&args[2], args.get(3).map(|s| s.parse().unwrap())),
        _ => eprintln!("usage: supervise <out> <workers> <secs> [seed] | work <out> <seed> <secs> | run <file> [cfg] | min <file> [cfg]"),
    }
}
