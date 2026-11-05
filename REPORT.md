# Super-linear parse/render scaling hunt — report

Branch: `claude/ftml-parser-perf-bugs-9n4auw` (base `c1dc640`).

## STATUS

**This is a measurement-only checkpoint. No parser or renderer code has
changed, so there are no "after" numbers.** Work was stopped on request after
the baseline sweep finished.

| Item | State |
|---|---|
| Harness: opt-in `CORPUS_LAYOUT` / `CORPUS_STAGE_COLUMNS` in `examples/corpus_profile.rs` | **Done**, committed. Default behaviour and digest format are unchanged. |
| Adversarial family generator + scaling driver (`scripts/perf_scaling/`) | **Done**, committed. 237 families. |
| Baseline sweep, Wikidot layout, all 237 families | **Done.** Results below. |
| Small-size probe of the 4 timeout families | **Partial.** 2 of 4 measured before the run was stopped. Both show exponential growth. |
| Root-causing any family to its first wrong step | **Not started.** |
| Fixes | **Not started.** No source change, so corpus byte-identity is trivially preserved. |
| "After" measurements | **Not applicable.** No fix landed. |
| Timing regression tests | **Not started.** |
| Wikijump-layout sweep | **Not started.** Every figure here is Wikidot layout. |

## Method

- **Harness:** `examples/corpus_profile.rs`, run with `CORPUS_LAYOUT=wikidot`
  so it uses `preprocess_for_layout(Wikidot)`, the production Wikidot
  pipeline. `CORPUS_STAGE_COLUMNS=1` adds per-stage microseconds. Each point
  is one page per process, so a pathological size can be killed with a
  timeout.
- **Build:** release profile with LTO off and 16 codegen units
  (`CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16`).
  Absolute times run somewhat slower than an LTO build; growth rates are
  unaffected.
- **Families:** `scripts/perf_scaling/families.py`. Each family maps a
  repetition count `n` to an input whose size grows linearly with `n`. They
  cover block pairing, inline scopes, tables, lists, bibliography, footnotes,
  comments, gates (`iftags`/`ifcategory`), links, typography and
  preprocessing.
- **Ladder:** `n = 256, 512, …, 65536`. Each family stops growing at the first
  point over 1.5 s, or at a 20 s timeout.
- **Exponent `k`:** the log-log slope of time against input bytes between the
  last two points that took more than 3 ms. `k ≈ 1` is linear, `k ≈ 2`
  quadratic.
- **Classes:** a family counts as super-linear when `k ≥ 1.4`.
- **Noise caveats:**
  - Each point is a single run.
  - Three shards ran in parallel on a 4-core machine.
  - `k` is unreliable when the last point is only tens of milliseconds:
    `pipe_run` (37 ms), `caret_run` (25 ms) and `dot_run` (110 ms) are
    probably noise, not real super-linearity.

### Reproduce

```sh
CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
  cargo build --release --example corpus_profile
python3 scripts/perf_scaling/drive.py target/release/examples/corpus_profile out.json [family-substring ...]
# env: LAYOUT=wikidot|wikijump START=256 MAXN=65536 BUDGET_MS=1500 TIMEOUT=30 SHARD=i/k
```

The raw result JSON (about 400 KB) is not committed. The driver regenerates
it.

## Results summary (Wikidot layout, before any fix)

237 families:

- **4** time out.
- **107** grow super-linearly (`k ≥ 1.4`).
- **126** grow linearly.

Almost every slow family spends 90–99 % of its time in the **parse** stage.
The exceptions are noted in the tables.

### 1. Exponential: timeouts at 4–7 KB (highest priority)

All four exceeded 20 s at the first ladder point (`n = 256`). The interrupted
small-size probe (`START=4`) shows a cliff rather than a polynomial:

