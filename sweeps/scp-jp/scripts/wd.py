"""Shared helpers for the scp-jp divergence sweep.

Anonymous, read-only HTTP access to public Wikidot pages:

* page HTML (``GET https://<site>.wikidot.com/<page>``) for the live render
  and the page id;
* ``viewsource/ViewSourceModule`` through ``ajax-module-connector.php`` for
  the page wikitext;
* ``list/ListPagesModule`` for rating-ordered candidate selection;
* ``edit/PagePreviewModule`` for minimized repro confirmation (renders a
  submitted source without saving anything).

No credentials are used. ``wikidot_token7`` is Wikidot's anonymous CSRF
double-submit token: any value works as long as cookie and field agree.
"""

import datetime
import hashlib
import html
import json
import re
import ssl
import time
import urllib.error
import urllib.parse
import urllib.request

USER_AGENT = "ftml-divergence-sweep/1.0 (+https://github.com/Rokurolize/ftml)"
TOKEN = "ftmlsweep"
PACE_SECONDS = 1.0

_last_request = [0.0]


def _context():
    ctx = ssl.create_default_context()
    try:
        ctx.load_verify_locations("/root/.ccr/ca-bundle.crt")
    except OSError:
        pass
    return ctx


_CTX = _context()


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")


def sha256(data):
    if isinstance(data, str):
        data = data.encode("utf-8")
    return hashlib.sha256(data).hexdigest()


def request(url, data=None, retries=4):
    """Perform one paced HTTP request; return (status, body_bytes, provenance)."""
    headers = {"User-Agent": USER_AGENT}
    if data is not None:
        headers["Cookie"] = f"wikidot_token7={TOKEN}"
        headers["Content-Type"] = "application/x-www-form-urlencoded; charset=UTF-8"
        data = urllib.parse.urlencode(data).encode("utf-8")
    delay = 2.0
    for attempt in range(retries + 1):
        wait = PACE_SECONDS - (time.monotonic() - _last_request[0])
        if wait > 0:
            time.sleep(wait)
        _last_request[0] = time.monotonic()
        req = urllib.request.Request(url, data=data, headers=headers)
        try:
            with urllib.request.urlopen(req, context=_CTX, timeout=60) as resp:
                body = resp.read()
                status = resp.status
        except urllib.error.HTTPError as err:
            body = err.read()
            status = err.code
            if status not in (404,) and attempt < retries and status >= 500:
                time.sleep(delay)
                delay *= 2
                continue
        except (urllib.error.URLError, TimeoutError, ConnectionError) as err:
            if attempt < retries:
                time.sleep(delay)
                delay *= 2
                continue
            raise RuntimeError(f"request failed: {url}: {err}") from err
        prov = {
            "url": url,
            "method": "POST" if data is not None else "GET",
            "fetched_at": now(),
            "status": status,
            "response_sha256": sha256(body),
            "response_bytes": len(body),
        }
        return status, body, prov
    raise RuntimeError(f"request failed after retries: {url}")


def module(site, module_name, **params):
    """Call ajax-module-connector anonymously; return (json, provenance)."""
    fields = {"moduleName": module_name, "wikidot_token7": TOKEN}
    fields.update(params)
    url = f"https://{site}.wikidot.com/ajax-module-connector.php"
    status, body, prov = request(url, data=fields)
    prov["module"] = module_name
    prov["params"] = {k: v for k, v in params.items() if k != "source"}
    if status != 200:
        raise RuntimeError(f"module {module_name} on {site}: HTTP {status}")
    payload = json.loads(body.decode("utf-8"))
    if payload.get("status") != "ok":
        raise RuntimeError(f"module {module_name} on {site}: {payload.get('status')}: {payload.get('message')}")
    return payload, prov


WIKIREQUEST_RE = re.compile(r'WIKIREQUEST\.info\.(\w+) = ("?)([^";]*)\2;')


def page_html(site, page):
    """Fetch a page's full live HTML. Returns dict with info, page-content, provenance."""
    url = f"https://{site}.wikidot.com/{page}"
    status, body, prov = request(url)
    text = body.decode("utf-8", errors="replace")
    info = {m.group(1): m.group(3) for m in WIKIREQUEST_RE.finditer(text)}
    content = extract_page_content(text) if status == 200 else None
    return {
        "status": status,
        "info": info,
        "page_id": int(info["pageId"]) if info.get("pageId", "").isdigit() else None,
        "page_content": content,
        "provenance": prov,
    }


DIV_TAG_RE = re.compile(r"<div\b[^>]*>|</div\s*>", re.IGNORECASE)


def extract_page_content(text):
    """Return the inner HTML of <div id="page-content"> by balanced div counting."""
    marker = '<div id="page-content">'
    start = text.find(marker)
    if start < 0:
        return None
    pos = start + len(marker)
    depth = 1
    for m in DIV_TAG_RE.finditer(text, pos):
        if m.group(0).startswith("</"):
            depth -= 1
            if depth == 0:
                return text[pos : m.start()]
        else:
            depth += 1
    return None


PAGE_SOURCE_RE = re.compile(r'<div class="page-source">\n\t(.*)\n</div>\s*$', re.DOTALL)


def decode_view_source(body):
    """Undo ViewSourceModule's presentation transforms to recover the wikitext.

    ViewSourceModule applies, in effect: htmlspecialchars, an <a> link around
    each [[include]] target, nl2br (``<br />`` before every newline), and
    ``&nbsp;`` for runs of spaces. Source text can never contain a raw ``<``
    (it would be escaped), so every remaining tag is presentation.
    """
    m = PAGE_SOURCE_RE.search(body)
    if not m:
        raise ValueError("no page-source div in ViewSourceModule body")
    inner = m.group(1)
    inner = inner.replace("<br />", "")
    inner = re.sub(r"<a\b[^>]*>|</a>", "", inner)
    inner = inner.replace("&nbsp;", " ")
    return html.unescape(inner)


def view_source(site, page_id):
    payload, prov = module(site, "viewsource/ViewSourceModule", page_id=str(page_id))
    return decode_view_source(payload["body"]), prov


def page_preview(site, source, page_unix_name="", title="preview"):
    """Render a source with PagePreviewModule (anonymous, nothing saved)."""
    payload, prov = module(
        site,
        "edit/PagePreviewModule",
        source=source,
        mode="page",
        title=title,
        page_unix_name=page_unix_name,
    )
    prov["source_sha256"] = sha256(source)
    return payload["body"], prov


def unix_name(name):
    """Approximate Wikidot's page unix-name normalisation for include targets."""
    name = name.strip().lower()
    name = re.sub(r"[^a-z0-9:_\-]+", "-", name)
    name = re.sub(r"-{2,}", "-", name)
    name = re.sub(r"(^|:)-+", r"\1", name)
    name = re.sub(r"-+($|:)", r"\1", name)
    return name


def split_page_ref(ref, current_site):
    """':site:page' -> (site, page); 'cat:page' -> (current_site, 'cat:page')."""
    ref = ref.strip()
    if ref.startswith(":"):
        parts = ref[1:].split(":", 1)
        if len(parts) == 2:
            return parts[0].strip().lower(), unix_name(parts[1])
    return current_site, unix_name(ref)
