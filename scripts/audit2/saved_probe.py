#!/usr/bin/env python3
"""Saved-page capture for corpus fixtures PagePreview cannot judge.

Creates/updates run-owned ``muse-probe-*`` pages on the sandbox with the
fixture source and the corpus runner's tags (fruit, component), then fetches
each saved page twice anonymously and compares the ``#page-content`` render
against the fixture's ``wikidot.html`` with the same comparator the
PagePreview audit uses.

Usage:
  saved_probe.py --repo WT --out DIR --cases a/b c/d ...
"""

import argparse
import hashlib
import json
import os
import re
import sys
import time
from pathlib import Path

import httpx
import wikidot
from bs4 import BeautifulSoup
from wikidot.connector.ajax import AjaxModuleConnectorConfig

SITE = "sandbox-for-codex"
TAGS = ["fruit", "component"]
UA = "ftml-corpus-audit2/1 (saved-page capture, run-owned probe pages)"


def extract_content(html):
    """Inner HTML of the #page-content container."""
    soup = BeautifulSoup(html, "html.parser")
    node = soup.find(id="page-content")
    if node is None:
        return None
    return node.decode_contents()


def fetch(fullname):
    url = f"https://{SITE}.wikidot.com/{fullname}"
    resp = httpx.get(url, headers={"User-Agent": UA}, timeout=60,
                     follow_redirects=True)
    return resp.status_code, resp.text


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--repo", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--cases", nargs="+", default=[])
    ap.add_argument("--cases-file", type=Path, default=None)
    ap.add_argument("--force", action="store_true")
    args = ap.parse_args()
    cases = list(args.cases)
    if args.cases_file:
        cases.extend(line.strip() for line in
                     args.cases_file.read_text(encoding="utf-8").splitlines()
                     if line.strip() and not line.startswith("#"))
    if not cases:
        ap.error("no cases given")

    repo = Path(args.repo)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    sys.path.insert(0, str(repo / "scripts"))
    import audit_corpus_live as audit

    manifest_path = out / "saved-manifest.json"
    manifest = json.loads(manifest_path.read_text()) if manifest_path.exists() else {}
    results_path = out / "saved-results.json"
    results = json.loads(results_path.read_text()) if results_path.exists() else {}
    case_to_name = {v: k for k, v in manifest.items()}

    def next_index():
        used = [int(n.rsplit("-", 1)[1]) for n in manifest
                if n.rsplit("-", 1)[-1].isdigit()]
        return max(used, default=0) + 1

    username = os.environ["WIKIDOT_USERNAME"]
    password = os.environ["WIKIDOT_PASSWORD"]
    config = AjaxModuleConnectorConfig(allow_insecure_session_transport_for=SITE)
    with wikidot.Client(username=username, password=password,
                        amc_config=config) as client:
        print("logged_in:", client.is_logged_in, flush=True)
        site = client.site.get(SITE)
        for case in cases:
            if case in results and not args.force:
                print("skip", case, flush=True)
                continue
            try:
                family, name = case.split("/", 1)
                case_dir = repo / "test" / family / name
                source = (case_dir / "input.ftml").read_text(encoding="utf-8")
                expected = (case_dir / "wikidot.html").read_text(encoding="utf-8")
                fullname = case_to_name.get(case) or f"muse-probe-audit2-{next_index()}"
                result = site.page.publish(
                    fullname=fullname,
                    title=f"audit2 {case}",
                    source=source,
                    comment="dsh-audit2 saved-page probe",
                    tags=TAGS,
                    verify_source=False,
                    post_save_visibility_attempts=3,
                )
                via = "created" if result.created else "updated"
                print("saved", fullname, case, via, flush=True)
                time.sleep(1.2)
                check = site.page.get(fullname)

                def good_fetch():
                    """Fetch until a 200 response with non-empty page content."""
                    last = (None, "", None)
                    for _ in range(4):
                        status, html = fetch(fullname)
                        content = extract_content(html) if status == 200 else None
                        last = (status, html, content)
                        if status == 200 and content is not None and (
                                content.strip() or not source.strip()):
                            return last
                        time.sleep(1.5)
                    return last

                status1, html1, content1 = good_fetch()
                time.sleep(0.8)
                status2, html2, content2 = good_fetch()
                content_file = out / "content" / (case.replace("/", "__") + ".html")
                content_file.parent.mkdir(parents=True, exist_ok=True)
                content_file.write_text(content1 or "", encoding="utf-8")
                verdict = audit.compare(content1 or "", expected)
                results[case] = {
                    "case": case,
                    "fullname": fullname,
                    "source": source,
                    "source_sha256": audit.sha256(source),
                    "tags": list(check.tags or []),
                    "status": [status1, status2],
                    "content_file": str(content_file),
                    "content_sha256": [
                        hashlib.sha256((content1 or "").encode()).hexdigest(),
                        hashlib.sha256((content2 or "").encode()).hexdigest()],
                    "full_html_sha256": [
                        hashlib.sha256(html1.encode()).hexdigest(),
                        hashlib.sha256(html2.encode()).hexdigest()],
                    "stable": content1 == content2,
                    "dom": verdict["dom"],
                    "text": verdict["text"],
                    "live_norm": verdict["live_norm"],
                    "fixture_norm": verdict["fixture_norm"],
                    "context": audit.preview_context(source),
                }
                manifest[fullname] = case
                results_path.write_text(json.dumps(results, ensure_ascii=False, indent=1))
                manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=1))
                print("captured", fullname, case, "stable=", results[case]["stable"],
                      "dom=", results[case]["dom"], "text=", results[case]["text"],
                      flush=True)
            except Exception as error:
                print("FAILED", case, repr(error), flush=True)
                continue


if __name__ == "__main__":
    main()
