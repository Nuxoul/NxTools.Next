from pathlib import Path
import base64,datetime,difflib,hashlib,json,re
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-download-detail-async';ev=task/'evidence';scratch=Path(__file__).parent
owned=['docs/reference-code/download.md','rust/apps/nxtools-desktop/Cargo.toml','rust/apps/nxtools-desktop/src/downloads.rs','rust/apps/nxtools-desktop/src/main_window.rs','rust/apps/nxtools-desktop/src/main_window/download_page.rs','rust/apps/nxtools-desktop/src/main_window/shell.rs','rust/crates/nxtools-download/Cargo.toml','rust/crates/nxtools-download/src/engine/projection.rs','rust/crates/nxtools-download/src/storage.rs']
new=['rust/apps/nxtools-desktop/src/downloads/detail_request.rs','rust/apps/nxtools-desktop/src/downloads/detail_request_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_page/detail_async_tests.rs']
assert not (ev/'own-deltas.json').exists(),'Never overwrite published evidence'
def sha(data):return hashlib.sha256(data).hexdigest()
def put(path,data):
 assert not path.exists(),str(path)
 path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
def dump(path,obj):put(path,(json.dumps(obj,indent=2)+'\n').encode())
base=json.loads((ev/'baseline-manifest.json').read_text())['files'];by={x['path']:x for x in base};history=json.loads((ev/'historical-manifest.json').read_text())['files']
# Validate before publishing any source delta.
protected=[]
for x in base:
 if x['path'] not in owned:
  assert sha((root/x['path']).read_bytes())==x['sha256'],'Unexpected live movement '+x['path']
  protected.append(x)
for x in history:assert sha((root/x['path']).read_bytes())==x['sha256'],'Historical movement '+x['path']
checks=[]
def same(name,a,b):assert a==b,name;checks.append({'name':name,'result':'PASS','sha256':sha(a.encode())})
def old(rel):return (task/by[rel]['snapshot']).read_text(encoding='utf-8')
def current(rel):return (root/rel).read_text(encoding='utf-8')
def region(text,start,end):return text.split(start,1)[1].split(end,1)[0]
p='rust/apps/nxtools-desktop/src/main_window/download_page.rs';a=old(p);b=current(p)
same('audit11 projection/filter/sort/selection/capabilities unchanged',a.split('impl MainWindow {',1)[0],b.split('impl MainWindow {',1)[0])
same('audit11 virtual range snapshot and resource implementation unchanged',region(a,'        let visible_count = visible_tasks.len();','        let filter_width ='),region(b,'        let visible_count = visible_tasks.len();','        let filter_width ='))
same('detail renderer/masking/context/batch/headers/persistence unchanged',a.split('fn download_aggregate_progress(',1)[1].split('\n#[cfg(test)]\n#[path = "download_page/kit_migration_tests.rs"]',1)[0],b.split('fn download_aggregate_progress(',1)[1].split('\n#[cfg(test)]\n#[path = "download_page/kit_migration_tests.rs"]',1)[0])
same('owned row display/paint/callbacks except explicit scope retirement',region(a,'    let DownloadRowSelection {','fn download_aggregate_progress('),region(b,'    let DownloadRowSelection {','fn download_aggregate_progress(').replace('''                if window
                    .download_detail_scope
                    .as_ref()
                    .is_some_and(|scope| Some(scope.task_id()) != window.selected_download_task)
                {
                    window.retire_download_detail_scope(cx);
                }
''',''))
p='rust/crates/nxtools-download/src/engine/projection.rs';a=old(p);b=current(p)
same('engine original task/snapshot code unchanged',a.split('    pub(super) fn command_task_detail(',1)[0],b.split('    /// Uses the existing runtime and original TaskDetail command/store chain.',1)[0])
same('engine command_task_detail storage/diagnostics unchanged',a.split('    pub(super) fn command_task_detail(',1)[1],b.split('    pub(super) fn command_task_detail(',1)[1])
p='rust/crates/nxtools-download/src/storage.rs';a=old(p);b=current(p)
same('store queries/schema/lock implementation unchanged',a.split('    pub fn open(path:',1)[1],b.split('    pub fn open(path:',1)[1])
assert current('docs/reference-code/download.md').startswith(old('docs/reference-code/download.md'));checks.append({'name':'Download reference exact baseline prefix','result':'PASS'})
# Native close/hide policy remains the same match body; only request scope retirement added.
p='rust/apps/nxtools-desktop/src/main_window/shell.rs';a=old(p);b=current(p)
compact=lambda s:re.sub(r'\s+','',s)
same('native close/tray/minimize/maximize policy unchanged',compact(region(a,'move |_, window, cx| match control {','        })')),compact(region(b,'            match control {','        })')[:-14] if False else region(b,'            match control {','\n            }\n        })')))
for rel in ['rust/apps/nxtools-desktop/src/downloads/window_support.rs','rust/apps/nxtools-desktop/src/downloads/artifact_actions.rs','rust/apps/nxtools-desktop/src/downloads/task_windows_impl.rs','rust/apps/nxtools-desktop/src/downloads/completion_shutdown_impl.rs','rust/apps/nxtools-desktop/src/main_window/download_page/projection_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_page/kit_migration_tests.rs','rust/apps/nxtools-desktop/src/main_window/tests.rs','rust/crates/nxtools-download/src/engine.rs','rust/crates/nxtools-download/src/lib.rs','rust/crates/nxtools-download/src/message.rs']:
 assert sha((root/rel).read_bytes())==by[rel]['sha256'];checks.append({'name':'unchanged protected owner '+rel,'result':'PASS','sha256':by[rel]['sha256']})
