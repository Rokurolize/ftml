#!/usr/bin/env python3
"""HTML -> canonical token lines, with per-render random ids / timestamps masked.
Both sides go through the same function; only presentation-neutral differences are removed:
  entity vs literal char, attribute quote style, <br> vs <br />, runs of whitespace,
  whitespace-only text between tags. Attribute order and values are preserved."""
import re
from html import escape
from html.parser import HTMLParser

VOID = {"br", "hr", "img", "input", "meta", "link"}

MASKS = [
    (re.compile(r"\bprw\d+\b"), "prwN"),                      # rate widget id
    (re.compile(r"\btime_\d{9,}\b"), "time_TS"),              # odate timestamp class
    (re.compile(r"\b1[5-9]\d{8}\b"), "TS"),                   # bare unix timestamps
    (re.compile(r"\b[0-9a-f]{16,}\b"), "HEX"),                # long random hex ids
    (re.compile(r"\b(bibitem-)[0-9a-z]{8,12}\b"), r"\1RND"),
    (re.compile(r"\bwiki-tabview-[0-9a-f]+\b"), "wiki-tabview-RND"),
]

def mask(s):
    for rx, rep in MASKS:
        s = rx.sub(rep, s)
    return s

INLINE = {"a","b","strong","em","i","u","s","sup","sub","tt","code","del","ins","span","big","small","font","strike","img","br"}

class Tok(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.out, self.buf = [], []
    def flush(self):
        t = re.sub(r"[ \t\r\n\f]+", " ", "".join(self.buf))
        self.buf = []
        if t:
            self.out.append(("T", t))
    def tag(self, name, attrs):
        self.flush()
        a = "".join(f' {k}="{escape(v if v is not None else "", quote=True)}"' for k, v in attrs)
        self.out.append(("S", name, f"<{name}{a}>"))
    def handle_starttag(self, name, attrs): self.tag(name, attrs)
    def handle_startendtag(self, name, attrs): self.tag(name, attrs)
    def handle_endtag(self, name):
        if name in VOID: return
        self.flush(); self.out.append(("E", name, f"</{name}>"))
    def handle_data(self, d): self.buf.append(d)
    def handle_comment(self, d): pass

def _inline(tok):
    return tok is not None and tok[0] in "SE" and tok[1] in INLINE

def normalize(html, do_mask=True):
    t = Tok(); t.feed(html); t.flush()
    toks, lines = t.out, []
    for i, tok in enumerate(toks):
        if tok[0] == "T":
            txt = tok[1]
            prev = toks[i-1] if i else None
            nxt = toks[i+1] if i + 1 < len(toks) else None
            # edge spaces next to a block boundary (or document edge) are layout noise
            if not _inline(prev) and not (prev and prev[0] == "T"):
                txt = txt.lstrip(" ")
            if not _inline(nxt) and not (nxt and nxt[0] == "T"):
                txt = txt.rstrip(" ")
            if txt:
                lines.append('T"' + escape(txt, quote=False) + '"')
        else:
            lines.append(tok[2])
    return [mask(l) if do_mask else l for l in lines]
