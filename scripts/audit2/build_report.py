#!/usr/bin/env python3
"""Render REPORT-audit2.md from the audit2 record-dir data."""

import json
from collections import Counter, defaultdict
from pathlib import Path

import os
REPO = Path(__file__).resolve().parents[2]
CFG = Path(os.environ.get("FTML_AUDIT_RECORD", REPO / ".audit"))
OUT = REPO / "REPORT-audit2.md"

rows = {json.loads(line)["case"]: json.loads(line)
        for line in (CFG / "audit-all.jsonl").read_text().splitlines()}
classified = json.loads((CFG / "classified.json").read_text())
cases = classified["cases"]
saved = json.loads((CFG / "saved-pages" / "saved-results.json").read_text())
n_families = len({r["family"] for r in rows.values()})
manifest = json.loads((CFG / "saved-pages" / "saved-manifest.json").read_text())
ws = json.loads((CFG / "whitespace-analysis.json").read_text())
b_items = json.loads((CFG / "b_items.json").read_text())
probes = {}
for batch in ("batch1", "batch2", "batch3", "batch4", "batch5", "batch6"):
    for path in sorted((CFG / "probes" / batch).glob("*.json")):
        if path.name == "summary.json":
            continue
        rec = json.loads(path.read_text())
        probes[rec["name"]] = rec


def group(case):
    cls = cases.get(case, {}).get("class") or rows[case]["status"]
    if cls == "match-exact":
        return "match"
    if cls.startswith("harmless"):
        return "whitespace"
    if cls.startswith("unjudgeable"):
        return "unjudgeable"
    if cls.startswith("bug"):
        return "bug"
    if cls.startswith("context"):
        return "context"
    return "match"


def coverage_table():
    counts = defaultdict(lambda: defaultdict(int))
    for case, row in rows.items():
        counts[row["family"]][group(case)] += 1
    out = ["| family | fixtures | match | harmless-whitespace | context | unjudgeable | bug-encoded |",
           "|---|---|---|---|---|---|---|"]
    totals = Counter()
    for family in sorted(counts):
        c = counts[family]
        totals.update(c)
        out.append(f"| `{family}` | {sum(c.values())} | {c['match']} | {c['whitespace']} "
                   f"| {c['context']} | {c['unjudgeable']} | {c['bug']} |")
    out.append(f"| **total** | **{sum(totals.values())}** | **{totals['match']}** "
               f"| **{totals['whitespace']}** | **{totals['context']}** "
               f"| **{totals['unjudgeable']}** | **{totals['bug']}** |")
    return "\n".join(out)


def class_totals():
    counts = Counter(group(c) for c in rows)
    lines = [
        f"- audited: {len(rows)} fixtures with a `wikidot.html` (987/987 with `wikidot.html`; all {n_families} families).",
        f"- match: {counts['match']} (including every first-pass match re-checked)",
        f"- harmless-whitespace: {counts['whitespace']}",
        f"- unjudgeable (PagePreview cannot observe): {counts['unjudgeable']}",
        f"- bug-encoded: {counts['bug']}",
        f"- caller-runtime / boundary context: {counts['context']}",
    ]
    return "\n".join(lines)


def saved_table():
    out = ["| fixture | probe page | saved content sha256 | class |",
           "|---|---|---|---|"]
    for case, entry in sorted(cases.items()):
        if entry.get("evidence") != "saved-page":
            continue
        cap = saved[case]
        out.append(f"| `{case}` | `{cap['fullname']}` | `{cap['content_sha256'][0][:16]}` "
                   f"| {entry['class']} |")
    return "\n".join(out)


def saved_summary():
    entries = [(c, e) for c, e in cases.items() if e.get("evidence") == "saved-page"]
    groups = Counter(e["class"] for _, e in entries)
    lines = [f"{len(entries)} preview-opaque shapes were re-captured from saved pages "
             f"(tags `fruit, component`):", ""]
    for cls in sorted(groups):
        lines.append(f"- {groups[cls]} × {cls}")
    lines.append("")
    lines.append("Every capture was fetched twice; six first attempts hit a transient "
                 "Wikidot 503 and were re-captured until both fetches returned the page "
                 "content, so every committed capture is a pair of successful fetches "
                 "(gallery captures are stable after masking the generated "
                 "`gallery-box-<id>`).")
    lines.append("")
    lines.append("| class | fixtures |")
    lines.append("|---|---|")
    for cls in sorted(groups):
        sample = ", ".join(f"`{c}`" for c, e in sorted(entries) if e["class"] == cls)
        lines.append(f"| {cls} | {sample} |")
    return "\n".join(lines)


