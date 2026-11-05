#!/usr/bin/env python3
"""Fetch public page source (ViewSourceModule) and live rendered #page-content
for scp-wiki pages. Usage: fetch.py DATA_DIR SLUG [SLUG...]  (cached; polite 1s delay)."""
import html, json, re, sys, time, urllib.parse, urllib.request, http.cookiejar
from pathlib import Path

UA = "ftml-divergence-sweep/1 (discovery; public pages only)"
BASE = "https://scp-wiki.wikidot.com"
jar = http.cookiejar.CookieJar()
op = urllib.request.build_opener(urllib.request.HTTPCookieProcessor(jar))
op.addheaders = [("User-Agent", UA)]

def decode_source(body):
    """ViewSourceModule HTML -> original wikitext. nbsp entity stands for a space,
    <br />\\n for a newline, link-wrapped include targets are stripped of <a>."""
    m = re.search(r'<div class="page-source">\n\t?(.*)\n</div>\s*$', body, re.S)
    if not m:
        raise ValueError("unrecognised view-source body")
    s = m.group(1)
    s = s.replace("<br />\n", "\n")
    s = re.sub(r"</?a\b[^>]*>", "", s)
    s = s.replace("&nbsp;", " ")
    if re.search(r"<(?!\s)", s):
        raise ValueError("unexpected markup left in source")
    return html.unescape(s)

def live_content(page):
    """Inner HTML of #page-content, found by balancing <div> depth (ad scripts follow it)."""
    open_tag = '<div id="page-content">'
    a = page.find(open_tag)
    if a < 0:
        raise ValueError("no #page-content")
    start = a + len(open_tag)
    depth = 1
    for m in re.finditer(r"<div\b|</div>", page[start:]):
        depth += 1 if m.group(0) == "<div" else -1
        if depth == 0:
            return page[start:start + m.start()].strip("\n")
    raise ValueError("unbalanced #page-content")

def fetch(slug, out):
    d = out / slug
    if (d / "source.ftml").exists() and (d / "live.html").exists():
        return "cached"
    d.mkdir(parents=True, exist_ok=True)
    try:
        page = op.open(f"{BASE}/{slug}", timeout=60).read().decode("utf-8")
    except Exception as e:
        return f"page-fail {e}"
    (d / "page.raw.html").write_text(page)
    pid = re.search(r"pageId = (\d+)", page)
    if not pid:
        return "no-pageid"
    tok = next((c.value for c in jar if c.name == "wikidot_token7"), None)
    if not tok:
        return "no-token"
    time.sleep(1)
    data = urllib.parse.urlencode({"moduleName": "viewsource/ViewSourceModule", "page_id": pid.group(1),
                                   "callbackIndex": 1, "wikidot_token7": tok}).encode()
    req = urllib.request.Request(f"{BASE}/ajax-module-connector.php", data=data,
                                 headers={"Cookie": f"wikidot_token7={tok}", "User-Agent": UA})
    j = json.loads(urllib.request.urlopen(req, timeout=60).read())
    if j.get("status") != "ok":
        return f"vs-{j.get('status')}"
    (d / "viewsource.json").write_text(json.dumps(j, ensure_ascii=False))
    (d / "source.ftml").write_text(decode_source(j["body"]))
    (d / "live.html").write_text(live_content(page))
    (d / "meta.json").write_text(json.dumps({"slug": slug, "page_id": int(pid.group(1)),
                                             "url": f"{BASE}/{slug}", "fetched_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}))
    return "ok"

if __name__ == "__main__":
    out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
    for slug in sys.argv[2:]:
        print(slug, fetch(slug, out), flush=True)
        time.sleep(1)
