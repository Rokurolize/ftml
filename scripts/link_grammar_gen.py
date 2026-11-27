#!/usr/bin/env python3
"""Link grammar derivation & bounded Cartesian differential generator (Track T5).

Extracts character classes and states from:
  - PHP reference: Freelink.php, Url.php (Parse + Render/Xhtml), Anchor.php
  - FTML: link_single.rs, link_triple.rs, link_anchor.rs, anchor.rs, url.rs

Generates a bounded Cartesian case space over:
  (opener, first_host_byte, continuation, delimiter_run,
   separator, label_terminator, trailing_punct, close_form)

and runs local differential comparison against:
  1. PHP reference (batch-census.php in `tokens` and `render` modes)
  2. FTML (`render_html_jsonl` in `wikidot` layout)
"""

from __future__ import annotations

import argparse
import html as html_lib
import itertools
import json
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Extracted Link Grammar Character Classes & States
# ---------------------------------------------------------------------------

SCHEMES_ALL = [
    "http://",
    "https://",
    "ftp://",
    "gopher://",
    "news://",
    "mailto:",
    "mms://",
    "ssh://",
    "git://",
    "sftp://",
    "svn://",
    "telnet://",
    "cvs://",
]

# Representative schemes covering standard, Wikidot-only, and non-slash schemes
SCHEMES_REP = ["http://", "ssh://", "mailto:"]

# First host byte classes (findings 854, 857, 858, 859)
FIRST_HOST_BYTES: dict[str, tuple[str, str]] = {
    "alnum": ("a", "ASCII alphanumeric baseline"),
    "lbracket": ("[", "Leading delimiter [ (finding 857)"),
    "rbracket": ("]", "Leading delimiter ] (finding 857)"),
    "pipe": ("|", "Leading delimiter | (finding 857)"),
    "nbsp": ("\u00a0", "Unicode WS NBSP (finding 858)"),
    "nel": ("\u0085", "Unicode WS NEL (finding 858)"),
    "ideosp": ("\u3000", "Unicode WS U+3000 (finding 858)"),
    "nul": ("\x00", "C0 NUL (finding 859, save-inadmissible)"),
    "soh": ("\x01", "C0 SOH (finding 859, save-inadmissible)"),
    "vt": ("\x0b", "C0 VT (finding 858/859, save-inadmissible)"),
    "ff": ("\x0c", "C0 FF (finding 858/859, save-inadmissible)"),
    "us": ("\x1f", "C0 US (finding 859, save-inadmissible)"),
    "tab": ("\t", "ASCII TAB (control boundary)"),
    "esc": ("\x1b", "C0 ESC (control boundary)"),
    "del": ("\x7f", "ASCII DEL (control boundary)"),
    "dquote": ('"', "Double quote (finding 854)"),
    "squote": ("'", "Single quote"),
    "space": (" ", "ASCII space"),
}

# Continuation classes
CONTINUATIONS: dict[str, str] = {
    "empty": "",
    "word": "b",
    "dquote_mid": 'b"c',
    "squote_mid": "b'c",
}

# Delimiter runs (finding 855 and friends)
DELIMITER_RUNS: dict[str, str] = {
    "none": "",
    "dp_word": "||c",      # doubled pipe + word char (finding 855)
    "dp_dash": "||-",      # doubled pipe + non-continuing char (finding 855 bound)
    "dp_end": "||",        # trailing doubled pipe
    "sp_word": "|c",       # single pipe + word char
    "sp_bs": "|\\c",       # single pipe + backslash + word char
    "math_open": "[[",     # inline math/block opener
}

# Separator + label pairs
SEPARATORS_LABELS: dict[str, tuple[str, str]] = {
    "none": ("", ""),
    "sp_lbl": (" ", "label"),
    "tab_lbl": ("\t", "label"),
    "nbsp_lbl": ("\u00a0", "label"),
    "pipe_lbl": ("|", "label"),
    "pipe_empty": ("|", ""),
    "sp_dollar_lbl": (" ", "label$"),
}

# Trailing punctuation (finding 862 + sentence boundary)
TRAILING_PUNCT: dict[str, str] = {
    "none": "",
    "dot": ".",
    "dollar": "$",         # finding 862 ([http://a$])
    "dquote": '"',
    "rbracket": "]",
}

