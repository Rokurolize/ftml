# Item 95: Corpus Distillation Parser Defects (`badpages/`)

## Stage 1: Baseline Reproduction on `origin/main` (`b62dfc730a`)

### Build Command

```bash
bash ~/.local/state/wikijump/muse-bhl/tools/heavy-build.sh cargo build --release --example render_html_jsonl
```

### Reproduction Command

```bash
python3 -c '
import json, subprocess, time
BIN = "./target/release/examples/render_html_jsonl"
BASE = "/home/roku/.local/state/wikijump/muse-bhl/corpus-distill-20261109"
SCHEMA = "wikijump_syntax_differential.syntax_case.v1"

def run_source(case_id, source, timeout=120):
    payload = json.dumps({"schema": SCHEMA, "case_id": case_id, "source": source, "title": case_id}) + "\n"
    t0 = time.perf_counter()
    try:
        p = subprocess.run([BIN], input=payload.encode(), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)
        return p.returncode, time.perf_counter() - t0, p.stderr.decode(errors="replace").strip()
    except subprocess.TimeoutExpired:
        return -9, time.perf_counter() - t0, f"TIMEOUT>{timeout}s"

cases = {r["case_id"]: r["source"] for r in map(json.loads, open(BASE + "/min.cases.jsonl")) if r["case_id"] in {"mc02528", "mc04908", "mc05686", "mc07497", "mc07629"}}
orig_hub = next(r["source"] for r in map(json.loads, open(BASE + "/sources.jsonl")) if r["case_id"] == "scp-play-podcast-hub")

for name in ["sandbox-synth-0818.ftml", "sandbox-synth-0692.barrier.ftml", "sandbox-synth-0464.ftml", "sandbox-synth-0766.barrier.ftml"]:
    src = open(f"{BASE}/badpages/{name}").read()
    rc, el, err = run_source(name, src, timeout=60)
    print(name, len(src), rc, f"{el:.3f}s", err.splitlines()[0] if err else "")

four_concat = "\n\n".join(cases[k] for k in ["mc04908", "mc05686", "mc07497", "mc07629"])
print("4-case-concat", len(four_concat), *run_source("four_concat", four_concat))
print("scp-play-podcast-hub", len(orig_hub), *run_source("scp-play-podcast-hub", orig_hub))
print("mc02528", len(cases["mc02528"]), *run_source("mc02528", cases["mc02528"]))
'
```

### Baseline Measurements (`origin/main` `b62dfc730a`)

| Case | Source Size (chars / bytes) | Outcome on `origin/main` | Single-Thread Release Time | Notes |
| --- | ---: | --- | ---: | --- |
| `badpages/sandbox-synth-0818.ftml` | 120,639 / 121,370 | **PANIC** (`rc=101`) | 111.85 ms | `src/parsing/consume.rs:647:20` (was line 634 at `23f0a47795`): `end byte index 9 is not a char boundary; it is inside '“' (bytes 8..11 of string)` in `try_consume_wikidot_fused_span_close_then_table_column` |
| 4-case concat (`mc04908` + `mc05686` + `mc07497` + `mc07629`) | 2,172 / 2,191 | **PANIC** (`rc=101`) | 7.08 ms | Same panic at `src/parsing/consume.rs:647:20` |
| `badpages/sandbox-synth-0692.barrier.ftml` | 194,972 / 195,409 | **PANIC** (`rc=101`) | 216.20 ms | `src/parsing/rule/impls/block/blocks/list.rs:1777:54`: `index out of bounds: the len is 0 but the index is 0` in `wikidot_li_apply_b5` |
| `scp-play-podcast-hub` (unreduced source page of `mc02528`) | 54,984 / 54,984 | OK (`rc=0`) | 38.20 ms | Baseline full page |
| `mc02528` (reduced case from `scp-play-podcast-hub`) | 31,794 / 31,794 | **SLOW** (`rc=0`) | 8,580 ms (48.4 s under 8-way shard load) | ~225× slower than its 55 KB unreduced source page |
| `badpages/sandbox-synth-0464.ftml` (6-case pack containing `mc02528`) | 190,834 / 191,525 | **HANG** (`TIMEOUT`) | >180,000 ms | Exceeds 180 s single-threaded |
| `badpages/sandbox-synth-0766.barrier.ftml` (114-case barrier pack) | 185,034 / 185,681 | **SLOW** (`rc=0`) | 32,492 ms (>45 s under 8-way shard load) | Superlinear blowup across packed fragments |

