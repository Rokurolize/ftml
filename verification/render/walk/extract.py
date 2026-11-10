#!/usr/bin/env python3
import pathlib
import sys

root = pathlib.Path(__file__).resolve().parents[3]
out = pathlib.Path(sys.argv[1])


def need(path, needle, label):
    file = root / path
    text = file.read_text(encoding="utf-8")
    count = text.count(needle)
    if count != 1:
        print(f"MODEL DRIFT: {path} {label} count {count}", file=sys.stderr)
        raise SystemExit(2)
    return True


fn_contains = need(
    "src/render/html/context.rs",
    "if self.footnote_render_stack.contains(&index)",
    "footnote contains guard",
)
fn_exit = need(
    "src/render/html/element/footnotes.rs",
    "if tooltip_text.is_none() {\n        ctx.exit_footnote_ref(index.get());\n    }",
    "footnote exit guard",
)
bib_return = need(
    "src/render/html/element/bibliography.rs",
    "ctx.exit_bibliography_ref(label);\n                return;",
    "bibliography legacy return",
)
walk_fn = need(
    "src/render/html/element/math.rs",
    "stack.push((ctx.footnotes()[number - 1].as_slice(), 0));",
    "equation footnote walk",
)
abort_dup = need(
    "src/render/html/element/tabs.rs",
    "if assignments[view_index].is_some() {\n                return None;\n            }",
    "tab nav duplicate abort",
)
flush = need(
    "src/render/html/element/tabs.rs",
    "let mut lifts = ctx.take_wikidot_tabview_lifts();",
    "tab lift flush",
)
restore = need(
    "src/render/html/element/tabs.rs",
    "ctx.set_wikidot_tabview_depth(depth);",
    "tab depth restore",
)
need(
    "src/render/html/element/tabs.rs",
    "ctx.set_wikidot_tabview_depth(depth + 1);",
    "tab depth enter",
)


def flag(value):
    return "TRUE" if value else "FALSE"


def write(name, lines):
    body = "CONSTANTS\n" + "".join(f"  {line}\n" for line in lines)
    (out / name).write_text(body, encoding="utf-8")


write(
    "ref.cfg",
    [
        f"FnContains = {flag(fn_contains)}",
        f"FnExitOnlyElements = {flag(fn_exit)}",
        f"BibLegacyReturns = {flag(bib_return)}",
    ],
)
write("eq.cfg", [f"WalkFootnoteBodies = {flag(walk_fn)}"])
write("nav.cfg", [f"AbortOnDuplicate = {flag(abort_dup)}"])
write(
    "lift.cfg",
    [
        f"FlushOuter = {flag(flush)}",
        f"RestoreDepth = {flag(restore)}",
    ],
)
print("extract ok")