# Close forms
CLOSE_FORMS: dict[str, str] = {
    "eof": "",
    "single": "]",
    "double": "]]",
    "triple": "]]]",
}

# Openers for scheme URLs
SCHEME_OPENERS: dict[str, str] = {
    "bare": "",
    "star_bare": "*",
    "single": "[",
    "star_single": "[*",
    "triple": "[[[",
    "star_triple": "[[[*",
}

# C0 bytes that are save-inadmissible on live Wikidot (cannot be saved via browser/XML-RPC)
SAVE_INADMISSIBLE_CHARS = set(
    [chr(c) for c in range(0x00, 0x20) if c not in (0x09, 0x0A, 0x0D)] + ["\x7f"]
)


def is_save_inadmissible(source: str) -> bool:
    return any(ch in SAVE_INADMISSIBLE_CHARS for ch in source)


def tag_findings(source: str, family: str, axes: dict[str, str]) -> list[str]:
    """Tag rows that correspond to known ARTEX Task 31 findings 854-859, 862."""
    tags: list[str] = []
    if '"' in source and family == "scheme_url":
        tags.append("854")
    if "||" in source:
        tags.append("855")
    if family == "anchor_link" and axes.get("separator") == "nbsp":
        tags.append("856")
    if axes.get("first_host") in ("lbracket", "rbracket", "pipe"):
        tags.append("857")
    if axes.get("first_host") in ("nbsp", "nel", "ideosp", "vt", "ff"):
        tags.append("858")
    if axes.get("first_host") in ("nul", "soh", "vt", "ff", "us"):
        tags.append("859")
    if axes.get("trailing") == "dollar" or axes.get("label", "").endswith("$"):
        tags.append("862")
    return tags