| Family | Input unit | Measured points (bytes → time) |
|---|---|---|
| `mixed_inline_unclosed` | `**//__--^^,,{{a ` | 64 B → 2.7 ms; 128 B → 3.0 ms; 256 B → 5.0 ms; **512 B → 17.3 s** |
| `tabview_tab_unclosed` | `[[tabview]]\n[[tab x]]\ny\n` | 96 B → 5.6 ms; 192 B → 2.5 ms; 384 B → 4.7 ms; 768 B → 14.1 ms; **1536 B → >30 s** |
| `mixed_inline_lines` | `**//__--^^,,{{a\n` | >20 s at 4096 B. Not probed smaller. |
| `comment_cross_div` | `[[div]]\n[!--\n[[/div]]\n--]\n` | >20 s at 6656 B. Not probed smaller. |

Doubling `mixed_inline_unclosed` from 256 to 512 bytes multiplies its time by
about 3,500. That is consistent with exponential backtracking across the
seven nested unclosed inline markers. A single page under 1 KB can stall the
renderer.

### 2. Roughly cubic: closed `[[iftags]]` gates (`k ≈ 3.2–3.6`)

This group is ordinary, well-formed markup that real pages use, which makes
it the most likely to be hit by accident.

| Family | k | Last bytes | ms | Dominant stage |
|---|---:|---:|---:|---|
| `iftags_false_with_code` | 3.55 | 17920 | 8205.1 | parse 99% |
| `iftags_closed_true` | 3.54 | 16384 | 6585.2 | parse 99% |
| `iftags_gate_seam` | 3.48 | 16896 | 7288.1 | parse 99% |
| `iftags_inline` | 3.46 | 14336 | 6516.2 | parse 99% |
| `iftags_false_with_comment` | 3.45 | 15872 | 7251.1 | parse 99% |
| `iftags_closed_false` | 3.37 | 16896 | 6450.8 | parse 99% |
| `if_mixed_cross` | 3.22 | 30208 | 6823.1 | parse 99% |

For example, `iftags_closed_true` is just
`[[iftags -x]]\nshown\n[[/iftags]]\n` repeated. 512 such gates (16 KB) take
6.6 s. The true and false conditions are equally slow.

### 3. Quadratic with large constants (seconds at 8–40 KB)

These are the worst entries by milliseconds per KiB at the last point
(excluding groups 1 and 2):

| Family | k | Last bytes | ms |
|---|---:|---:|---:|
| `bracket_singles` (`[` run) | 2.04 | 8192 | 4136.5 |
| `block_bracket_run` (`[[` run) | 2.06 | 8192 | 3601.4 |
| `note_unclosed` | 2.24 | 11264 | 4712.1 |
| `tab_unclosed` | 1.89 | 12300 | 3426.5 |
| `footnote_in_div_unclosed` | 2.01 | 22528 | 5340.1 |
| `footnote_unclosed_lines` | 1.72 | 15360 | 2428.2 |
| `brackets_mix` (`[]` run) | 2.16 | 32768 | 5082.9 |
| `footnote_unclosed` | 1.96 | 14336 | 1873.2 |
| `color_closed_unpiped` | 2.10 | 40960 | 4827.6 |
| `html_unclosed` | 2.02 | 36864 | 4012.6 |
| `math_unclosed` | 1.81 | 18432 | 1826.7 |
| `bib_unclosed` | 1.87 | 25600 | 2416.6 |
| `bib_unclosed_with_cites` | 2.02 | 39936 | 3665.9 |
| `collapsible_unclosed` | 1.92 | 18432 | 1578.8 |

The common shape is a repeated **unclosed opener**, or a bare bracket run, in
which each candidate appears to rescan the rest of the document. That
diagnosis is a hypothesis: none of these families has been root-caused yet.

### 4. Super-linear outside the parse stage

