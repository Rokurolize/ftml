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