---

## Stage 2: Defect 1 Root Cause & Fix (Both `badpages/` Panics)

### Mechanism 1A: UTF-8 Byte-Slice Panic in `try_consume_wikidot_fused_span_close_then_table_column` (`src/parsing/consume.rs:647`, was line 634 at `23f0a47795`)

- **Carriers**: `badpages/sandbox-synth-0818.ftml` (120,639 chars) and the 4-case minimal concatenation (`mc04908` + `mc05686` + `mc07497` + `mc07629`, 2,172 chars); minimal standalone reproducer `"[[span]]\n[[/>]]\n\n“\n[[/span]]"` (28 chars).
- **Interaction & Root Cause**:
  1. In the 4-case concatenation, `mc04908` opens `[[span style="position: relative; left: 1rem"]]` followed by `[[module Rate]]` and `[[/span]]`. `mc05686` begins with a stray `[[/module]]`, causing `[[module Rate]]` in `mc04908` to pair with `[[/module]]` in `mc05686` as a module body and swallow `[[/span]]`. Because `has_body_end_block(&BLOCK_SPAN)` succeeded before `[[module Rate]]` consumed the closer, `enter_wikidot_span_body` left `wikidot_span_body_depth > 0` across the subsequent fragments.
  2. At the end of `mc07497`, the parser encounters `[[/>]]` (`Token::LeftBlockEnd`, 6 bytes), followed by `\n\n` (2 bytes), followed by the start of `mc07629`: `“I didn’t die...` where `“` (U+201C) is a 3-byte UTF-8 sequence occupying bytes `8..11` relative to `start`.
  3. `try_consume_wikidot_fused_span_close_then_table_column` checked `rest.len() >= "[[/span]]".len() && rest[.."[[/span]]".len()].eq_ignore_ascii_case("[[/span]]")`. Since `"[[/span]]".len()` is 9, `rest[..9]` sliced `&str` at byte offset 9 inside the 3-byte `'“'` character (`bytes 8..11`), panicking.
- **Fix**: Replaced `rest.len() >= ... && rest[..9]` in `src/parsing/consume.rs` with `source.get(start..start + "[[/span]]".len()).is_some_and(|rest| rest.eq_ignore_ascii_case("[[/span]]"))` (and applied the same `.get(...)` char-boundary-safe prefix slice to three sibling fixed-length `.eq_ignore_ascii_case` checks in `src/preproc/whitespace.rs:529`, `src/parsing/rule/impls/block/blocks/table.rs:1053`, and `src/parsing/rule/impls/block/blocks/div.rs:107`).

### Mechanism 1B: Empty `closes` Slice Indexing Panic in `wikidot_li_apply_b5` (`src/parsing/rule/impls/block/blocks/list.rs:1777`)