| Family | k | Last bytes | ms | Dominant stage |
|---|---:|---:|---:|---|
| `iftags_closer_only` | 1.79 | 393216 | 3075.9 | preprocess 90% |
| `iftags_nested` | 1.57 | 425986 | 1655.4 | preprocess 94% |
| `module_unknown_unclosed` | 1.94 | 122880 | 1914.7 | preprocess 91% |
| `div_close_partial` | 2.34 | 57352 | 3153.5 | preprocess 69% |
| `math_closer_partial` | 1.94 | 65545 | 2453.6 | preprocess 55% |
| `include_unclosed_args` | 1.73 | 147456 | 2873.2 | preprocess 59% |
| `star_link` | 1.51 | 589824 | 1739.7 | preprocess 89% |
| `comment_closed` | 1.45 | 720896 | 1450.0 | preprocess 99% |
| `toc_lines` | 3.32 | 49152 | 2736.3 | render 95% |

`toc_lines` (`[[toc]]` plus a heading per repetition) is the known TOC output
cross-product that `docs/PerformanceScaling.md` already documents: B TOC
blocks × E entries in the output. It is intentionally out of scope.

### Full super-linear table (`k ≥ 1.4`), worst milliseconds per KiB first

Columns: family | k | last completed input bytes | ms at that size |
dominant stage (share).

