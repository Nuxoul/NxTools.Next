from pathlib import Path
import base64,datetime,difflib,hashlib,json,re
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-quick-retained-text';ev=task/'evidence';scratch=Path(__file__).parent
owned=['docs/reference-code/quick-launch-research.md','rust/apps/nxtools-desktop/src/quick_launch.rs','rust/apps/nxtools-desktop/src/quick_launch/panel.rs','rust/apps/nxtools-desktop/src/quick_launch/actions.rs','rust/apps/nxtools-desktop/src/quick_launch/tests.rs','rust/apps/nxtools-desktop/src/quick_launch/extension_providers.rs']
new=['rust/apps/nxtools-desktop/src/quick_launch/retained_text.rs','rust/apps/nxtools-desktop/src/quick_launch/retained_text_tests.rs']
assert not (ev/'own-deltas.json').exists(),'Never overwrite published evidence'
def sha(data):return hashlib.sha256(data).hexdigest()
def put(path,data):
 assert not path.exists(),str(path)
 path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
def dump(path,obj):put(path,(json.dumps(obj,indent=2)+'\n').encode())
base=json.loads((ev/'baseline-manifest.json').read_text())['files'];by={x['path']:x for x in base};history=json.loads((ev/'historical-manifest.json').read_text())['files']
protected=[]
for x in base:
 assert sha((task/x['snapshot']).read_bytes())==x['sha256'],'Baseline snapshot movement '+x['path']
 if x['path'] not in owned:
  assert sha((root/x['path']).read_bytes())==x['sha256'],'Unexpected live movement '+x['path']
  protected.append(x)
for x in history:assert sha((root/x['path']).read_bytes())==x['sha256'],'Historical movement '+x['path']
checks=[]
def old(p):return (task/by[p]['snapshot']).read_text(encoding='utf-8')
def live(p):return (root/p).read_text(encoding='utf-8')
def same(name,a,b):assert a==b,name;checks.append({'name':name,'result':'PASS','sha256':sha(a.encode())})
def methods(text):
 out=[]
 for m in re.finditer(r'(?m)^    (?:pub(?:\([^)]*\))? )?(?:async )?fn ([a-zA-Z_0-9]+)\(',text):
  end=re.search(r'(?m)^    }\s*$',text[m.end():]);assert end,m.group(1)
  out.append((m.group(1),text[m.start():m.end()+end.end()]))
 return out
for p,changed in [('rust/apps/nxtools-desktop/src/quick_launch/panel.rs',{'new','prepare_for_show','run_search','enter_recent_directory_mode','replace_results','set_error','render_result','render_action','render'}),('rust/apps/nxtools-desktop/src/quick_launch/actions.rs',{'enter_in_context','title'})]:
 a=methods(old(p));b=methods(live(p));assert [n for n,_ in a]==[n for n,_ in b],p
 actual=set()
 for (name,left),(_,right) in zip(a,b):
  if left==right:checks.append({'name':'unchanged original method '+p+'::'+name,'result':'PASS','sha256':sha(left.encode())})
  else:actual.add(name)
 assert actual==changed,(p,actual,changed)
p='rust/apps/nxtools-desktop/src/quick_launch/tests.rs';after=live(p)
reversed_text=after.replace('panel.results = recent_directory_results(&panel.recent_directories, "").into();','panel.results = recent_directory_results(&panel.recent_directories, "");').replace('panel.replace_results(panel.results.to_vec(), cx);','panel.replace_results(panel.results.clone(), cx);').replace('panel.results = vec![search_result("current", ApplicationSource::IndexedFile)].into();','panel.results = vec![search_result("current", ApplicationSource::IndexedFile)];')
for label in ['current-ime-result','current-cancel-result','composing-result','committed-result']:
 a=f'panel.results = vec![search_result(\n            "{label}",\n            ApplicationSource::IndexedFile,\n        )]\n        .into();';b=a.replace(')]\n        .into();',')];');assert a in reversed_text,label;reversed_text=reversed_text.replace(a,b)
same('existing QuickLaunch tests exact inverse: container adaptations only, all old assertions unchanged',old(p),reversed_text)
p='rust/apps/nxtools-desktop/src/quick_launch/extension_providers.rs';same('extension provider production and old tests unchanged except one fixture conversion',old(p),live(p).replace('panel.results = vec![normal()].into();','panel.results = vec![normal()];'))
p='rust/apps/nxtools-desktop/src/quick_launch.rs';same('original application/search generation/preview/native lifecycle owner exact inverse, module registration only',old(p),live(p).replace('mod retained_text;\nuse retained_text::QuickLaunchResults;\n','').replace('#[cfg(test)]\nmod retained_text_tests;\n',''))
p='docs/reference-code/quick-launch-research.md';assert (root/p).read_bytes().startswith((task/by[p]['snapshot']).read_bytes());checks.append({'name':'Quick Launch reference exact live-baseline byte prefix','result':'PASS'})
for p in ['Cargo.toml','Cargo.lock','rust/apps/nxtools-desktop/Cargo.toml','rust/apps/nxtools-desktop/src/quick_launch/search.rs','rust/apps/nxtools-desktop/src/quick_launch/preview.rs','rust/apps/nxtools-desktop/src/quick_launch/polish_tests.rs','rust/apps/nxtools-desktop/src/quick_launch/file_dialog_search.rs','rust/apps/nxtools-desktop/src/ui/metrics.rs','rust/apps/nxtools-desktop/src/ui/typography.rs','tests/architecture/quick-launch-lifecycle.ps1']:
 assert sha((root/p).read_bytes())==by[p]['sha256'];checks.append({'name':'unchanged protected owner '+p,'result':'PASS','sha256':by[p]['sha256']})
