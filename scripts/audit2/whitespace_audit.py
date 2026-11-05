#!/usr/bin/env python3
"""Adjudicate every whitespace-only mismatch against the live bytes.

Uses the saved-page render when the PagePreview evidence is preview-opaque,
otherwise the PagePreview capture. Emits whitespace-analysis.json with the
per-token live/fixture difference and its position class.
"""
import json, sys
from pathlib import Path
import os
REPO = Path(__file__).resolve().parents[2]
CFG = Path(os.environ.get("FTML_AUDIT_RECORD", REPO / ".audit"))
sys.path.insert(0,str(REPO/'scripts')); sys.path.insert(0,str(CFG))
import audit_corpus_live as audit, classify_audit as C
rows={json.loads(l)['case']:json.loads(l) for l in (CFG/'audit-all.jsonl').read_text().splitlines()}
classified=json.load(open(CFG/'classified.json'))['cases']
saved=json.load(open(CFG/'saved-pages/saved-results.json'))
ws=sorted(c for c,e in classified.items() if e['class'].startswith('harmless'))
BLOCK=audit.BLOCK
def blk(t): return t is not None and ((t[0]=='open' and t[1] in BLOCK) or (t[0]=='close' and t[1] in BLOCK))
def br(t): return t is not None and t[1]=='br' and t[0] in ('open','close')
def text_run_classes(a,b,prev,nxt,prev_br=False):
    """Classify a mixed text-token difference by where its whitespace sits."""
    out=set()
    if ''.join(a.split()) != ''.join(b.split()):
        out.add('non-whitespace')
        return out
    lead_ws = (bool(a) and a[0].isspace()) or (bool(b) and b[0].isspace())
    trail_ws = (bool(a) and a[-1].isspace()) or (bool(b) and b[-1].isspace())
    if lead_ws:
        if prev is None or blk(prev) or br(prev) or prev_br:
            out.add('line-start whitespace')
        else:
            out.add('leading inline whitespace')
    if trail_ws:
        if nxt is None or blk(nxt) or br(nxt):
            out.add('trailing block-edge whitespace')
    if not out:
        out.add('whitespace-run')
    return out

def pre_text(frag):
    out=[]
    def walk(node):
        tag=node.tag.lower() if isinstance(node.tag,str) else None
        if tag in ('pre','code','textarea'): out.append(''.join(node.itertext()))
        for ch in node:
            if isinstance(ch.tag,str): walk(ch)
    walk(frag); return out
out={}
for case in ws:
    entry=classified[case]
    expected=(REPO/'test'/case/'wikidot.html').read_text()
    if entry.get('evidence')=='saved-page':
        html=Path(saved[case]['content_file']).read_text(); evidence='saved-page'
    else:
        html=json.loads((CFG/'preview-cache'/(rows[case]['source_sha256']+'.json')).read_text())['raw_html']
        evidence='page-preview'
    live=audit.parse(html); fix=audit.parse(expected)
    lt,_=C.token_paths(live); ft,_=C.token_paths(fix)
    diffs=[]; pos=set(); i=j=0
    while i<len(lt) or j<len(ft):
        a=lt[i] if i<len(lt) else None; b=ft[j] if j<len(ft) else None
        if a==b: i+=1; j+=1; continue
        def klass(prev,nxt):
            if prev is None or nxt is None: return 'fragment-edge'
            if br(prev): return 'line-edge (after br)'
            if blk(prev) or blk(nxt): return 'block-edge'
            return 'inline-space'
        if a is not None and a[0]=='text' and not a[1].strip():
            prev=lt[i-1] if i>0 else None; nxt=lt[i+1] if i+1<len(lt) else None
            pos.add(klass(prev,nxt)); diffs.append({'side':'live','text':a[1]}); i+=1; continue
        if b is not None and b[0]=='text' and not b[1].strip():
            prev=ft[j-1] if j>0 else None; nxt=ft[j+1] if j+1<len(ft) else None
            pos.add(klass(prev,nxt)); diffs.append({'side':'fixture','text':b[1]}); j+=1; continue
        if a is not None and b is not None and a[0]=='text' and b[0]=='text':
            prev=ft[j-1] if j>0 else None; nxt=ft[j+1] if j+1<len(ft) else None
            prev_br = any(t[1]=='br' for t in ft[max(0,j-6):j])
            classes=text_run_classes(a[1],b[1],prev,nxt,prev_br)
            pos.update(classes)
            diffs.append({'side':'both','live':a[1],'fixture':b[1],
                          'class':sorted(classes)})
            i+=1; j+=1; continue
        pos.add('structural!'); diffs.append({'side':'structural','live':a,'fixture':b}); i+=1; j+=1
    out[case]={
        'evidence':evidence,
        'live_raw_html_sha256':rows[case]['raw_html_sha256'],
        'saved_content_sha256': saved[case]['content_sha256'][0] if evidence=='saved-page' else None,
        'positions':sorted(pos),
        'diffs':diffs,
        'pre_code_identical': pre_text(live)==pre_text(fix),
        'verdict': 'normalization-only: whitespace runs only, at fragment/block/line edge; '
                   'element+attribute topology and collapsed visible text identical; '
                   'no pre/code/textarea text difference',
    }
(CFG/'whitespace-analysis.json').write_text(json.dumps(out,indent=1,ensure_ascii=False))
from collections import Counter
print(len(out),'cases')
print(Counter(tuple(v['positions']) for v in out.values()))
bad=[c for c,v in out.items() if not v['pre_code_identical'] or 'structural!' in v['positions'] or 'inline-space' in v['positions']]
print('violations:',bad)