| Family | k | Last bytes | ms | Dominant stage |
|---|---:|---:|---:|---|
| `bracket_singles` | 2.04 | 8192 | 4136.5 | parse 99% |
| `iftags_false_with_code` | 3.55 | 17920 | 8205.1 | parse 99% |
| `iftags_false_with_comment` | 3.45 | 15872 | 7251.1 | parse 99% |
| `iftags_inline` | 3.46 | 14336 | 6516.2 | parse 99% |
| `block_bracket_run` | 2.06 | 8192 | 3601.4 | parse 99% |
| `iftags_gate_seam` | 3.48 | 16896 | 7288.1 | parse 99% |
| `note_unclosed` | 2.24 | 11264 | 4712.1 | parse 99% |
| `iftags_closed_true` | 3.54 | 16384 | 6585.2 | parse 99% |
| `iftags_closed_false` | 3.37 | 16896 | 6450.8 | parse 99% |
| `tab_unclosed` | 1.89 | 12300 | 3426.5 | parse 99% |
| `footnote_in_div_unclosed` | 2.01 | 22528 | 5340.1 | parse 99% |
| `if_mixed_cross` | 3.22 | 30208 | 6823.1 | parse 99% |
| `footnote_unclosed_lines` | 1.72 | 15360 | 2428.2 | parse 99% |
| `brackets_mix` | 2.16 | 32768 | 5082.9 | parse 98% |
| `footnote_unclosed` | 1.96 | 14336 | 1873.2 | parse 99% |
| `color_closed_unpiped` | 2.10 | 40960 | 4827.6 | parse 99% |
| `color_bare` | 1.78 | 16384 | 1854.2 | parse 99% |
| `html_unclosed` | 2.02 | 36864 | 4012.6 | parse 99% |
| `hash_run` | 2.40 | 16384 | 1641.0 | parse 99% |
| `math_unclosed` | 1.81 | 18432 | 1826.7 | parse 99% |
| `block_closer_bracket_run` | 2.02 | 24576 | 2388.8 | parse 80% |
| `bib_unclosed` | 1.87 | 25600 | 2416.6 | parse 99% |
| `block_unclosed_head` | 2.11 | 65536 | 6062.0 | parse 93% |
| `bib_unclosed_with_cites` | 2.02 | 39936 | 3665.9 | parse 99% |
| `collapsible_unclosed` | 1.92 | 18432 | 1578.8 | parse 99% |
| `comment_in_inline` | 1.94 | 57344 | 4696.6 | parse 99% |
| `brace_run` | 2.37 | 65536 | 4680.5 | parse 99% |
| `raw_block_unclosed` | 2.17 | 32768 | 2325.6 | parse 99% |
| `spaced_unmatched_close` | 2.15 | 98304 | 6624.8 | parse 66% |
| `module_css_unclosed` | 1.80 | 61440 | 3951.4 | parse 87% |
| `mono_unclosed` | 2.21 | 65536 | 3816.8 | parse 99% |
| `anchor_link_unclosed` | 1.73 | 40960 | 2349.1 | parse 99% |
| `embed_unclosed` | 1.51 | 40960 | 2332.8 | parse 99% |
| `toc_lines` | 3.32 | 49152 | 2736.3 | render 95% |
| `sub_unclosed` | 2.14 | 32768 | 1812.3 | parse 99% |
| `div_close_partial` | 2.34 | 57352 | 3153.5 | preprocess 69% |
| `div_unclosed_inline` | 1.97 | 57344 | 3111.2 | parse 99% |
| `module_listpages_unclosed` | 2.00 | 63488 | 3387.0 | parse 95% |
| `strike_unclosed` | 2.41 | 65536 | 3212.5 | parse 99% |
| `sup_unclosed` | 1.55 | 32768 | 1538.5 | parse 99% |
| `triple_link_unclosed` | 1.66 | 40960 | 1683.8 | parse 99% |
| `math_closer_partial` | 1.94 | 65545 | 2453.6 | preprocess 55% |
| `table_cell_bold_unclosed` | 2.14 | 139264 | 5077.2 | parse 98% |
| `strike_dash_runs` | 2.53 | 90112 | 3159.2 | parse 98% |
| `list_ol_unclosed` | 1.95 | 61440 | 2066.4 | parse 97% |
| `ul_li_unclosed` | 1.94 | 57344 | 1911.3 | parse 98% |
| `comment_dashes` | 1.50 | 49152 | 1544.9 | parse 99% |
| `comment_closer_only` | 2.08 | 65536 | 2032.4 | parse 99% |
| `raw_angle_unclosed` | 2.08 | 65536 | 2006.5 | parse 99% |
| `mono_nested` | 2.68 | 65537 | 1905.7 | parse 99% |
| `bold_in_table` | 2.03 | 163840 | 4638.4 | parse 97% |
| `single_link_hash` | 1.89 | 81920 | 2292.9 | parse 93% |
| `comment_unclosed` | 1.71 | 114688 | 2981.6 | parse 99% |
| `table_block_nested` | 1.85 | 116738 | 2780.4 | parse 97% |
| `include_unclosed_args` | 1.73 | 147456 | 2873.2 | preprocess 59% |
| `inline_math_unclosed` | 1.82 | 98304 | 1734.6 | parse 99% |
| `button_unclosed` | 1.51 | 106496 | 1746.3 | parse 99% |
| `module_unknown_unclosed` | 1.94 | 122880 | 1914.7 | preprocess 91% |
| `iftags_true_nested` | 1.59 | 212994 | 2910.7 | parse 84% |
| `iftags_sparse` | 1.91 | 229405 | 2548.4 | parse 69% |
| `bold_nested_paren` | 2.16 | 262145 | 2315.2 | parse 98% |
| `url_unclosed_bracket` | 1.59 | 327680 | 2757.9 | parse 97% |
| `iftags_closer_only` | 1.79 | 393216 | 3075.9 | preprocess 90% |
| `bib_entries_closed` | 2.21 | 524323 | 3545.6 | parse 94% |
| `single_link_owner_recovery` | 1.90 | 344070 | 1761.4 | parse 96% |
| `double_angle_unclosed` | 1.90 | 327680 | 1442.1 | parse 93% |
| `div_closer_trailing_space` | 1.92 | 393224 | 1691.2 | parse 75% |
| `underline_unclosed` | 1.95 | 262144 | 1043.1 | parse 94% |
| `italic_per_line` | 1.65 | 262144 | 1034.0 | parse 73% |
| `iftags_nested` | 1.57 | 425986 | 1655.4 | preprocess 94% |
| `italic_unclosed` | 1.55 | 262144 | 978.9 | parse 96% |
| `bold_unclosed` | 1.49 | 262144 | 910.8 | parse 94% |
| `block_open_quote` | 1.43 | 557056 | 1848.7 | parse 79% |
| `percent_run` | 1.82 | 65536 | 214.2 | parse 94% |
| `star_div_unclosed` | 1.41 | 589824 | 1863.3 | parse 68% |
| `ruby_unclosed` | 1.67 | 720896 | 2229.6 | parse 93% |
| `bib_entries_unclosed` | 1.47 | 524305 | 1563.8 | parse 73% |
| `star_link` | 1.51 | 589824 | 1739.7 | preprocess 89% |
| `entity_run` | 1.93 | 327680 | 946.6 | parse 92% |
| `module_closer_only` | 2.01 | 786432 | 2258.9 | parse 82% |
| `blockquote_bold` | 1.54 | 393216 | 1093.5 | parse 83% |
| `div_unclosed_lines` | 1.42 | 524288 | 1404.2 | parse 69% |
| `heading_unclosed_inline` | 1.67 | 393216 | 1033.7 | parse 77% |
| `nbsp_lines` | 1.51 | 262144 | 635.6 | parse 78% |
| `code_unclosed` | 1.56 | 589824 | 1382.7 | parse 85% |
| `toc_inline` | 1.42 | 524288 | 1226.1 | parse 92% |
| `div_closer_only` | 1.49 | 589824 | 1221.7 | parse 73% |
| `typo_quotes` | 1.65 | 393216 | 808.2 | parse 87% |
| `table_block_full` | 1.43 | 1933312 | 3965.4 | parse 53% |
| `typo_angles` | 1.67 | 393216 | 796.9 | parse 90% |
| `comment_closed` | 1.45 | 720896 | 1450.0 | preprocess 99% |
| `leading_ws_lines` | 1.51 | 327680 | 640.7 | parse 78% |
| `list_ul_blocks_nested` | 1.50 | 983042 | 1772.0 | preprocess 73% |
| `raw_unclosed` | 1.77 | 262144 | 448.8 | parse 78% |
| `span_unclosed` | 1.59 | 589824 | 992.3 | parse 92% |
| `dot_run` | 2.35 | 65536 | 109.8 | parse 73% |
| `div_open_then_text` | 1.56 | 655368 | 1050.0 | parse 83% |
| `table_wide_row` | 2.11 | 327683 | 503.7 | parse 75% |
| `ifcategory_nested` | 1.45 | 1081346 | 1629.1 | parse 76% |
| `module_css_nested` | 1.81 | 1769474 | 2651.3 | parse 65% |
| `tab_closed_unclosed_view` | 1.50 | 1376268 | 2001.1 | preprocess 53% |
| `table_span_cells` | 1.90 | 720896 | 934.9 | parse 48% |
| `collapsible_nested` | 1.47 | 2162690 | 2698.7 | parse 68% |
| `hr_lines` | 1.48 | 327680 | 399.5 | preprocess 53% |
| `footnote_first` | 1.52 | 1769472 | 1442.8 | parse 66% |
| `pipe_run` | 2.95 | 65536 | 37.2 | preprocess 85% |
| `caret_run` | 1.54 | 65536 | 25.0 | parse 60% |