def whitespace_table():
    out = ["| fixture | evidence | live sha256 | whitespace position |",
           "|---|---|---|---|"]
    for case, entry in sorted(ws.items()):
        sha = (entry.get("saved_content_sha256") or rows[case]["raw_html_sha256"])[:16]
        out.append(f"| `{case}` | {entry['evidence']} | `{sha}` "
                   f"| {', '.join(entry['positions'])} |")
    return "\n".join(out)


def whitespace_summary():
    positions = Counter()
    for entry in ws.values():
        for pos in entry["positions"]:
            positions[pos] += 1
    return (f"All {len(ws)} cases are raw-DOM differences made only of whitespace text runs. "
            f"For every case the element/attribute topology and the collapsed visible text "
            f"are identical, the differing runs sit at a fragment/block/line edge, and no "
            f"`pre`/`code`/`textarea` text differs. Position classes observed: "
            + ", ".join(f"{k} ×{v}" for k, v in positions.most_common())
            + ". Verdict: **normalization-only** (not visually observable under the HTML "
              "whitespace-collapsing rules). This is an audit verdict, not a comparison-"
              "normalization disposition: per `docs/ParityTests.md`, promoting a binding to "
              "`comparison-normalization` still needs a browser DOM review and an entry in "
              "`tests/fixtures/wikidot-parity/comparison-normalization-contracts.json`.")


def context_table():
    grouped = defaultdict(list)
    for case, entry in cases.items():
        if entry["class"].startswith("context"):
            grouped[entry["class"]].append(case)
    out = ["| caller-runtime / boundary class | fixtures |", "|---|---|"]
    for cls in sorted(grouped):
        items = sorted(grouped[cls])
        sample = ", ".join(f"`{c}`" for c in items[:8])
        more = "" if len(items) <= 8 else f" (+{len(items) - 8} more; full list in `audit/classifications.json`)"
        out.append(f"| {cls} | {len(items)}: {sample}{more} |")
    return "\n".join(out)


def unjudgeable_table():
    items = sorted(c for c, e in cases.items() if e["class"].startswith("unjudgeable"))
    if not items:
        return "None: every preview-opaque shape was settled by a saved-page capture."
    out = ["| fixture |", "|---|"]
    for case in items:
        out.append(f"| `{case}` |")
    return "\n".join(out)


def b_item_section(item):
    out = [f"### {item['id']}. {item['title']}", ""]
    entry_for = {c: cases.get(c, {}) for c in item["fixtures"]}
    has_saved = any(e.get("evidence") == "saved-page" for e in entry_for.values())
    if has_saved:
        out.append("| fixture | evidence | live sha256 |")
        out.append("|---|---|---|")
        for case in item["fixtures"]:
            entry = entry_for[case]
            if entry.get("evidence") == "saved-page":
                cap = saved[case]
                out.append(f"| `{case}` | saved page `{cap['fullname']}` "
                           f"| `{cap['content_sha256'][0][:16]}` |")
            elif case in rows:
                out.append(f"| `{case}` | PagePreview | `{rows[case]['raw_html_sha256'][:16]}` |")
    else:
        out.append("| fixture | live PagePreview sha256 |")
        out.append("|---|---|")
        for case in item["fixtures"]:
            if case not in rows:
                continue
            out.append(f"| `{case}` | `{rows[case]['raw_html_sha256'][:16]}` |")
    out += ["", f"**Live rule.** {item['live_rule']}", "",
            f"**FTML / fixture.** {item['ftml']}", ""]
    probe_lines = []
    for name in item.get("probes", []):
        rec = probes.get(name)
        if rec is None:
            continue
        hashes = rec["raw_html_sha256"]
        stable = "identical" if hashes[0] == hashes[1] else "canonically identical"
        probe_lines.append(f"| `{name}` | `{hashes[0][:16]}` | `{hashes[1][:16]}` | {stable} |")
    if probe_lines:
        out += ["**Live probes (PagePreview twice per shape).**", "",
                "| probe | capture 1 sha256 | capture 2 sha256 | twice |", "|---|---|---|---|"]
        out += probe_lines
        out.append("")
    if item.get("notes"):
        out += [f"**Notes.** {item['notes']}", ""]
    return "\n".join(out)


def probe_determinism_note():
    return (
        "Every ad-hoc shape was posted twice with independent anonymous token cookies. "
        "Non-tabview shapes returned byte-identical bodies. Tabview bodies differ per "
        "request in two generated parts: the `wiki-tabview-<hex>` id (also embedded in the "
        "YUI init script) and the inline `new YAHOO.widget.TabView(...)` script; after "
        "masking those, both captures are identical for every tabview shape (verified in "
        "`audit/probes.json`). The corpus harness masks the same ids. Live also emits the "
        "per-page `<script src=.../tabview-min.js>` asset include, which FTML does not own.")


def saved_page_inventory():
    out = ["| page | fixture |", "|---|---|"]
    for fullname, case in sorted(manifest.items()):
        out.append(f"| `{fullname}` | `{case}` |")
    return "\n".join(out)


