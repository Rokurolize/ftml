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