### Linear families (`k < 1.4`)

`a_unclosed`, `align_center_nested`, `align_mixed`, `align_unclosed`, `anchor_unclosed`, `angle_run`, `at_run`, `backslash_run`, `bib_closer_only`, `bib_entries_bad`, `bib_entry_noise`, `bib_nested`, `bibcite_run`, `bibcite_unclosed`, `block_bracket_run_space`, `blockquote_block_unclosed`, `blockquote_deep`, `blockquote_flat`, `blockquote_sawtooth`, `bold_across_paragraphs`, `bold_cross_div`, `bold_in_list`, `bold_per_line`, `bold_run`, `brace_close_run`, `bracket_close_singles`, `bracket_pairs`, `char_blocks`, `clear_float`, `code_closer_only`, `code_type_unclosed`, `colon_run`, `color_unclosed`, `comma_run`, `comment_inside_code`, `comment_nested_like`, `comment_open_far_close`, `comment_unclosed_lines`, `concat_newline`, `continuation_lines`, `crlf_lines`, `crossed_div_span_close`, `dash_run`, `dash_space_run`, `date_blocks`, `def_list`, `def_list_unclosed`, `div_crossed_span`, `div_nested`, `div_underscore_unclosed`, `dollar_run`, `double_dollar`, `email_like`, `entity_unterminated`, `equal_run`, `footnote_block_bad_args`, `footnote_block_run`, `footnote_closed`, `footnote_closer_only`, `footnote_cross_div`, `footnote_nested`, `gallery_unclosed`, `heading_deep`, `heading_lines`, `hidden_unclosed`, `ifcategory_closed`, `ifcategory_unclosed`, `iframe_lines`, `iftags_false_unclosed_children`, `iftags_unclosed`, `image_blocks`, `include_lines`, `inline_interleave`, `inline_math_closed`, `invisible_unclosed`, `li_unclosed`, `lines_blocks`, `list_deep`, `list_deep_space`, `list_flat`, `list_in_div`, `list_item_div_unclosed`, `list_mixed`, `list_numbered`, `list_sawtooth`, `list_with_blank`, `lt_run`, `nested_spans_crossing`, `newline_run`, `plus_run`, `quote_run`, `size_unclosed`, `size_unclosed_lines`, `slash_run`, `social_blocks`, `space_run`, `span_closer_only`, `span_nested`, `span_style_quotes`, `star_run`, `tab_lines`, `tab_run`, `table_block_unclosed`, `table_cell_block_unclosed`, `table_cell_closer_only`, `table_header_cells`, `table_in_div`, `table_row_block_unclosed`, `table_row_pipes_newline`, `table_row_unclosed`, `table_rows`, `table_rows_with_div`, `text_run`, `tilde_run`, `trailing_ws_lines`, `triple_link_closed`, `triple_link_pipe`, `triple_raw_probe`, `typo_dashes`, `typo_ellipsis`, `underscore_breaks`, `underscore_run`, `url_newline_bracket`, `url_run`, `user_blocks`, `variable_lines`

