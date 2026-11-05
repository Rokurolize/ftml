#!/usr/bin/env python3
"""Minimal-difference resolution of the tabview structural divergences."""
import json, re, sys, itertools, os
from pathlib import Path
REPO=Path(__file__).resolve().parents[2]
CFG=Path(os.environ.get('FTML_AUDIT_RECORD', REPO/'.audit'))
sys.path.insert(0,str(REPO/'scripts')); sys.path.insert(0,str(CFG))
import audit_corpus_live as audit, classify_audit as C
def base(html):
    html=re.sub(r'<script\b[^>]*src="https?://[^"]*cloudfront\.net/[^"]*"[^>]*>\s*</script>','',html)
    html=re.sub(r'(?s)<script\b[^>]*>(?:(?!</script>).)*?YAHOO\.widget\.TabView.*?</script>','',html)
    return html
def fix_display(h): return re.sub(r'(<div id="wiki-tab-[^"]*")\s+style="display:none"', r'\1', h)
def fix_em(h): return h.replace('<em></em>','')
def fix_br(h): return re.sub(r'<br>\n?</p>', '</p>', h)
def fix_bq(h):
    prev=None
    while prev!=h:
        prev=h; h=re.sub(r'<blockquote>\s*</blockquote>','',h)
    return h
FIX={'display':fix_display,'em':fix_em,'br':fix_br,'bq':fix_bq}
def eq(a,b):
    la,lb=audit.parse(a),audit.parse(b)
    sa=[t for t in audit.tokens(la,False) if t[0]!='text']; sb=[t for t in audit.tokens(lb,False) if t[0]!='text']
    return sa==sb and audit.visible_text(la)==audit.visible_text(lb)
rows={json.loads(l)['case']:json.loads(l) for l in (CFG/'audit-all.jsonl').read_text().splitlines()}
classified=json.load(open(CFG/'classified.json'))['cases']
saved=json.load(open(CFG/'saved-pages/saved-results.json'))
bugs=sorted(c for c,e in classified.items() if e['class'].startswith('bug') and c.startswith('tabview/'))
res={}
for case in bugs:
    entry=classified[case]
    exp0=base((REPO/'test'/case/'wikidot.html').read_text())
    if entry.get('evidence')=='saved-page':
        raw0=base(Path(saved[case]['content_file']).read_text())
    else:
        raw0=base(json.loads((CFG/'preview-cache'/(rows[case]['source_sha256']+'.json')).read_text())['raw_html'])
    found=None
    for n in range(0,5):
        for combo in itertools.combinations(FIX,n):
            l=raw0; f=exp0
            for name in combo:
                l=FIX[name](l); f=FIX[name](f)
            if eq(l,f): found=combo; break
        if found is not None: break
    res[case]=list(found) if found is not None else None
json.dump(res,open(CFG/'tabview-class.json','w'),indent=1)
for c,v in res.items(): print(f"{c:46} {v}")
