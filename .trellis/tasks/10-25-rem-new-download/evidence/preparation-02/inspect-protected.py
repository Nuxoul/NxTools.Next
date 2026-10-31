from pathlib import Path
import hashlib,json,re
R=Path.cwd();S=Path(__file__).parent;h=lambda b:hashlib.sha256(b).hexdigest()
rows=json.loads((S/'baseline-manifest.json').read_text());p='rust/apps/nxtools-desktop/src/downloads/new_download_view.rs';row=next(x for x in rows if x['path']==p)
b=Path(row['baseline']).read_text(encoding='utf8');a=(R/p).read_text(encoding='utf8');checks=[]
def norm(t):return re.sub(r'\s+','',t)
def section(t,begin,end):
 i=t.index(begin);return t[i:t.index(end,i)]
def check(label,bb,aa):
 ok=norm(bb)==norm(aa);checks.append(dict(path=p,label=label,normalized_equal=ok,baseline_sha256=h(bb.encode()),current_sha256=h(aa.encode()),normalized_sha256=h(norm(bb).encode())));assert ok,label
# Imports may add ordinary_rem, never change any existing behavior declaration/method.
check('all original state/constructors/default Enter/category/request/probe/submit/footer action owners',section(b,'/// Translates raw engine','impl Render for NewDownloadView'),section(a,'/// Translates raw engine','impl Render for NewDownloadView').replace('use crate::ui::metrics::ordinary_rem;\n',''))
check('format/native sizing/media selection/name resolution/window options',section(b,'pub(crate) fn format_download_size','fn new_download_form_field'),section(a,'pub(crate) fn format_download_size','fn new_download_form_field'))
# Parse balanced argument lists while ignoring Rust comments and quoted strings.
def end_args(t,start):
 depth=1;i=start;mode=None;esc=False
 while i<len(t):
  c=t[i];n=t[i:i+2]
  if mode=='line':
   if c=='\n':mode=None
  elif mode=='block':
   if n=='*/':mode=None;i+=1
  elif mode=='string':
   if esc:esc=False
   elif c=='\\':esc=True
   elif c=='"':mode=None
  elif n=='//':mode='line';i+=1
  elif n=='/*':mode='block';i+=1
  elif c=='"':mode='string'
  elif c=='(':depth+=1
  elif c==')':
   depth-=1
   if depth==0:return i+1
  i+=1
 raise AssertionError('unbalanced callback')
def callbacks(t):
 result=[]
 for m in re.finditer(r'\.(on_click|on_change|on_open_change|capture_key_down|capture_key_up)\(',t):
  e=end_args(t,m.end());result.append((m.group(1),t[m.start():e]))
 return result
bc=callbacks(b);ac=callbacks(a);assert len(bc)==len(ac),(len(bc),len(ac))
for i,((kind,bb),(current_kind,aa)) in enumerate(zip(bc,ac)):
 assert kind==current_kind;check(f'original callback {i:02} {kind}',bb,aa)
for name in ['protected-baseline.json','frozen-history.json']:
 entries=json.loads((S/name).read_text());changed=[r['path'] for r in entries if not (R/r['path']).exists() or h((R/r['path']).read_bytes())!=r['sha256']];assert not changed,(name,changed);checks.append(dict(label=name,count=len(entries),unchanged=True))
ref=next(x for x in rows if x['path']=='docs/reference-code/download.md');before=Path(ref['baseline']).read_bytes();after=(R/ref['path']).read_bytes();assert after.startswith(before);checks.append(dict(label='original download reference exact prefix',path=ref['path'],prefix_equal=True,baseline_sha256=h(before)))
(S/'owner-invariants.json').write_text(json.dumps(checks,indent=2)+'\n',encoding='utf8');print('PASS',len(checks),'behavior/callback/protected/history/reference checks; callbacks',len(bc))