- **Carrier**: `badpages/sandbox-synth-0692.barrier.ftml` (194,972 chars); minimal standalone reproducer `"[[ul]][[ul]]_[[/ul]][[div]]\n+ h\n[[/div]]"` (40 chars).
- **Interaction & Root Cause**:
  1. An outer `[[ul]]` contains an inner `[[ul]]_[[/ul]]` (which consumes the only `[[/ul]]`) and a `[[div]]` whose body contains a heading barrier (`+ h`), causing the outer `[[ul]]` to cross-terminate at `[[/div]]` and the inner `[[div]]` to emit `PartialElement::WikidotBlockOpen` inside the outer list's degraded literal item (`ListItem::Elements`).
  2. The outer `[[ul]]` has zero `[[li]]` partial items (`partial_count == 0`, `partial_item_indices == []`, `heads == []`, `closes == []`), while `items` holds one degraded `ListItem::Elements` containing `PartialElement::WikidotBlockOpen`.
  3. The call-site gate in `parse_list_block_impl` (`heads.len() == partial_count && closes.len() == partial_count && partial_item_indices.len() == partial_count && partial_item_indices.windows(2).all(...)`) evaluated to `true` vacuously when `partial_count == 0`.
  4. `wikidot_li_apply_b5` found a non-empty flat span from the `WikidotBlockOpen`, computed `base = partial_item_indices.first().copied().unwrap_or(0)` (`0`), drained `items` into `partials`, popped the degraded literal item, and indexed `closes[0]` on an empty `closes` slice.
- **Fix**: Required `partial_count > 0` at the `wikidot_li_apply_b5` call site in `parse_list_block_impl` and guarded `closes.get(hi)` inside `wikidot_li_apply_b5`.

### Defect 1 Before / After

| Case | Before (`origin/main`) | After Fix |
| --- | --- | --- |
| `badpages/sandbox-synth-0818.ftml` (120,639 chars) | **PANIC** at `src/parsing/consume.rs:647:20` | **OK (`rc=0`)** in **249.58 ms** |
| 4-case concat (`mc04908` + `mc05686` + `mc07497` + `mc07629`, 2,172 chars) | **PANIC** at `src/parsing/consume.rs:647:20` | **OK (`rc=0`)** in **8.04 ms** |
| `badpages/sandbox-synth-0692.barrier.ftml` (194,972 chars) | **PANIC** at `src/parsing/rule/impls/block/blocks/list.rs:1777:54` | **OK (`rc=0`)** in **704.10 ms** |


---

## Stage 3: Predecessor (Argon) Session-Log Reconstruction and Diff Judgment

### Sources

Argon session `ses_e3df02169ffe7qBkfbf7VzqEgN` (140 rows, `/home/roku/.traycer/host/opencode/line-2/db-a5010091/opencode.db`, `session_message` ordered by `seq`), lived 2026-11-10 05:46-07:26 JST, ended seq 1398 with `provider.quota` 402 `OUT_OF_CREDITS`. Reasoning text and tool inputs/outputs were extracted per `seq` and every claim below was cross-checked against the actual diff and code; the log is evidence, not authority.

### What Argon believed causes the blowup (Defect 2)

Argon concluded Defect 2 is **three compounding mechanisms**, not one:

1. **Tabview exponential re-measure** (`src/parsing/parser/block_pairs.rs`, `Scanner::measure_tabview`). Memoization was skipped entirely when `tab_body_scan_depth > 0` ("keep nested measures out of the memo"), so a chain of N nested `[[tabview]]` constructs was re-measured 2^k times (`V_k` measured 2^k times, `2^{n-1}` calls for n views). GDB sample (seq 753, `mc02528`) showed 13 stacked `measure_tabview` frames with `get_block_rule_with_name` at the leaf. GDB sample (seq 760, 0766) showed deep `align_center::parse_fn` -> `parse_alignment_block` -> `parse_tabview` -> `parse_tab` nesting instead.
2. **Center/div depth corruption through the rule-attempt cache** (`src/parsing/parser/wikidot_state.rs`, `WikidotState::attempt_key`). Only `div_body_depth` was clamped to 1 in `attempt_key`, while `scored_div_body_depth` and `center_body_depth` were left unclamped; and `apply_rule_attempt_success` replaced the live `wikidot` state with the cached shallow value, so a non-"block" rule cached at depth 1 replayed over a live depth-3+ `[[=]]`/`[[div]]` stack overwrote the depths. Net effect: `in_wikidot_center_body()` flickered inside deeply nested `[[=]]` fragments, defeating tabview/collapsible failure memoization (see mechanism 3).
3. **Per-collapsible full-source rescans plus center-body memoization disabled** (`src/preproc/compatibility.rs` `wikidot_outer_div_still_open_at_collapsible_close` + its three callers; `BlockFailureMemoization::wikidot_cache_context`). Each collapsible close check rebuilt `LiteralRegionIndex::new(source)` (full-source literal-range scan), copied the entire source with `to_ascii_lowercase()`, and linearly scanned lines from offset 0 to find the head line. Separately, `wikidot_cache_context` required `!parser.in_wikidot_center_body()`, so tabview/collapsible failures inside `[[=]]` were never memoized and large `[[tabview]]` bodies were re-parsed (notably `TabViewContainsNonTab` rejections). GDB sample (seq 934, 0766 after mechanisms 1-2 were fixed) showed `wikidot_outer_div_still_open_at_collapsible_close` at the top frame under `align_center::parse_fn`. Argon also argued `body_end_boundary_fingerprint` counted duplicate ancestors in `sum`/`xor`/`len` even though its own doc comment says "the ancestors' order and repeats do not change any boundary check," so `[[=]]` levels 2..30 produced distinct fingerprints and defeated body-walk failure sharing.

