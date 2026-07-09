# FTML performance baseline and hotspot profile (2026-07-09)

Date: 2026-07-09

Branch: `perf/bench-harness`

Commands used:

```bash
cargo bench --bench performance
CARGO_PROFILE_BENCH_DEBUG=1 CARGO_PROFILE_BENCH_LTO=false cargo bench --bench performance --no-run
valgrind --tool=callgrind --callgrind-out-file=/tmp/ftml-callgrind-full-large-debug.out target/release/deps/performance-86b810df2c1e0eef --profile-time 1 --noplot 'ftml_full_pipeline/large_kitchen_sink'
valgrind --tool=callgrind --callgrind-out-file=/tmp/ftml-callgrind-render-large-debug.out target/release/deps/performance-86b810df2c1e0eef --profile-time 1 --noplot 'ftml_stage/render_html/large_kitchen_sink'
```

Files changed or added:

* `Cargo.toml`: added Criterion dev-dependency and `[[bench]]` target with `harness = false`.
* `benches/performance.rs`: added deterministic corpus builder and Criterion benchmarks for preprocess, tokenize, parse, render HTML, and full pipeline.
* `PERF_BASELINE.md`: this working note.

Corpus notes:

* `kitchen_sink`: 57,138 input bytes, built by concatenating 45 checked-in valid representative `test/**/input.ftml` fixtures and repeating to about 50 KiB.
* `large_kitchen_sink`: 514,242 input bytes, built by repeating the same kitchen sink seed to about 500 KiB.
* `plain_text`: 65,682 input bytes of deterministic plain paragraphs with no intentional FTML markup.
* `markup_dense`: 65,954 input bytes of deterministic tables, links, formatting, CSS module, TOC, date, list, div, and collapsible markup.

Baseline table:

| Corpus | Stage | Bytes | Median | Throughput |
|---|---:|---:|---:|---:|
| kitchen_sink | preprocess | 57,138 | 163.870 us | 332.53 MiB/s |
| kitchen_sink | tokenize | 56,842 | 436.842 us | 124.09 MiB/s |
| kitchen_sink | parse | 56,842 | 1.854 ms | 29.23 MiB/s |
| kitchen_sink | render_html | 56,842 | 1.041 ms | 52.05 MiB/s |
| kitchen_sink | full_pipeline | 57,138 | 3.423 ms | 15.92 MiB/s |
| large_kitchen_sink | preprocess | 514,242 | 7.950 ms | 61.69 MiB/s |
| large_kitchen_sink | tokenize | 511,594 | 4.194 ms | 116.34 MiB/s |
| large_kitchen_sink | parse | 511,594 | 18.153 ms | 26.88 MiB/s |
| large_kitchen_sink | render_html | 511,594 | 22.902 ms | 21.30 MiB/s |
| large_kitchen_sink | full_pipeline | 514,242 | 50.158 ms | 9.78 MiB/s |
| plain_text | preprocess | 65,682 | 211.405 us | 296.30 MiB/s |
| plain_text | tokenize | 65,188 | 113.833 us | 546.14 MiB/s |
| plain_text | parse | 65,188 | 1.034 ms | 60.14 MiB/s |
| plain_text | render_html | 65,188 | 140.990 us | 440.94 MiB/s |
| plain_text | full_pipeline | 65,682 | 1.829 ms | 34.24 MiB/s |
| markup_dense | preprocess | 65,954 | 156.004 us | 403.19 MiB/s |
| markup_dense | tokenize | 65,756 | 162.883 us | 385.00 MiB/s |
| markup_dense | parse | 65,756 | 1.407 ms | 44.58 MiB/s |
| markup_dense | render_html | 65,756 | 2.334 ms | 26.87 MiB/s |
| markup_dense | full_pipeline | 65,954 | 4.350 ms | 14.46 MiB/s |

Linearity check:

* `large_kitchen_sink` has 9.00x the bytes of `kitchen_sink`, but full-pipeline median time is 14.65x higher, so this baseline is not linear.
* Stage ratios for 500 KiB vs 50 KiB: preprocess 48.51x time / 5.39x per-byte slowdown, tokenize 9.60x time / 1.07x per-byte slowdown, parse 9.79x time / 1.09x per-byte slowdown, render HTML 21.99x time / 2.44x per-byte slowdown.
* Tokenize and parse are close to linear. Preprocess is strongly superlinear on repeated kitchen-sink content, and render HTML is moderately superlinear.

Hotspot ranking:

* 1. HTML render on large kitchen sink: 22.902 ms median, 43.0% of summed stage medians for the large corpus. Render-only callgrind reported 445,461,760 instruction refs, 51.05% of the render-only profile, in libc `__memcpy_avx_unaligned_erms`, with render call paths including `HtmlRender::render`, `render_elements`, `render_style`, `render_date`, and `render_latex`.
* 2. Parse on large kitchen sink: 18.153 ms median, 34.1% of summed stage medians. Full-pipeline callgrind attributed 141,030,273 instruction refs, 9.82% of that profiled run, to `ftml::parsing::parse` inside the full-pipeline closure.
* 3. Preprocess on large kitchen sink: 7.950 ms median, 14.9% of summed stage medians, but the main nonlinear stage. Full-pipeline callgrind attributed 384,756,288 instruction refs, 26.80% of that profiled run, to `ftml::preproc::preprocess`; source shows repeated `String::replace_range` in regex replacement loops in `src/preproc/mod.rs` and `src/preproc/whitespace.rs`.
* 4. Tokenize on large kitchen sink: 4.194 ms median, 7.9% of summed stage medians. Tokenizer is close to byte-linear; source preallocates `Vec::with_capacity(text.len() / 2 + 2)` in `src/parsing/token/scanner.rs:4`.

Suspect verdicts:

* Regex compilation per call: refuted for local hot-path matches found. `rg 'Regex::new|RegexBuilder::new' src` shows regexes wrapped in `LazyLock`, for example `src/preproc/whitespace.rs:34-61`, `src/preproc/typography.rs:41-66`, `src/parsing/rule/impls/block/parser.rs:34-35`, `src/includes/mod.rs:45-54`, `src/url.rs:115-116`, and `src/tree/attribute/safe.rs:188-189`.
* ICU formatter or locale construction per render call: confirmed for date rendering. `src/tree/date.rs:255-404` constructs ICU `DateTimeFormatter`, `FixedCalendarDateTimeNames`, `DateTimeFormatterPreferences`, and relative-time formatters inside directive handlers; render-only callgrind includes `icu_datetime::neo::DateTimeFormatter::try_new`, `icu_locale_core::locale::Locale::try_from_utf8`, `icu_decimal::decimal_formatter::DecimalFormatter::try_new`, and `icu_experimental::relativetime::RelativeTimeFormatter::try_new_long_day`.
* lightningcss CSS parsing invoked per element or attribute during HTML render: refuted for ordinary attributes, confirmed for each `Element::Style`. The only `lightningcss` use is `src/render/html/element/style.rs`; `render_style` calls `StyleSheet::parse` and `to_css` for style elements, while normal `style` attributes are emitted through attribute rendering without lightningcss. Render-only callgrind includes `ftml::render::html::element::style::render_style`, `lightningcss::stylesheet::StyleSheet::parse`, and `StyleSheet<T>::to_css`.
* latex2mathml cost in non-math documents: refuted. `latex_to_mathml` is only called from `render_latex` in `src/render/html/element/math.rs:111-122`, which is reached by `Element::Math` and `Element::MathInline` only. Plain-text render median is 140.990 us at 440.94 MiB/s, and source has no latex call outside math rendering. Render-only callgrind for the kitchen corpus confirms latex call paths when math fixtures are present.
* Per-token or per-element heap allocation and cloning patterns dominate: partially confirmed. Callgrind flat profiles are dominated by libc memcpy: 848,403,723 instruction refs, 59.10% of full large profile, and 445,461,760 instruction refs, 51.05% of render-only large profile. Evidence points to copy-heavy string mutation/output, especially preprocess `replace_range`, corpus cloning in `iter_batched`, and HTML output construction. Tokenizer itself preallocates tokens and is not the dominant stage.
* O(n^2) behavior with document length: confirmed as a risk in this corpus pair. 9.00x bytes caused 14.65x full-pipeline time; preprocess was 48.51x and render HTML was 21.99x, while tokenize and parse were near 10x. The strongest concrete source suspect is repeated in-place `String::replace_range` over growing documents in preprocessor replacement loops.

Ranked optimization recommendations:

* 1. Replace preprocessor repeated in-place `replace_range` loops with single-pass rewrite buffers per transformation or combined passes. Potential: large-corpus preprocess dropped from 7.950 ms toward the 50 KiB per-byte rate would save about 6.5 ms on the 500 KiB corpus, roughly 13% of full pipeline.
* 2. Reduce HTML render copying and repeated output reallocations/copies. Potential: render is the largest large-corpus stage at 22.902 ms; a 20-30% render improvement would save about 4.6-6.9 ms, roughly 9-14% of full pipeline.
* 3. Cache ICU formatters or precomputed locale/date formatting helpers by language and directive shape. Potential: high on date-heavy documents, lower on plain text; this is a targeted render win and should be guarded by tests because date formatting is behavior-sensitive.
* 4. Avoid reparsing identical CSS module bodies when repeated content produces identical `Element::Style` inputs. Potential: useful for repeated templates/modules, but not a general win for documents with unique CSS.
* 5. Leave tokenizer alone initially. It is close to linear and is only 7.9% of the large-corpus summed stage medians.

Raw `cargo bench --bench performance` output:

```text
   Compiling ftml v1.42.0+roku.20260630.1 (/home/roku/src/Rokurolize/ftml-wt/perf-bench)
    Finished `bench` profile [optimized] target(s) in 1m 37s
     Running benches/performance.rs (target/release/deps/performance-ba685c2a7bf3d658)
Gnuplot not found, using plotters backend
Benchmarking ftml_stage/preprocess/kitchen_sink
Benchmarking ftml_stage/preprocess/kitchen_sink: Warming up for 500.00 ms
Benchmarking ftml_stage/preprocess/kitchen_sink: Collecting 10 samples in estimated 1.0009 s (5720 iterations)
Benchmarking ftml_stage/preprocess/kitchen_sink: Analyzing
ftml_stage/preprocess/kitchen_sink
                        time:   [161.24 µs 163.91 µs 167.86 µs]
                        thrpt:  [324.63 MiB/s 332.45 MiB/s 337.96 MiB/s]
Found 2 outliers among 10 measurements (20.00%)
  1 (10.00%) low mild
  1 (10.00%) high mild
Benchmarking ftml_stage/tokenize/kitchen_sink
Benchmarking ftml_stage/tokenize/kitchen_sink: Warming up for 500.00 ms
Benchmarking ftml_stage/tokenize/kitchen_sink: Collecting 10 samples in estimated 1.0113 s (2310 iterations)
Benchmarking ftml_stage/tokenize/kitchen_sink: Analyzing
ftml_stage/tokenize/kitchen_sink
                        time:   [439.98 µs 450.75 µs 460.93 µs]
                        thrpt:  [117.61 MiB/s 120.26 MiB/s 123.21 MiB/s]
Benchmarking ftml_stage/parse/kitchen_sink
Benchmarking ftml_stage/parse/kitchen_sink: Warming up for 500.00 ms
Benchmarking ftml_stage/parse/kitchen_sink: Collecting 10 samples in estimated 1.0786 s (550 iterations)
Benchmarking ftml_stage/parse/kitchen_sink: Analyzing
ftml_stage/parse/kitchen_sink
                        time:   [1.7884 ms 1.8485 ms 1.9638 ms]
                        thrpt:  [27.604 MiB/s 29.326 MiB/s 30.311 MiB/s]
Benchmarking ftml_stage/render_html/kitchen_sink
Benchmarking ftml_stage/render_html/kitchen_sink: Warming up for 500.00 ms
Benchmarking ftml_stage/render_html/kitchen_sink: Collecting 10 samples in estimated 1.0425 s (990 iterations)
Benchmarking ftml_stage/render_html/kitchen_sink: Analyzing
ftml_stage/render_html/kitchen_sink
                        time:   [1.0434 ms 1.1550 ms 1.2742 ms]
                        thrpt:  [42.542 MiB/s 46.933 MiB/s 51.953 MiB/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high severe
Benchmarking ftml_stage/preprocess/large_kitchen_sink
Benchmarking ftml_stage/preprocess/large_kitchen_sink: Warming up for 500.00 ms
Benchmarking ftml_stage/preprocess/large_kitchen_sink: Collecting 10 samples in estimated 1.3359 s (165 iterations)
Benchmarking ftml_stage/preprocess/large_kitchen_sink: Analyzing
ftml_stage/preprocess/large_kitchen_sink
                        time:   [7.8156 ms 7.9186 ms 8.0620 ms]
                        thrpt:  [60.831 MiB/s 61.933 MiB/s 62.749 MiB/s]
Benchmarking ftml_stage/tokenize/large_kitchen_sink
Benchmarking ftml_stage/tokenize/large_kitchen_sink: Warming up for 500.00 ms
Benchmarking ftml_stage/tokenize/large_kitchen_sink: Collecting 10 samples in estimated 1.1387 s (275 iterations)
Benchmarking ftml_stage/tokenize/large_kitchen_sink: Analyzing
ftml_stage/tokenize/large_kitchen_sink
                        time:   [4.1818 ms 4.3470 ms 4.5145 ms]
                        thrpt:  [108.07 MiB/s 112.24 MiB/s 116.67 MiB/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high mild
Benchmarking ftml_stage/parse/large_kitchen_sink
Benchmarking ftml_stage/parse/large_kitchen_sink: Warming up for 500.00 ms

Warning: Unable to complete 10 samples in 1.0s. You may wish to increase target time to 1.1s or enable flat sampling.
Benchmarking ftml_stage/parse/large_kitchen_sink: Collecting 10 samples in estimated 1.0752 s (55 iterations)
Benchmarking ftml_stage/parse/large_kitchen_sink: Analyzing
ftml_stage/parse/large_kitchen_sink
                        time:   [17.432 ms 18.321 ms 19.492 ms]
                        thrpt:  [25.031 MiB/s 26.631 MiB/s 27.989 MiB/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high mild
Benchmarking ftml_stage/render_html/large_kitchen_sink
Benchmarking ftml_stage/render_html/large_kitchen_sink: Warming up for 500.00 ms

Warning: Unable to complete 10 samples in 1.0s. You may wish to increase target time to 1.2s or enable flat sampling.
Benchmarking ftml_stage/render_html/large_kitchen_sink: Collecting 10 samples in estimated 1.2454 s (55 iterations)
Benchmarking ftml_stage/render_html/large_kitchen_sink: Analyzing
ftml_stage/render_html/large_kitchen_sink
                        time:   [22.327 ms 22.833 ms 23.276 ms]
                        thrpt:  [20.961 MiB/s 21.368 MiB/s 21.852 MiB/s]
Benchmarking ftml_stage/preprocess/plain_text
Benchmarking ftml_stage/preprocess/plain_text: Warming up for 500.00 ms
Benchmarking ftml_stage/preprocess/plain_text: Collecting 10 samples in estimated 1.0052 s (4675 iterations)
Benchmarking ftml_stage/preprocess/plain_text: Analyzing
ftml_stage/preprocess/plain_text
                        time:   [211.07 µs 211.75 µs 212.86 µs]
                        thrpt:  [294.27 MiB/s 295.81 MiB/s 296.78 MiB/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high mild
Benchmarking ftml_stage/tokenize/plain_text
Benchmarking ftml_stage/tokenize/plain_text: Warming up for 500.00 ms
Benchmarking ftml_stage/tokenize/plain_text: Collecting 10 samples in estimated 1.0027 s (8690 iterations)
Benchmarking ftml_stage/tokenize/plain_text: Analyzing
ftml_stage/tokenize/plain_text
                        time:   [113.97 µs 114.50 µs 115.29 µs]
                        thrpt:  [539.24 MiB/s 542.95 MiB/s 545.47 MiB/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high severe
Benchmarking ftml_stage/parse/plain_text
Benchmarking ftml_stage/parse/plain_text: Warming up for 500.00 ms
Benchmarking ftml_stage/parse/plain_text: Collecting 10 samples in estimated 1.0024 s (825 iterations)
Benchmarking ftml_stage/parse/plain_text: Analyzing
ftml_stage/parse/plain_text
                        time:   [1.0198 ms 1.0412 ms 1.0671 ms]
                        thrpt:  [58.258 MiB/s 59.708 MiB/s 60.963 MiB/s]
Found 2 outliers among 10 measurements (20.00%)
  1 (10.00%) low mild
  1 (10.00%) high severe
Benchmarking ftml_stage/render_html/plain_text
Benchmarking ftml_stage/render_html/plain_text: Warming up for 500.00 ms
Benchmarking ftml_stage/render_html/plain_text: Collecting 10 samples in estimated 1.0039 s (7150 iterations)
Benchmarking ftml_stage/render_html/plain_text: Analyzing
ftml_stage/render_html/plain_text
                        time:   [140.54 µs 147.07 µs 153.63 µs]
                        thrpt:  [404.67 MiB/s 422.72 MiB/s 442.35 MiB/s]
Benchmarking ftml_stage/preprocess/markup_dense
Benchmarking ftml_stage/preprocess/markup_dense: Warming up for 500.00 ms
Benchmarking ftml_stage/preprocess/markup_dense: Collecting 10 samples in estimated 1.0044 s (6160 iterations)
Benchmarking ftml_stage/preprocess/markup_dense: Analyzing
ftml_stage/preprocess/markup_dense
                        time:   [154.03 µs 155.98 µs 157.25 µs]
                        thrpt:  [399.98 MiB/s 403.25 MiB/s 408.35 MiB/s]
Benchmarking ftml_stage/tokenize/markup_dense
Benchmarking ftml_stage/tokenize/markup_dense: Warming up for 500.00 ms
Benchmarking ftml_stage/tokenize/markup_dense: Collecting 10 samples in estimated 1.0035 s (6105 iterations)
Benchmarking ftml_stage/tokenize/markup_dense: Analyzing
ftml_stage/tokenize/markup_dense
                        time:   [162.29 µs 163.19 µs 163.85 µs]
                        thrpt:  [382.73 MiB/s 384.27 MiB/s 386.41 MiB/s]
Benchmarking ftml_stage/parse/markup_dense
Benchmarking ftml_stage/parse/markup_dense: Warming up for 500.00 ms
Benchmarking ftml_stage/parse/markup_dense: Collecting 10 samples in estimated 1.0320 s (770 iterations)
Benchmarking ftml_stage/parse/markup_dense: Analyzing
ftml_stage/parse/markup_dense
                        time:   [1.4061 ms 1.4403 ms 1.4801 ms]
                        thrpt:  [42.370 MiB/s 43.538 MiB/s 44.599 MiB/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high mild
Benchmarking ftml_stage/render_html/markup_dense
Benchmarking ftml_stage/render_html/markup_dense: Warming up for 500.00 ms
Benchmarking ftml_stage/render_html/markup_dense: Collecting 10 samples in estimated 1.0441 s (440 iterations)
Benchmarking ftml_stage/render_html/markup_dense: Analyzing
ftml_stage/render_html/markup_dense
                        time:   [2.3171 ms 2.3598 ms 2.4400 ms]
                        thrpt:  [25.701 MiB/s 26.575 MiB/s 27.064 MiB/s]
Found 2 outliers among 10 measurements (20.00%)
  2 (20.00%) high severe

Benchmarking ftml_full_pipeline/kitchen_sink
Benchmarking ftml_full_pipeline/kitchen_sink: Warming up for 500.00 ms
Benchmarking ftml_full_pipeline/kitchen_sink: Collecting 10 samples in estimated 1.0525 s (330 iterations)
Benchmarking ftml_full_pipeline/kitchen_sink: Analyzing
ftml_full_pipeline/kitchen_sink
                        time:   [3.2211 ms 3.3336 ms 3.5258 ms]
                        thrpt:  [15.455 MiB/s 16.346 MiB/s 16.917 MiB/s]
Benchmarking ftml_full_pipeline/large_kitchen_sink
Benchmarking ftml_full_pipeline/large_kitchen_sink: Warming up for 500.00 ms
Benchmarking ftml_full_pipeline/large_kitchen_sink: Collecting 10 samples in estimated 1.0079 s (20 iterations)
Benchmarking ftml_full_pipeline/large_kitchen_sink: Analyzing
ftml_full_pipeline/large_kitchen_sink
                        time:   [49.895 ms 51.442 ms 53.548 ms]
                        thrpt:  [9.1584 MiB/s 9.5335 MiB/s 9.8290 MiB/s]
Found 1 outliers among 10 measurements (10.00%)
  1 (10.00%) high mild
Benchmarking ftml_full_pipeline/plain_text
Benchmarking ftml_full_pipeline/plain_text: Warming up for 500.00 ms
Benchmarking ftml_full_pipeline/plain_text: Collecting 10 samples in estimated 1.0393 s (605 iterations)
Benchmarking ftml_full_pipeline/plain_text: Analyzing
ftml_full_pipeline/plain_text
                        time:   [1.6127 ms 1.7048 ms 1.8860 ms]
                        thrpt:  [33.212 MiB/s 36.742 MiB/s 38.842 MiB/s]
Benchmarking ftml_full_pipeline/markup_dense
Benchmarking ftml_full_pipeline/markup_dense: Warming up for 500.00 ms
Benchmarking ftml_full_pipeline/markup_dense: Collecting 10 samples in estimated 1.2178 s (275 iterations)
Benchmarking ftml_full_pipeline/markup_dense: Analyzing
ftml_full_pipeline/markup_dense
                        time:   [4.2761 ms 4.3783 ms 4.4519 ms]
                        thrpt:  [14.129 MiB/s 14.366 MiB/s 14.709 MiB/s]
```