def generate_cases() -> list[dict[str, Any]]:
    """Generate the bounded Cartesian case space across all link families."""
    cases: list[dict[str, Any]] = []
    seen_sources: set[str] = set()
    counter = 0

    def add_case(family: str, slice_name: str, axes: dict[str, str], source: str) -> None:
        nonlocal counter
        if source in seen_sources:
            return
        seen_sources.add(source)
        counter += 1
        cases.append({
            "case_id": f"t5-{counter:04d}",
            "family": family,
            "slice": slice_name,
            "axes": axes,
            "findings": tag_findings(source, family, axes),
            "save_inadmissible": is_save_inadmissible(source),
            "source": source,
        })

    # -----------------------------------------------------------------------
    # Slice 1: Scheme URL x (opener, scheme, first_host_byte, continuation, close_form)
    # Covers findings 854, 857, 858, 859 and control/delimiter boundaries.
    # -----------------------------------------------------------------------
    matching_closes = {
        "bare": ["eof"],
        "star_bare": ["eof"],
        "single": ["single", "double"],
        "star_single": ["single", "double"],
        "triple": ["triple"],
        "star_triple": ["triple"],
    }
    for op_k, sch, fh_k, cont_k in itertools.product(
        SCHEME_OPENERS.keys(),
        ["http://", "ssh://"],
        FIRST_HOST_BYTES.keys(),
        ["empty", "word"],
    ):
        for cl_k in matching_closes[op_k]:
            op = SCHEME_OPENERS[op_k]
            fh = FIRST_HOST_BYTES[fh_k][0]
            cont = CONTINUATIONS[cont_k]
            cl = CLOSE_FORMS[cl_k]
            src = f"{op}{sch}{fh}{cont}{cl}"
            add_case(
                "scheme_url",
                "first_host_gate",
                {
                    "opener": op_k,
                    "scheme": sch,
                    "first_host": fh_k,
                    "continuation": cont_k,
                    "close": cl_k,
                },
                src,
            )

    # -----------------------------------------------------------------------
    # Slice 2: Scheme URL x (opener, continuation, delimiter_run, separator_label, close_form)
    # Covers findings 854 (quotes), 855 (doubled pipes), separators, and close forms.
    # -----------------------------------------------------------------------
    for op_k, cont_k, del_k, sep_k in itertools.product(
        ["bare", "single", "star_single", "triple"],
        ["word", "dquote_mid", "squote_mid"],
        DELIMITER_RUNS.keys(),
        ["none", "sp_lbl", "tab_lbl", "nbsp_lbl", "pipe_lbl"],
    ):
        # Skip invalid/redundant combinations: bare URLs don't take bracket labels
        if op_k == "bare" and sep_k != "none":
            continue
        if op_k in ("single", "star_single") and sep_k == "pipe_lbl":
            continue
        if op_k == "triple" and sep_k in ("sp_lbl", "tab_lbl", "nbsp_lbl"):
            continue
        cl_k = "eof" if op_k == "bare" else ("triple" if op_k == "triple" else "single")
        op = SCHEME_OPENERS[op_k]
        cont = CONTINUATIONS[cont_k]
        delim = DELIMITER_RUNS[del_k]
        sep, lbl = SEPARATORS_LABELS[sep_k]
        cl = CLOSE_FORMS[cl_k]
        src = f"{op}http://a{cont}{delim}{sep}{lbl}{cl}"
        add_case(
            "scheme_url",
            "delim_and_sep",
            {
                "opener": op_k,
                "scheme": "http://",
                "first_host": "alnum",
                "continuation": cont_k,
                "delimiter": del_k,
                "separator": sep_k,
                "close": cl_k,
            },
            src,
        )

    # -----------------------------------------------------------------------
    # Slice 3: Scheme URL trailing punctuation & close-form interactions (finding 862)
    # -----------------------------------------------------------------------
    for op_k, sch, tr_k, sep_k, cl_k in itertools.product(
        ["bare", "single", "star_single", "triple"],
        ["http://", "mailto:"],
        TRAILING_PUNCT.keys(),
        ["none", "sp_lbl", "pipe_lbl", "pipe_empty", "sp_dollar_lbl"],
        ["eof", "single", "double", "triple"],
    ):
        if op_k == "bare" and cl_k != "eof":
            continue
        if op_k != "bare" and cl_k == "eof":
            continue
        if op_k == "bare" and sep_k != "none":
            continue
        if op_k in ("single", "star_single") and sep_k in ("pipe_lbl", "pipe_empty"):
            continue
        if op_k == "triple" and sep_k in ("sp_lbl", "sp_dollar_lbl"):
            continue
        op = SCHEME_OPENERS[op_k]
        tr = TRAILING_PUNCT[tr_k]
        sep, lbl = SEPARATORS_LABELS[sep_k]
        cl = CLOSE_FORMS[cl_k]
        host = "a" if sch == "http://" else "a@b.c"
        src = f"{op}{sch}{host}{tr}{sep}{lbl}{cl}"
        add_case(
            "scheme_url",
            "trailing_punct",
            {
                "opener": op_k,
                "scheme": sch,
                "trailing": tr_k,
                "separator": sep_k,
                "label": lbl,
                "close": cl_k,
            },
            src,
        )

    # -----------------------------------------------------------------------
    # Slice 4: Local anchor links [#...], [*#...], [[#...]], [[[#...]]]
    # Covers findings 856 (NBSP separator) and 862 (terminal $ in label).
    # -----------------------------------------------------------------------
    anchor_openers = {"single": "[#", "star_single": "[*#", "double": "[[#", "triple": "[[[#"}
    anchor_targets = {"valid": "a", "chars": "a-b_c%20", "dot": "a.b", "empty": "", "quote": 'a"b'}
    anchor_seps = {"space": " ", "tab": "\t", "nbsp": "\u00a0", "emsp": "\u2003", "none": ""}
    anchor_labels = {"empty": "", "lbl": "label", "lbl_dollar": "label$"}
    anchor_closes = {"single": "]", "double": "]]", "triple": "]]]"}

    for op_k, tgt_k, sep_k, lbl_k, cl_k in itertools.product(
        anchor_openers.keys(),
        anchor_targets.keys(),
        anchor_seps.keys(),
        anchor_labels.keys(),
        anchor_closes.keys(),
    ):
        # Bound: only test double/triple close on valid/empty target or dollar label
        if cl_k == "triple" and op_k != "triple":
            continue
        if op_k == "triple" and cl_k != "triple":
            continue
        if tgt_k == "quote" and sep_k not in ("space", "none"):
            continue
        if sep_k == "emsp" and lbl_k != "lbl":
            continue
        op = anchor_openers[op_k]
        tgt = anchor_targets[tgt_k]
        sep = anchor_seps[sep_k]
        lbl = anchor_labels[lbl_k]
        cl = anchor_closes[cl_k]
        src = f"{op}{tgt}{sep}{lbl}{cl}"
        add_case(
            "anchor_link",
            "anchor_cartesian",
            {
                "opener": op_k,
                "target": tgt_k,
                "separator": sep_k,
                "label": lbl,
                "close": cl_k,
            },
            src,
        )

    # -----------------------------------------------------------------------
    # Slice 5: Relative /path, interwiki, and triple-bracket freelinks
    # -----------------------------------------------------------------------
    rel_targets = [
        "/foo",
        '/foo"bar',
        "/foo'bar",
        "/foo]]bar",
        "wikipedia:Page",
        "WIKIPEDIA:Page",
        "foo:bar",
        "javascript:void(0)",
    ]
    for op_k, tgt, sep_k, cl_k in itertools.product(
        ["single", "star_single"],
        rel_targets,
        ["none", "sp_lbl", "tab_lbl", "nbsp_lbl"],
        ["single", "double"],
    ):
        op = SCHEME_OPENERS[op_k]
        sep, lbl = SEPARATORS_LABELS[sep_k]
        cl = CLOSE_FORMS[cl_k]
        src = f"{op}{tgt}{sep}{lbl}{cl}"
        add_case(
            "relative_or_interwiki",
            "rel_interwiki",
            {"opener": op_k, "target": tgt, "separator": sep_k, "close": cl_k},
            src,
        )

    freelink_targets = [
        "page",
        "_default:page",
        " _default:page",
        "\u001b_default:page",
        "\u200b_default:page",
        "cat:page",
        "#a",
        " #a",
        "a#b",
        "a##b",
        "a#b#c",
        "foo/#/bar",
        "foo/#/bar#baz",
        "/path",
        "*http://example.com",
    ]
    for op_k, tgt, lbl_k in itertools.product(
        ["triple", "star_triple"],
        freelink_targets,
        ["none", "pipe_lbl", "pipe_empty"],
    ):
        op = SCHEME_OPENERS[op_k]
        lbl = "" if lbl_k == "none" else ("|label" if lbl_k == "pipe_lbl" else "|")
        src = f"{op}{tgt}{lbl}]]]"
        add_case(
            "freelink",
            "freelink_cartesian",
            {"opener": op_k, "target": tgt, "label": lbl_k},
            src,
        )

    return cases


