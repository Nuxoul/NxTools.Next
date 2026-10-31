"""Read-only Queue06 owner, live baseline and immutable-history checks."""
from pathlib import Path
import hashlib,json,re
R=Path(__file__).resolve().parents[4];T=Path(__file__).resolve().parents[1];E=T/'evidence'
sha=lambda b:hashlib.sha256(b).hexdigest()
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
norm=lambda s:re.sub(r'\s+','',s)
rows=load(E/'baseline-manifest.json');checks=[]
def section(s,start,end):
 i=s.index(start);return s[i:s.index(end,i)]
def equal(path,label,b,a):
 ok=norm(b)==norm(a);checks.append(dict(path=path,label=label,equal=ok,before_sha256=sha(b.encode()),after_sha256=sha(a.encode())));assert ok,label

def end_args(s,i):
 depth=1;mode=None;escape=False
 while i<len(s):
  c=s[i];pair=s[i:i+2]
  if mode=='line':
   if c=='\n':mode=None
  elif mode=='block':
   if pair=='*/':mode=None;i+=1
  elif mode=='string':
   if escape:escape=False
   elif c=='\\':escape=True
   elif c=='"':mode=None
  elif pair=='//':mode='line';i+=1
  elif pair=='/*':mode='block';i+=1
  elif c=='"':mode='string'
  elif c=='(':depth+=1
  elif c==')':
   depth-=1
   if depth==0:return i+1
  i+=1
 raise AssertionError('unbalanced callback')
def callbacks(s):
 return [(m.group(1),s[m.start():end_args(s,m.end())]) for m in re.finditer(r'\.(on_click|on_change|on_open_change|on_mouse_down|capture_key_down|capture_key_up)\(',s)]
settings='rust/apps/nxtools-desktop/src/downloads/settings_view.rs';task='rust/apps/nxtools-desktop/src/downloads/task_window.rs'
for p in [settings,task]:
 row=next(x for x in rows if x['path']==p)
 b=(T/row['baseline']).read_text(encoding='utf8');a=(R/p).read_text(encoding='utf8')
 if p==settings:
  ranges=[('settings state constructors parsing subscriptions picker draft save merge notification owner','pub(crate) struct DownloadSettingsInput','impl DownloadSettingsView {\n    fn render_setting_control'),('settings semantic Button choices and callbacks','pub(crate) fn download_settings_anchor','pub(crate) fn download_settings_path_row'),('settings group/field spec and old regression suffix','impl DownloadSettingsPage','#[cfg(test)]\n#[path = "settings_task_rem_tests.rs"]')]
  for label,start,end in ranges:
   if label.endswith('suffix'):
    equal(p,label,b[b.index(start):],a[a.index(start):a.index(end)])
   else:equal(p,label,section(b,start,end),section(a,start,end))
 else:
  ranges=[('native phase/minimum/details policy shell and retained snapshot state','pub(crate) struct DownloadTaskWindowPhaseLayout','impl Render for DownloadTaskWindowView'),('display sampling and fixed expanded/collapsed constants','/// The collapsed progress window','fn task_window_filename'),('segment status semantics','pub(crate) fn segment_display_status','fn segment_progress_table')]
  for label,start,end in ranges:equal(p,label,section(b,start,end),section(a,start,end))
  equal(p,'task capabilities semantic suffix',b[b.index('pub(super) fn task_window_capabilities'):],a[a.index('pub(super) fn task_window_capabilities'):])
 bc,ac=callbacks(b),callbacks(a);assert len(bc)==len(ac),(p,len(bc),len(ac))
 for i,((kind,old),(new_kind,current)) in enumerate(zip(bc,ac)):
  assert kind==new_kind;equal(p,f'original callback {i:02} {kind}',old,current)
 assert 'DEBUG-AUX' not in a
protected=[]
for row in rows:
 before=(T/row['baseline']).read_bytes();assert sha(before)==row['sha256'],row['path']
 if row['path'] not in [settings,task,'docs/reference-code/download.md']:
  current=(R/row['path']).read_bytes();assert sha(current)==row['sha256'],row['path'];protected.append(dict(path=row['path'],sha256=sha(current)))
ref=next(x for x in rows if x['path']=='docs/reference-code/download.md')
assert (R/ref['path']).read_bytes().startswith((T/ref['baseline']).read_bytes())
previous=[]
for name,expected in [('10-26-rem-new-download-basic','091cfbf96febcf32a2811de9ded60b407cf61f42964d24e09410659bc4976c68'),('10-26-rem-new-download-overlays','53b16bf699f382cfdb0c93890f2c0aa3cff49fd081ead55badcbd923d720b4a9')]:
 root=R/'.trellis/tasks'/name;manifest_path=root/'evidence/manifest.json';assert sha(manifest_path.read_bytes())==expected,name
 manifest=load(manifest_path)
 for item in manifest['evidence']:assert sha((root/item['path']).read_bytes())==item['sha256'],item['path']
 for item in manifest['documents']:assert sha((R/item['path']).read_bytes())==item['sha256'],item['path']
 previous.append(dict(task=name,evidence_count=len(manifest['evidence']),manifest_sha256=expected))
history=load(R/'.trellis/tasks/10-25-rem-new-download/evidence/checkpoint-01/frozen-history.json')
for item in history:assert sha((R/item['path']).read_bytes())==item['sha256'],item['path']
assert 'DEBUG-AUX' not in (R/'rust/apps/nxtools-desktop/src/downloads/settings_task_rem_tests.rs').read_text(encoding='utf8')
print(json.dumps(dict(owner_checks=checks,protected_files=protected,previous_deliveries=previous,frozen_history_count=len(history),reference_prefix_equal=True),indent=2))
print(f'PASS {len(checks)} owner/section/callback checks,{len(protected)} live protected files,161 prior evidence hashes,{len(history)} frozen history')