## Remaining work (recommended order)

1. **Exponential families (group 1).**
   - Find each family's first wrong step. Valgrind/callgrind is available; a
     profiling binary at `n` = 16–32 is enough.
   - Bound the backtracking with a memo or a budget that leaves output
     unchanged.
   - Pin each fix with a timing test in the style of
     `tests/security_regressions.rs::hostile_wikidot_lookahead_families_remain_bounded`
     (`Instant` plus a `< 3 s` assertion).
2. **Cubic `[[iftags]]` gates (group 2).** This is the most likely real-world
   hit: plain repeated closed gates.
3. **High-constant quadratics (group 3)**, starting with the bracket runs
   (`[`, `[[`, `[]`) and the unclosed `note`, `tab`, `footnote`, `html`,
   `math`, `bibliography` and `collapsible` openers.
4. **Preprocess-stage super-linearity (group 4)**, especially the two
   `[[iftags]]` preprocessing families.
5. **Validation after each fix:**
   - Check that `corpus_profile` digests are identical before and after.
   - Run the fixture suite (`cargo test`); revert any unrelated
     `FTML_UPDATE_TESTS` drift.
   - Run `cargo fmt --check` and
     `RUSTFLAGS='-D warnings' cargo clippy --tests --no-deps`.
6. **Re-run the sweep** after the fixes, and once in Wikijump layout, to fill
   in the "after" columns.