summary={}
for name,count in [('audit13-retained-text-red-02',None),('audit13-final-formatted',5),('quick-original-final',121),('quick-polish-final',11)]:
 text=(scratch/(name+'.log')).read_text(encoding='utf-8',errors='replace');lines=re.findall(r'test result:.*',text);assert lines,name;summary[name]=lines[-1]
 if count:assert f'{count} passed; 0 failed; 0 ignored' in lines[-1],name
assert '0 passed; 2 failed' in summary['audit13-retained-text-red-02']
for name in ['desktop-strict-tests-final','workspace-check-final']:assert json.loads((scratch/(name+'.json')).read_text(encoding='utf-8-sig'))['exit']==0
assert json.loads((scratch/'desktop-strict-production.json').read_text(encoding='utf-8-sig'))['exit']!=0
finish=json.loads((scratch/'finish-check-results-02.json').read_text(encoding='utf-8-sig'));assert len(finish)==13
assert [v['exit'] for v in finish[:3]]==[0,1,0]
assert sum(v['exit']==0 for v in finish[3:])==9 and finish[3]['exit']==1
for p in new:
 text=live(p);assert '#[ignore' not in text and 'allow(clippy' not in text and 'DEBUG-' not in text
# All validation above precedes durable publication.
deltas=[]
for rel in owned:
 before=(task/by[rel]['snapshot']).read_bytes();after=(root/rel).read_bytes();left=before.splitlines(keepends=True);right=after.splitlines(keepends=True);ops=[]
 for kind,i,j,k,l in difflib.SequenceMatcher(a=left,b=right,autojunk=False).get_opcodes():
  if kind!='equal':ops.append({'old_start':i,'old_end':j,'new_start':k,'new_end':l,'old':base64.b64encode(b''.join(left[i:j])).decode(),'new':base64.b64encode(b''.join(right[k:l])).decode()})
 assert ops,rel
 forward=left.copy();reverse=right.copy()
 for op in reversed(ops):
  forward[op['old_start']:op['old_end']]=base64.b64decode(op['new']).splitlines(keepends=True)
  reverse[op['new_start']:op['new_end']]=base64.b64decode(op['old']).splitlines(keepends=True)
 assert b''.join(forward)==after and b''.join(reverse)==before
 dest='evidence/current/'+rel;put(task/dest,after)
 diff=''.join(difflib.unified_diff(before.decode().splitlines(True),after.decode().splitlines(True),fromfile='live-dirty-baseline/'+rel,tofile='own-current/'+rel));put(ev/'deltas'/(rel.replace('/','__')+'.txt'),diff.encode())
 deltas.append({'path':rel,'before_sha256':sha(before),'after_sha256':sha(after),'after_snapshot':dest,'ops':ops})
for rel in new:
 data=(root/rel).read_bytes();dest='evidence/current/'+rel;put(task/dest,data);deltas.append({'path':rel,'new_file':True,'after_sha256':sha(data),'after_snapshot':dest})
dump(ev/'own-deltas.json',deltas);dump(ev/'protected-manifest.json',protected);dump(ev/'owner-checks.json',checks)
for p in sorted(scratch.iterdir()):
 if p.is_file():put(ev/'commands'/p.name,p.read_bytes())
record={'status':'IMPLEMENTED_BOUNDED_VERIFICATION_WITH_BASELINE_QUALITY_BLOCKERS','final_tests':137,'results':summary,'protected_files':len(protected),'historical_files':len(history),'owner_checks':len(checks),'test_strict':'PASS','workspace_check':'PASS with baseline warnings','production_strict':'FAIL baseline SPACING_12 / DecodedMedia.animated','scoped_fmt_diff':'PASS/PASS','workspace_fmt':'FAIL four baseline-identical files','source_gates':'9 PASS / QuickLaunch FAIL baseline QL-11 literal CONTROL_HEIGHT_INPUT at line458; line1765 not reached','full_desktop_native_release':'NOT RUN'}
dump(ev/'verification-summary.json',record)
print(json.dumps(record,indent=2))