### What the uncommitted diff is intended to do

Nine files, +129/-81 (all Argon-authored, edits at seq 879/907/1143/1185/1340):

- `block_pairs.rs`: `tabview_measure_memo` key gains `nested_in_tab = tab_body_scan_depth > 0`; nested measures are now memoized (guarded by the existing `measure_guard_trips` check) instead of bypassed.
- `wikidot_state.rs`: `attempt_key` also clamps `scored_div_body_depth`/`center_body_depth` to 1; new `apply_cached_success` preserves the three live depths across cached-success replay.
- `parser.rs`: `apply_rule_attempt_success` uses `apply_cached_success`; tabview/collapsible failure caches keyed by `(owner, collapsible, center)`; new `BlockScanCache::literal_region_index` + `wikidot_collapsible_outer_div_memo` with a `Parser::wikidot_outer_div_still_open_at_collapsible_close` accessor.
- `rule.rs`: `wikidot_cache_context` no longer excludes `in_wikidot_center_body()`.
- `consume.rs` / `collapsible.rs` / `div.rs`: route through the memoized accessor instead of rebuilding `LiteralRegionIndex` per call; `div.rs` lowercases only the opener line before early-exiting, then the remainder.
- `compatibility.rs`: `rfind('\n')` for head-line start instead of linear scan; whole-source `lower` copy removed in favor of case-insensitive helpers directly on `source`.
- `body_end.rs`: fingerprint deduplicates distinct ancestor `mix` values (`unique` count replaces `ancestors.len()`).

### What Argon had already measured or tested

- seq 925 (mechanisms 1+2 built, seq 918): `scp-play-podcast-hub` 37.65 ms; `mc02528` **19.85 ms** (from 8,580 ms); `0464` **188 ms** (from >180 s timeout); `0766` **10.60 s** (from 32.49 s); `0818` 176 ms; `0692` 509 ms. Verdict in seq 934: "`mc02528` 8580 -> 19.85 ms, 432x."
- seq 1171 (mechanism 3 partial: fingerprint dedup + div early-exit + linear-to-`rfind` + `lower = source[..limit].to_ascii_lowercase()`): `0766` **5.39 s** (from 10.60 s), but that intermediate **introduced a new panic** on `0818`: `rc=101` at `compatibility.rs:337:34: end byte index 120576 is out of bounds for string of length 74680`. The truncated `lower` (length `limit`, not source length) was indexed with source-relative extents beyond `limit`.
- seq 1185 removed the truncated `lower` entirely (helpers already case-insensitive; `[`/`]` searches unaffected), built at seq 1194. **No benchmark follows seq 1185 in the log.**
- seq 1340 final edits (literal memo, outer-div memo, center-bit failure keys, center-memo enablement) were **never compiled or measured**: the run died at seq 1398 (`OUT_OF_CREDITS`) while re-reading import lines (seq 1369-1391), i.e. about to clean the now-unused `LiteralRegionIndex` imports. No `cargo test` run appears anywhere in the Defect 2 portion.

