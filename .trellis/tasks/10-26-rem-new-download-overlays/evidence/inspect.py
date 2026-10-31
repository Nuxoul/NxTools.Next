"""Read-only Queue05 scope/protected-history verification."""
from pathlib import Path
import hashlib,json,re
R=Path(__file__).resolve().parents[4]
T=Path(__file__).resolve().parents[1]
E=T/'evidence'
sha=lambda b:hashlib.sha256(b).hexdigest()
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
norm=lambda s:re.sub(r'\s+','',s)
rows=load(E/'baseline-manifest.json')
p='rust/apps/nxtools-desktop/src/downloads/new_download_view.rs'
row=next(x for x in rows if x['path']==p)
b=(T/row['baseline']).read_text(encoding='utf8');a=(R/p).read_text(encoding='utf8')
checks=[]
def section(s,start,end):
 i=s.index(start);return s[i:s.index(end,i)]
def equal(label,left,right):
 ok=norm(left)==norm(right);checks.append(dict(label=label,equal=ok,before_sha256=sha(left.encode()),after_sha256=sha(right.encode())));assert ok,label
for label,start,end in [
 ('state constructors Enter IME picker probe350 generation TLS submit deferred lifecycle','/// Translates raw engine','impl Render for NewDownloadView'),
 ('Queue04 footer actions basic confirmed compact body caption footer','        let footer_leading_actions','// Released Popover'),
 ('native size error media budget typed selection names window options','pub(crate) fn format_download_size','fn new_download_form_field'),
 ('Queue04 basic Form Input picker helper and paste callback','fn new_download_form_field','fn new_download_media_radio'),
 ('HLS labels and classification','pub(crate) fn hls_variant_detail_label','pub(crate) fn dash_track_options'),
 ('DASH nullable selection implementation','#[derive(Clone, Copy)]\npub(crate) enum DashTrackKind','pub(crate) fn dash_track_option_row'),
 ('DASH original metadata and click owner','pub(crate) fn dash_track_option_row','pub(crate) fn delayed_start_calendar'),
 ('resolution details Popover and inspector motion lifecycle','    let inspector = if let Some(resolution)','/// Stateless layout only:')
]:
 if label.startswith('resolution'):
  equal(label,section(b,start,'pub(crate) fn hls_quality_options'),section(a,start,end))
 else:equal(label,section(b,start,end),section(a,start,end))

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
 return [(m.group(1),s[m.start():end_args(s,m.end())]) for m in re.finditer(r'\.(on_click|on_change|on_open_change|capture_key_down|capture_key_up)\(',s)]
bc,ac=callbacks(b),callbacks(a);assert len(bc)==len(ac)
for i,((kind,left),(current_kind,right)) in enumerate(zip(bc,ac)):
 assert kind==current_kind;equal(f'original callback {i:02} {kind}',left,right)
protected=[]
for row in rows:
 before=(T/row['baseline']).read_bytes();assert sha(before)==row['sha256'],row['path']
 if row['path'] not in [p,'docs/reference-code/download.md']:
  current=(R/row['path']).read_bytes();assert sha(current)==row['sha256'],row['path']
  protected.append(dict(path=row['path'],sha256=sha(current)))
ref=next(x for x in rows if x['path']=='docs/reference-code/download.md')
assert (R/ref['path']).read_bytes().startswith((T/ref['baseline']).read_bytes())
previous=R/'.trellis/tasks/10-26-rem-new-download-basic'
assert sha((previous/'evidence/manifest.json').read_bytes())=='091cfbf96febcf32a2811de9ded60b407cf61f42964d24e09410659bc4976c68', 'Queue04 manifest changed'
manifest=load(previous/'evidence/manifest.json')
for item in manifest['evidence']:
 assert sha((previous/item['path']).read_bytes())==item['sha256'],item['path']
for item in manifest['documents']:
 assert sha((R/item['path']).read_bytes())==item['sha256'],item['path']
history=load(R/'.trellis/tasks/10-25-rem-new-download/evidence/checkpoint-01/frozen-history.json')
for item in history:assert sha((R/item['path']).read_bytes())==item['sha256'],item['path']
assert 'DEBUG-OVERLAY' not in a
new_test=(R/'rust/apps/nxtools-desktop/src/downloads/new_download_overlay_rem_tests.rs').read_text(encoding='utf8')
assert 'DEBUG-OVERLAY' not in new_test
result=dict(owner_checks=checks,callbacks=len(bc),protected_files=protected,reference_prefix_equal=True,previous_evidence_count=len(manifest['evidence']),previous_manifest_sha256=sha((previous/'evidence/manifest.json').read_bytes()),frozen_history_count=len(history))
print(json.dumps(result,indent=2))
print(f'PASS {len(checks)} owner checks including {len(bc)} original callbacks; {len(protected)} live protected files; {len(manifest["evidence"])} Queue04 evidence; {len(history)} frozen history')
