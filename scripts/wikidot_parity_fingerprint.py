#!/usr/bin/env python3

import re


def _bucket(value: int) -> str:
    if value == 0:
        return "0"
    if value == 1:
        return "1"
    if value == 2:
        return "2"
    return "3+"


def fingerprint(source: str) -> str:
    if source.startswith("\t"):
        owner = "leading-tab"
    elif source.startswith(" "):
        owner = "leading-space"
    elif re.match(r"^\+{1,6}(?:\*?)\s", source):
        owner = "heading"
    elif re.match(r"^[*#]+\s", source):
        owner = "list"
    elif source.startswith(">"):
        owner = "quote"
    elif source.startswith("||"):
        owner = "table"
    elif source.startswith("[[/"):
        owner = "block-close"
    elif source.startswith("[["):
        owner = "block-open"
    elif source.startswith("@@"):
        owner = "raw"
    elif source.startswith("{{"):
        owner = "mono"
    elif source.startswith(("[http", "[https", "[ftp")):
        owner = "single-link"
    elif re.match(r"^(?:https?|ftp|mailto):", source, re.IGNORECASE):
        owner = "auto-url"
    elif source.startswith("(("):
        owner = "bib"
    else:
        owner = "plain"

    families = []
    candidates = [
        ("block", "[[" in source),
        ("block-close", "[[/" in source),
        ("parserfn", "[[#" in source),
        ("comment", "[!--" in source),
        ("single-link", bool(re.search(r"\[(?:https?|ftp|mailto):", source, re.IGNORECASE))),
        ("auto-url", bool(re.search(r"(?<!\[)(?:https?|ftp|mailto):", source, re.IGNORECASE))),
        ("table", "||" in source),
        ("raw", "@@" in source or "@<" in source),
        ("mono", "{{" in source or "}}" in source),
        ("footnote", "[[footnote" in source.lower()),
        ("bib", "((bib" in source.lower()),
        ("bold", "**" in source),
        ("italics", "//" in source),
        ("underline", "__" in source),
        ("strike", "--" in source),
        ("sub", ",," in source),
        ("super", "^^" in source),
        ("math", "[[$" in source or "[[math" in source.lower()),
        ("image", "[[image" in source.lower()),
        ("module", "[[module" in source.lower()),
        ("collapsible", "[[collapsible" in source.lower()),
    ]
    for name, present in candidates:
        if present:
            families.append(name)

    delimiter_count = sum(
        name in families for name in ("bold", "italics", "underline", "strike", "sub", "super", "mono")
    )
    controls = any(ord(char) < 32 and char not in "\n\r\t" for char in source)
    non_ascii_whitespace = any(char.isspace() and ord(char) > 127 for char in source)
    line_count = source.count("\n") + (1 if source else 0)
    same_line_block = "[[/" in source and "\n" not in source.strip("\n")
    open_blocks = len(re.findall(r"\[\[(?!/|#)", source))
    close_blocks = source.count("[[/")
    bracket_delta = source.count("[[") - source.count("]]" )
    bracket_state = "neg" if bracket_delta < 0 else "zero" if bracket_delta == 0 else "pos"

    return "|".join(
        [
            f"owner={owner}",
            f"fam={','.join(families) or 'none'}",
            f"lines={_bucket(line_count)}",
            f"pipes={_bucket(source.count('|'))}",
            f"delim={_bucket(delimiter_count)}",
            f"openblk={_bucket(open_blocks)}",
            f"closeblk={_bucket(close_blocks)}",
            f"bracketdelta={bracket_state}",
            f"tab={int(chr(9) in source)}",
            f"cr={int(chr(13) in source)}",
            f"control={int(controls)}",
            f"nonasciiws={int(non_ascii_whitespace)}",
            f"samelineblk={int(same_line_block)}",
        ]
    )


def primary_family(source: str) -> str:
    lowered = source.lower()
    if "[[#" in lowered:
        return "parser-function"
    if "||" in lowered:
        return "table"
    if "[[collapsible" in lowered:
        return "collapsible"
    if "[[module" in lowered:
        return "module"
    if "[[image" in lowered:
        return "image"
    if "[http" in lowered or "[https" in lowered or re.search(
        r"(?<!\[)(?:https?|ftp|mailto):", lowered
    ):
        return "link-url"
    if "[[" in lowered:
        return "block-inline-owner"
    if any(token in lowered for token in ("**", "//", "__", "--", ",,", "^^", "{{")):
        return "inline-delimiter"
    if lowered.startswith(("*", "#")):
        return "list"
    if lowered.startswith(">"):
        return "quote"
    return "preproc-lexical"