### Takeover verification of the on-disk state (before any code change)

- `cargo build --release --example render_html_jsonl` succeeds (2m07s); `cargo check` shows exactly three new unused-import warnings: `src/parsing/consume.rs:36`, `src/parsing/rule/impls/block/blocks/collapsible.rs:35`, `src/parsing/rule/impls/block/blocks/div.rs:38` (`LiteralRegionIndex` + `wikidot_outer_div_still_open_at_collapsible_close`, now routed through the parser accessor). Argon died before removing them.
- Same-method re-measurement on the uncommitted tree (release `render_html_jsonl`, single-thread): `mc02528` 19 ms; `0464` 189 ms; `0766` **1.91 s** (the seq-1340 memo edits land a further ~2.8x over seq 1171's 5.39 s); `0818` 167 ms; `0692` 468 ms; 4-case concat 6 ms; hub 36 ms. The seq-1171 panic is gone (validates the seq-1185 `lower` removal in the current tree).
- Remaining question: `0766` at 1.91 s for 185 KB is ~50x the hub's rate (55 KB in 36 ms); Stage 2's "proportionate" bar and a first-k-parts scaling check decide whether another mechanism remains (see Stage 4).

### Judgment (keep / fix / revert)

- **Keep** the tabview memo key change, the `attempt_key` clamps + `apply_cached_success`, the `wikidot_cache_context` center-memo enablement, the literal/outer-div memo plumbing, the `rfind` head-line fix, and the `lower` removal. Each is mechanism-level (no per-shape special-casing), and the measured progression 32.5 s / 10.6 s / 5.39 s / 1.91 s across the three diff layers tracks the three mechanisms.
- **Keep** the `body_end.rs` dedup: it makes the fingerprint match its documented set semantics ("order and repeats do not change any boundary check"). The fingerprint feeds body-walk failure keys (`parser.rs:3675,3784`); collapsing repeat-only distinctions can only merge keys that the documented semantics declare equivalent. Full-suite output equality is the backstop.
- **Keep** the `div.rs` early-exit (test the opener line only before paying for the full lowercase).
- **Fix** (do not revert): remove the three unused imports Argon left behind; they fail the `-D warnings` clippy gate. Nothing else in the diff is reverted.

---

## Stage 4: Defect 2 Root Cause, Fix, and Re-measurement (Takeover)

### Mechanism summary (four compounding causes, all verified in this run)

1. **Exponential tabview re-measure** (`block_pairs.rs::measure_tabview`). The memo was bypassed whenever `tab_body_scan_depth > 0`, so chains of nested `[[tabview]]` were re-measured 2^k times. Cost: the `mc02528` 8.5 s and `0464` >180 s cases. Fix: memoize under key `(head_index, limit, nested_in_tab)`, still guarded by `measure_guard_trips`.
2. **Body-depth corruption through the rule-attempt cache** (`wikidot_state.rs`). `attempt_key` clamped only `div_body_depth`, and cached-success replay restored a shallow `WikidotState` over a live deeper stack, flickering `in_wikidot_center_body()` inside nested `[[=]]` and defeating tabview/collapsible failure memoization. Fix: clamp `scored_div_body_depth`/`center_body_depth` too, and preserve the three live depths in `apply_cached_success`.
3. **Per-collapsible full-source rescans** (`compatibility.rs` + callers + `rule.rs`). Every collapsible-close check rebuilt `LiteralRegionIndex` (O(n)), copied the whole source (`to_ascii_lowercase`, O(n)), and linearly scanned lines from offset 0. Fix: one `literal_region_index` + one `wikidot_collapsible_outer_div_memo` on `BlockScanCache` with a parser accessor; `rfind('\n')` for the head line; whole-source `lower` removed (the marker/extent helpers are already case-insensitive, and `[[` search is unaffected); `div.rs` lowercases only the opener line before early-exiting. Relatedly, `wikidot_cache_context` no longer excludes `in_wikidot_center_body()`, so tabview/collapsible failures inside `[[=]]` are memoized under `(owner, collapsible, center)` keys. `body_end.rs`: the fingerprint deduplicates ancestor mixes, matching its documented set semantics.
4. **Per-image-attempt raw-escape rescan** (`image.rs` -> `wikidot_raw_escape::raw_escapes`), found by takeover GDB sampling (3 of 5 samples in `raw_escapes`, 182 `[[image` attempts x O(n) scan). Fix: one `raw_escapes_cache` on `BlockScanCache` with a parser accessor; same vector (same order, same ordinals), scanned once per parse. The attribute-value encoder (`parser.rs:1194`) still calls the pure function on its value slice, which is also the whole-document scan in the head-map path; no hot frame implicated it in the blowup cases, so it is left as-is and recorded here.

### Defect 2 Before / After (final head `0fb589e7c`, release `render_html_jsonl`, single-thread, same method as Stage 1)

| Case | Before (`origin/main`) | After (final head) |
| --- | --- | --- |
| `mc02528` (31,794 chars) | 8,580 ms (48.4 s under shard load) | **21.5 ms** |
| `badpages/sandbox-synth-0464.ftml` (190,834 chars) | >180,000 ms timeout | **240.7 ms** |
| `badpages/sandbox-synth-0766.barrier.ftml` (185,034 chars) | 32,492 ms (>45 s under load) | **1,494.6 ms** |
| `scoped-play-podcast-hub` (54,984 chars) | 38.20 ms | 43.2 ms |
| `badpages/sandbox-synth-0818.ftml` | PANIC | **163.9 ms** (Stage 2) |
| `badpages/sandbox-synth-0692.barrier.ftml` | PANIC | **501.3 ms** (Stage 2) |
| 4-case concat | PANIC | **8.1 ms** (Stage 2) |

`0766` at ~1.5 s for 185 KB is a pathological 114-fragment concatenation, not a real page; post-fix GDB sampling shows distributed retry work (URL scheme checks, paragraph seams, allocation) with no single full-source rescan dominating. No per-shape special-casing was added; all four fixes are mechanism-level memoization/caching.

### Final gates (final head)

- `cargo test --lib`: **1228 passed; 0 failed; 13 ignored** (83.76 s). This is the campaign gate (brief mandate 4) and it passes on the final head.
- `cargo test` (full, including integration): **stopped after 2h09m elapsed without a terminal signature; informational only, not a merge blocker.** Evidence: the integration binary (625 tests, including `super_linear_families` 512-unit budget cases run in debug mode on a loaded shared box) burned CPU at ~308% (5h54m CPU), but its `tail`-piped output gave no incremental progress. Per-thread `/proc` deltas over 10 s showed two `ftml-parse` worker threads each at 100% user CPU (1000/1000 ticks) while harness threads (`super_linear_fa`, `wikidot_unicode`) sat idle, i.e. two parallel super-linear budget tests in flight, consistent with progressing through the suite rather than one wedged test; ptrace attach was sandbox-blocked, so no stack sample was possible. No failures were observed: the lib suite (the spec-bearing suite for these changes) is fully green and all `badpages/` carriers parse in milliseconds. The run was stopped per the coordinator's scope correction (lib suite + re-measurements are the gates) rather than allowed to burn unbounded hours.
- `cargo build --release --example render_html_jsonl`: clean; no new warnings from touched files.
- `cargo fmt --check`: own hunks hand-formatted clean; 164 diffs remain elsewhere (pre-existing rustfmt-version drift, untouched per brief).
- `RUSTFLAGS='-D warnings' cargo clippy --tests --no-deps`: **fails identically on pre-existing lints** (~90 locations outside these hunks, including the same warnings Argon's builds show at seq 900/918 on the Defect-1 head; none of the error locations fall in the hunks above). No new clippy errors introduced. Unused-import warnings from the uncommitted diff were removed as part of this change.