# Exact source snapshots and line-byte forward/inverse deltas; never HEAD.
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
 delta=''.join(difflib.unified_diff(before.decode('utf-8').splitlines(True),after.decode('utf-8').splitlines(True),fromfile='live-dirty-baseline/'+rel,tofile='own-current/'+rel));put(ev/'deltas'/(rel.replace('/','__')+'.txt'),delta.encode())
 deltas.append({'path':rel,'before_sha256':sha(before),'after_sha256':sha(after),'after_snapshot':dest,'ops':ops})
for rel in new:
 data=(root/rel).read_bytes();dest='evidence/current/'+rel;put(task/dest,data);deltas.append({'path':rel,'new_file':True,'after_sha256':sha(data),'after_snapshot':dest})
dump(ev/'own-deltas.json',deltas);dump(ev/'protected-manifest.json',protected);dump(ev/'owner-checks.json',checks)
# Immutable original attempts/logs/scripts.
for path in sorted(scratch.iterdir()):
 if path.is_file():put(ev/'commands'/path.name,path.read_bytes())
summary={}
for name,count in [('audit12-store-lock-red-no-pdb',None),('audit12-refresh-cache-red',None),('audit12-final-formatted',9),('final-download-page',29),('download-main-related-01',27),('downloads-lifecycle-no-network',48),('final-names-identity',18),('engine-task-detail-contract',1)]:
 text=(scratch/(name+'.log')).read_text(encoding='utf-8',errors='replace');lines=re.findall(r'test result:.*',text);assert lines,name;summary[name]=lines[-1]
 if count:assert f'{count} passed; 0 failed; 0 ignored' in lines[-1],name
assert 'next_ui_turn_before_watchdog=false watchdog_unlocked=true' in (scratch/'audit12-store-lock-red-no-pdb.log').read_text(encoding='utf-8')
assert 'next_ui_turn_before_watchdog=true watchdog_unlocked=false' in (scratch/'audit12-final-formatted.log').read_text(encoding='utf-8')
for name in ['strict-desktop-tests-final','strict-download-tests','strict-download-production','workspace-check-final']:
 assert json.loads((scratch/(name+'.json')).read_text(encoding='utf-8-sig'))['exit']==0,name
for name in ['strict-desktop-production-final','workspace-fmt-final']:
 assert json.loads((scratch/(name+'.json')).read_text(encoding='utf-8-sig'))['exit']!=0,name
gates=json.loads((scratch/'source-gate-results.json').read_text(encoding='utf-8-sig'));assert len(gates)==10 and all(x['exit']==0 for x in gates)
scoped=json.loads((scratch/'scoped-checks-final.json').read_text(encoding='utf-8-sig'));assert scoped['fmt']==scoped['diff']==0
new_text=''.join(current(rel) for rel in new);assert '#[ignore' not in new_text and 'allow(clippy' not in new_text and 'DEBUG-' not in new_text
record={'status':'IMPLEMENTED_BOUNDED_VERIFICATION_WITH_BASELINE_QUALITY_BLOCKERS','results':summary,'protected_files':len(protected),'historical_files':len(history),'owner_checks':len(checks),'test_strict':'PASS','download_strict_tests_production':'PASS/PASS','workspace_check':'PASS','production_desktop_strict':'FAIL baseline SPACING_12 / DecodedMedia.animated','workspace_fmt':'FAIL four baseline-identical files','source_gates':'10 PASS','network_full_desktop_native_release':'NOT RUN'}
dump(ev/'verification-summary.json',record)
print(json.dumps(record,indent=2))