# ---------------------------------------------------------------------------
# Local Differential Runners
# ---------------------------------------------------------------------------

def run_php_ref(cases: list[dict[str, Any]], refclone_dir: Path, mode: str) -> dict[str, Any]:
    """Run batch-census.php in `render` or `tokens` mode."""
    driver = refclone_dir / "harness" / "batch-census.php"
    stdin_data = "".join(
        json.dumps({"id": c["case_id"], "source": c["source"]}) + "\n"
        for c in cases
    )
    proc = subprocess.run(
        ["php", str(driver), mode],
        input=stdin_data,
        text=True,
        capture_output=True,
        check=True,
    )
    results: dict[str, Any] = {}
    for line in proc.stdout.split("\n"):
        if not line.strip():
            continue
        row = json.loads(line)
        results[row["id"]] = row
    return results


def run_ftml(cases: list[dict[str, Any]], ftml_bin: Path) -> dict[str, Any]:
    """Run FTML render_html_jsonl in `wikidot` layout."""
    stdin_data = "".join(
        json.dumps(
            {
                "schema": "wikijump_syntax_differential.syntax_case.v1",
                "case_id": c["case_id"],
                "source": c["source"],
                "title": "Test",
                "layout": "wikidot",
            },
        )
        + "\n"
        for c in cases
    )
    proc = subprocess.run(
        [str(ftml_bin)],
        input=stdin_data,
        text=True,
        capture_output=True,
        check=True,
    )
    results: dict[str, Any] = {}
    for line in proc.stdout.split("\n"):
        if not line.strip():
            continue
        row = json.loads(line)
        results[row["case_id"]] = row
    return results


def normalize_php_html(html: str | None) -> str:
    if html is None:
        return ""
    # Text_Wiki Xhtml adds trailing \n\n and sometimes wraps in <p>...</p>
    s = html.rstrip("\n")
    return s


def normalize_ftml_html(html: str | None) -> str:
    if html is None:
        return ""
    return html.rstrip("\n")


