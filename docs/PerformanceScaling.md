# FTML performance scaling results (2026-07-09)

This note records the measured effect of the performance work merged in
PRs #180-#185 against the pre-optimization baseline. It complements
`PerformanceBaseline.md`, which captured the original single-corpus baseline
and hotspot profile.

## What changed

| PR | Change | Nature |
|---|---|---|
| #180 | Benchmark harness + baseline profile | measurement |
| #181 | Preprocessor replacement passes made linear | removes O(n^2) |
| #182 | ICU date formatter cache | constant factor |
| #183 | lightningcss style cache | constant factor |
| #184 | Render document TOC list once per pass | removes redundant O(toc_blocks x entries) CPU |
| #185 | Strikethrough regression coverage | test-only |

All changes preserve behavior. Rendered output is byte-for-byte identical
between the baseline and the optimized build at every document size measured
below (verified by comparing output body lengths and by the existing fixture
suite).

## Headline result: preprocessing was quadratic

The baseline preprocessor rewrote the document with repeated in-place
`String::replace_range` calls, each of which shifts the entire tail of the
buffer. That is O(n^2) in document size. PR #181 rewrote every pass as a
single left-to-right scan into a fresh buffer, making preprocessing linear.

Measured preprocess-stage wall time (median), baseline vs optimized:

### Representative content (repeated multi-feature fixtures)

| Input size | Baseline | Optimized | Speedup |
|---:|---:|---:|---:|
| 256 KiB | 2.61 ms | 0.69 ms | 3.8x |
| 1 MiB | 46.1 ms | 2.94 ms | 15.7x |
| 4 MiB | 739.9 ms | 12.4 ms | 59.7x |
| 8 MiB | 3805.1 ms | 27.3 ms | 139.2x |

### Preprocess-stressing content (non-standard leading whitespace, CRLF)

| Input size | Baseline | Optimized | Speedup |
|---:|---:|---:|---:|
| 256 KiB | 34.1 ms | 1.77 ms | 19.3x |
| 1 MiB | 792.4 ms | 7.16 ms | 110.6x |
| 4 MiB | 13266.1 ms | 29.5 ms | 449.2x |
| 8 MiB | 80734.3 ms | 59.7 ms | 1352x |

At 8 MiB of preprocess-heavy content the baseline spent over 80 seconds; the
optimized build finishes in about 60 ms. This is the dominant robustness win:
a large page can no longer stall preprocessing for seconds to minutes.

## Full-pipeline scaling

Full pipeline = preprocess + tokenize + parse + render HTML. Measured on the
repeated kitchen-sink corpus (each repeated seed contains a `[[toc]]` block):

| Input size | Baseline | Optimized | Speedup | Output body |
|---:|---:|---:|---:|---:|
| 74 KiB | 4.47 ms | 4.23 ms | 1.06x | 0.4 MB |
| 260 KiB | 19.8 ms | 15.7 ms | 1.26x | 2.8 MB |
| 1.0 MiB | 149.8 ms | 80.1 ms | 1.87x | 34.7 MB |
| 4.0 MiB | 1842.9 ms | 529.5 ms | 3.48x | 505 MB |
| 8.0 MiB | 8598.2 ms | 1750.6 ms | 4.91x | 1.99 GB |

The full-pipeline speedup grows with document size (superlinearity is being
removed) but is bounded on *this* corpus by a property of the corpus itself:
every repeated seed contains a `[[toc]]` block, and each TOC block emits the
whole document-global table of contents. With B TOC blocks and E entries the
*output* is O(B x E), i.e. genuinely quadratic in document size (1.99 GB of
HTML from 8 MiB of input). PR #184 removed the redundant CPU cost of
re-rendering that list for each block (it is now rendered once and copied),
but the bytes must still be written, so the output-size quadratic remains.
Collapsing it would require changing TOC semantics (render once, reference
elsewhere), which is a Wikidot/Wikijump compatibility decision outside FTML's
responsibility boundary and is intentionally not attempted here.

On the standard 500 KiB benchmark corpus the full pipeline improved from
50.16 ms to 33.95 ms (1.48x).

## Per-stage state after this work (8 MiB kitchen corpus, optimized)

| Stage | Time | Notes |
|---|---:|---|
| preprocess | 27.3 ms | linear (was O(n^2)) |
| tokenize | 59.4 ms | already linear; hand-written scanner |
| parse | 224.4 ms | near-linear; now the dominant stage |
| render | 113.0 ms | linear on non-TOC-cross-product content |

Parse is the largest remaining stage. Profiling (see `PerformanceBaseline.md`)
found it near-linear with no exponential backtracking; the main candidate is
speculative dash-strikethrough collection, but a prototype short-circuit
yielded only ~1% and carried compatibility risk, so it was not merged (its
behavior-lock regression tests were kept in PR #185).

## Methodology

Measurements use release builds (`lto = true`) run back-to-back, single
process, on an otherwise idle machine. Corpora are built deterministically by
repeating checked-in `test/**/input.ftml` fixtures, the same construction the
Criterion harness in `benches/performance.rs` uses. Each data point is a median
over multiple iterations after warmup. Baseline is commit `cea507c7b` (harness
present, optimizations absent); optimized is the merged result of PRs
#181-#184.