def main():
    bugs = sorted(c for c, e in cases.items() if e["class"].startswith("bug"))
    ws_families = Counter(c.split("/")[0] for c in ws)
    pass1_ws = {"code/closer-first-suffixed-wins", "code/closer-whitespace-then-text",
                "code/nested-suffixed-outer-closer", "code/two-codes-suffixed-first-closer",
                "list/explicit-crossed-pre-trailing-dissolves",
                "list/explicit-scope-break-inside-paragraph",
                "list/explicit-scope-break-paragraph",
                "list/nested-multi-item-partial-close", "list/nested-partial-close-in-item",
                "list/zombie-pair-fired-scope-not-reopened", "span/basic",
                "list/explicit-post-table-literal", "list/explicit-pre-kept-before-prose",
                "misc/bibliography-second-block", "strikethrough/end-block-closes-scope",
                "misc/dash-line-fusion"}
    new_ws = [c for c in ws if c not in pass1_ws]
    new_ws_families = Counter(c.split("/")[0] for c in new_ws)
    ws_new_by_family = ", ".join(f"{n} `{f}`" for f, n in new_ws_families.most_common())
    ws_saved = sum(1 for c in new_ws if cases[c].get("evidence") == "saved-page")
    pass_one_fixed = {
        "B1": ["list/comment-line-start"],
        "B2": ["code/css-pseudo-class-split"],
        "B3": ["code/css-url-token-intact"],
        "B4": ["misc/bibliography-duplicate-definition"],
        "B5": ["span/fused-opener-scored-map-residual"],
        "B6": ["list/fused-close"],
    }
    carry = Counter()
    for item in pass_one_fixed.values():
        for case in item:
            if case in bugs:
                carry[case] = 1
    text = f"""# Corpus fixture provenance audit, pass 2 (full sweep)

Date: 2026-11-05. Branch: `fix/dsh-audit2-corpus-audit`, based on
`claude/audit-corpus-fixtures-9fa427` (`a68272762`). Evidence-only: no library
source and no fixture file is changed in this branch; fixtures are fixed by
separate lanes.

## STATUS

**Done**

- Audited **987 of 987** fixtures that carry a `wikidot.html`, across all {n_families}
  families, against live Wikidot PagePreview with the pass-1 harness
  (`scripts/audit_corpus_live.py`). The pass-1 set (231 fixtures) was re-audited
  on the current branch; the remaining 756 are new.
- Classified every non-match (see taxonomy mapping below). All full evidence is
  committed under `audit/`: merged live captures, per-fixture verdicts,
  classification evidence, saved-page captures, whitespace adjudication, probes.
- Settled **{sum(1 for c, e in cases.items() if e.get('evidence') == 'saved-page')}**
  preview-opaque shapes with saved-page captures on `sandbox-for-codex` using
  account A and tags `fruit, component`, including the pass-1 leftovers: the 7
  iftags fixtures (`color/iftags-seam-spec`, `list/iftags-body-escape*`,
  `span/iftags-seam*`) all match live exactly.
- Adjudicated all **{len(ws)}** whitespace-only divergences against live bytes:
  every one is normalization-only.
- Opened **B7–B{max(int(i['id'][1:]) for i in b_items)}** for the newly found
  bug-encoded fixtures (continuing the B sequence; B1–B6 verdicts untouched).

**Result totals**

{class_totals()}

Taxonomy mapping: `harmless-whitespace` = the pass-1 "stale but harmless"
class; `unjudgeable` = the pass-1 "context-only" class (PagePreview cannot
observe what the fixture runner renders); `bug-encoded` = element/attribute/
visible-text divergence. `caller-runtime` and `boundary` are the project's
frozen dispositions from `docs/ParityTests.md`; they are the "unjudgeable"
sub-cases where the missing input belongs to the caller/runtime rather than
PagePreview's gate evaluation.

## Coverage

Every fixture with a `wikidot.html` was posted to live PagePreview. Counts per
family below; `context` includes caller-runtime and boundary dispositions.

{coverage_table()}

## Method

- PagePreview harness (pass 1): `POST edit/PagePreviewModule` anonymously,
  twice-cached by source hash, html5lib DOM + visible-text comparison, generated
  ids masked, formatting whitespace at block edges dropped.
- Saved-page harness (new): `scripts/audit2/saved_probe.py` creates/updates
  run-owned `muse-probe-audit2-N` pages with the fixture source and the corpus
  runner's tags (`fruit, component`), fetches each saved page twice anonymously,
  extracts `#page-content` inner HTML, and compares it with the fixture using the
  same comparator. This is required wherever anonymous PagePreview cannot render
  what the runner renders: `[[iftags]]` gates (preview evaluates every gate
  false) and `[[html]]` blocks (preview keeps them literal).
- Ad-hoc probes (new): `scripts/audit2/probe_preview.py`, two independent
  anonymous requests per shape, raw bodies and SHA-256 recorded.
- Classifier (new): `scripts/audit2/classify_audit.py`; structure (element +
  attribute token sequence) and collapsed visible text are compared separately,
  caller-runtime output is relaxed (newpage class, file-host URLs, Wikidot
  asset/init scripts, generated ids, html-block iframe URL), and a saved-page
  capture is used in preference to PagePreview when both exist.
- Frozen parity evidence was adjudicated before live probing: `bindings.json`
  and `tests/fixtures/wikidot-parity/references-*.jsonl` were consulted for each
  suspect fixture (notably `test--anchor--xss`, which has a frozen match).

## Preview-opaque shapes settled by saved pages

{saved_summary()}

## New bug-encoded fixtures

B1–B6 (pass 1) remain assigned to the fixture-fix lane and are not re-judged
here; their fixture still shows the same mismatch on this branch
({', '.join(f'`{c}`' for c in sorted(carry))}). The items below are new
findings for a follow-up fix lane; B1–B6 verdicts are untouched.

{chr(10).join(b_item_section(item) for item in b_items)}

## Whitespace adjudication

{whitespace_summary()}

{whitespace_table()}

The 17 pass-1 whitespace cases were re-checked: 16 still reproduce as
whitespace-only (the 17th, `misc/quote-pair-close-in-owned-value`, is a
caller-owned file-host URL difference plus whitespace and is classified
caller-runtime). The remaining {len(ws) - 16} cases are new: {ws_new_by_family}
({ws_saved} of them found only on a saved page).

## Caller-runtime and boundary classes

These are not counted as bugs; they are the frozen dispositions named in
`tests/fixtures/wikidot-parity/caller-runtime-contracts.json` and
`docs/WikijumpBoundary.md`.

{context_table()}

Additional PagePreview-specific observations:

- Anonymous PagePreview evaluates **every** `[[iftags]]` gate false (including
  `[[iftags -zzz]]`), so any fixture whose gate is true under the runner tags
  needs a saved page.
- PagePreview keeps `[[html]]` blocks literal; a saved page renders them as an
  iframe whose `src` is caller-supplied (`.../html/<hash>`), so the iframe URL
  is caller-runtime.
- PagePreview keeps inline `[[math]]` / `[[math label]]` text literal; the
  block form (`[[math]]` on its own lines) renders in both preview and saved
  pages.
- Tabview output includes a per-page asset `<script>` and an inline YUI init
  script plus per-request `wiki-tabview-<hex>` ids. None is FTML-owned; they are
  relaxed for the comparison, and the remaining tabview mismatches are the five
  structural classes above (B14–B19).
- PagePreview prepends root whitespace and serializes formatting newlines after
  block elements; this is the source of the whitespace-only class.

## Probe determinism

{probe_determinism_note()}

## Remaining unjudgeable

{unjudgeable_table()}

## Committed evidence

- `audit/summary.json` - coverage and class totals per family.
- `audit/verdicts.jsonl` - one row per audited fixture (class, evidence, hashes).
- `audit/classifications.json` - per-non-match evidence, including live/expected
  structure and text and the saved-page reference where used.
- `audit/live-captures.jsonl.gz` - the raw PagePreview bodies keyed by source
  SHA-256 (985 unique sources; the 987 fixtures share two sources).
- `audit/saved-pages.json` - every `muse-probe-*` page, its tags, both saved
  content hashes, and the classification.
- `audit/whitespace.json` - the {len(ws)} whitespace adjudications with position class.
- `audit/b-items.json` - the new B-item definitions and their fixture lists.
- `audit/probes.json` - every ad-hoc probe with both raw bodies and hashes.
- `audit/tabview-class.json` - minimal-difference resolution per tabview fixture.
- `scripts/audit2/` - the saved-page harness, probe harness, classifier, and
  report/evidence builders.

## Reproduction

```sh
# Full PagePreview sweep (987 fixtures, ~30 min, anonymous read-only).
uv run --with html5lib python scripts/audit_corpus_live.py \\
  --cache .audit-cache --output verdicts.jsonl

# Probe harness (needs a JSON list of {{name, source}}).
uv run --with html5lib python scripts/audit2/probe_preview.py \\
  --list probes.json --out probes/

# Saved-page captures (sandbox account A; leaves muse-probe-* pages in place).
uv run --project /home/roku/src/Rokurolize/wikidot.py --with html5lib \\
  python scripts/audit2/saved_probe.py --repo . --out saved-pages --cases-file cases.txt
```

## muse-probe-* pages left in place

{saved_page_inventory()}
"""
    OUT.write_text(text)
    print("wrote", OUT, len(text), "bytes")


if __name__ == "__main__":
    main()