def normalize_benign(s: str) -> str:
    """Normalize benign HTML representation differences (entities, newpage class)."""
    s = s.replace("\xa0", "&#160;").replace("&#39;", "'")
    s = s.replace(' class="newpage"', "")
    s = s.replace('<a target="_blank" href="', '<a href="')
    return s


def adjudicate_row(
    case: dict[str, Any], r_html: str, f_html: str
) -> tuple[str, str, str]:
    """Return (adjudication, mechanism, evidence) for a differential row."""
    if r_html == f_html:
        return ("pinned_equal", "exact_html_match", "ref_render == ftml_render")

    if case["save_inadmissible"]:
        return (
            "out_of_scope_save_inadmissible",
            "c0_control_unsaveable",
            "STRATEGY-LIVE-ACCESS.md R14; live-index finding 859",
        )

    rh_norm = normalize_benign(r_html)
    fh_norm = normalize_benign(f_html)
    if rh_norm == fh_norm or (
        'target="_blank"' in r_html
        and 'target="_blank"' in f_html
        and r_html.replace(' target="_blank"', "")
        == f_html.replace(' target="_blank"', "")
    ):
        return (
            "pinned_benign_format",
            "entity_or_attr_order_or_newpage_class",
            "NBSP &#160; vs UTF-8, &#39; vs ', class=\"newpage\", or attr order",
        )

    src = case["source"]
    if "\t" in src and ("   " in rh_norm and "   " not in fh_norm):
        return (
            "known_separate_family_18081",
            "tab_separator_expansion",
            "live-index anchor/single-bracket-nbsp-separator notes (#18081)",
        )

    if src.startswith("[javascript:void(0)"):
        return (
            "intentional_security_sanitization",
            "dangerous_scheme_stripped",
            "ftml link_single.rs:1739 wikidot_layout_does_not_weaken_dangerous_single_link_sanitization",
        )

    if "$" in src:
        return (
            "known_fixed_in_flight_862",
            "terminal_dollar_trim_or_label",
            "live-index anchor/double-bracket-label-terminal-dollar; review-loop-state.md:474 (wK3)",
        )

    if "ssh://" in src and ("<a " in fh_norm) != ("<a " in rh_norm):
        return (
            "ftml_bug_candidate",
            "wikidot_only_scheme_empty_host",
            "ftml url.rs:57-72 starts_wikidot_only_scheme_url accepts empty-host delimiter/WS",
        )

    if "||" in src and "<a " in fh_norm and "<a " in rh_norm:
        return (
            "ftml_bug_candidate",
            "double_pipe_bracket_fallback_truncation",
            "ftml url.rs:400-406,644 bracket_fallback only set after ]]",
        )

    if "|\\" in src and "|/" in fh_norm:
        return (
            "ftml_bug_candidate",
            "pipe_backslash_href_slash",
            "ftml normalizes backslash to / in href whereas PHP ref keeps \\",
        )

    if src == "[*http://a.]]]":
        return (
            "ftml_bug_candidate",
            "sentence_boundary_right_link",
            "ftml url.rs:465-472 at_sentence_boundary misses Token::RightLink",
        )

    if src == "[wikipedia:Page]]":
        return (
            "ftml_bug_candidate",
            "unlabeled_interwiki_right_block",
            "ftml link_single.rs:429 closes=[RightBracket] misses RightBlock",
        )

    if src == "[[[*/path]]]":
        return (
            "ftml_bug_candidate",
            "freelink_star_path_label",
            "Freelink.php:195 strips * from no-label /path display text",
        )

    return (
        "ref_or_undecided_bracket_interior",
        f"bracket_interior_or_space_{case['family']}",
        "Freelink.php:83 / Anchor.php vs ftml link_triple.rs / link_anchor.rs",
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="T5 Link Grammar Generator & Diff Runner")
    sub = parser.add_subparsers(dest="cmd", required=True)

    p_gen = sub.add_parser("generate", help="Generate bounded Cartesian cases JSONL")
    p_gen.add_argument("-o", "--output", required=True, help="Output JSONL path")

    p_diff = sub.add_parser("run-diff", help="Generate and run local differential comparison")
    p_diff.add_argument("--refclone", required=True, help="Path to refclone")
    p_diff.add_argument("--ftml-bin", required=True, help="Path to render_html_jsonl binary")
    p_diff.add_argument("-o", "--output", required=True, help="Output corpus JSONL path")
    p_diff.add_argument("--summary", required=True, help="Output summary markdown path")

    args = parser.parse_args()

    cases = generate_cases()
    if args.cmd == "generate":
        out_path = Path(args.output)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        with out_path.open("w", encoding="utf-8") as f:
            for c in cases:
                f.write(json.dumps(c) + "\n")
        print(f"Generated {len(cases)} cases to {out_path}")
        return 0

    if args.cmd == "run-diff":
        refclone_dir = Path(args.refclone)
        ftml_bin = Path(args.ftml_bin)

        print(f"Generated {len(cases)} cases. Running PHP ref (render + tokens) and FTML...")
        ref_render = run_php_ref(cases, refclone_dir, "render")
        ref_tokens = run_php_ref(cases, refclone_dir, "tokens")
        ftml_res = run_ftml(cases, ftml_bin)

        out_path = Path(args.output)
        out_path.parent.mkdir(parents=True, exist_ok=True)

        counts = Counter()
        adj_counts = Counter()
        disagreements_by_family: dict[str, list[dict[str, Any]]] = defaultdict(list)
        finding_rows: dict[str, list[str]] = defaultdict(list)

        with out_path.open("w", encoding="utf-8") as f:
            for c in cases:
                cid = c["case_id"]
                r_html = normalize_php_html(ref_render.get(cid, {}).get("output"))
                r_toks = ref_tokens.get(cid, {}).get("output", "[]")
                f_html = normalize_ftml_html(ftml_res.get(cid, {}).get("html"))

                if r_html == f_html:
                    verdict = "equal"
                elif c["save_inadmissible"]:
                    verdict = "disagree_save_inadmissible"
                else:
                    verdict = "disagree"

                adjudication, mechanism, evidence = adjudicate_row(c, r_html, f_html)
                counts[verdict] += 1
                adj_counts[adjudication] += 1
                for ftag in c["findings"]:
                    finding_rows[ftag].append(f"{cid}:{adjudication}")

                record = {
                    **c,
                    "verdict": verdict,
                    "adjudication": adjudication,
                    "mechanism": mechanism,
                    "evidence": evidence,
                    "ref_html": r_html,
                    "ftml_html": f_html,
                    "ref_tokens": r_toks,
                }
                f.write(json.dumps(record) + "\n")
                if adjudication in ("ftml_bug_candidate", "ref_or_undecided_bracket_interior"):
                    disagreements_by_family[f"{adjudication}:{mechanism}"].append(record)

        summary_path = Path(args.summary)
        summary_path.parent.mkdir(parents=True, exist_ok=True)
        with summary_path.open("w", encoding="utf-8") as sf:
            sf.write("# Track T5 Link Grammar Differential & Adjudication Summary\n\n")
            sf.write(f"Total generated cases: **{len(cases)}**\n\n")
            sf.write("## Raw Differential Counts\n\n")
            for k, v in sorted(counts.items()):
                sf.write(f"- `{k}`: {v}\n")
            sf.write("\n## Adjudication Breakdown\n\n")
            for k, v in sorted(adj_counts.items()):
                sf.write(f"- `{k}`: {v}\n")
            sf.write("\n## Findings 854-859, 862 Coverage\n\n")
            for ftag in ["854", "855", "856", "857", "858", "859", "862"]:
                rows = finding_rows.get(ftag, [])
                sf.write(f"- Finding {ftag}: {len(rows)} generated rows (e.g. {', '.join(rows[:5])})\n")
            sf.write("\n## Actionable & Undecided Clusters\n\n")
            for k, rows in sorted(disagreements_by_family.items()):
                sf.write(f"### {k} ({len(rows)} rows)\n\n")
                for r in rows[:10]:
                    sf.write(
                        f"- `{r['case_id']}` source={r['source']!r} ({r['evidence']})\n"
                        f"  - ref: `{r['ref_html']}`\n"
                        f"  - ftml: `{r['ftml_html']}`\n"
                    )
                if len(rows) > 10:
                    sf.write(f"- ... and {len(rows) - 10} more\n")
                sf.write("\n")

        print(f"Done. Raw: {dict(counts)}")
        print(f"Adjudicated: {dict(adj_counts)}")
        print(f"Summary written to {summary_path}")
        return 0

    return 1


if __name__ == "__main__":
    raise SystemExit(main())
