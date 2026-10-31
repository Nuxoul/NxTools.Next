from pathlib import Path
import json,hashlib,base64,difflib,re,datetime,subprocess,ast
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-command-naming';evidence=task/'evidence'
scratch=Path(r'C:\Users\Administrator\.pi-desktop\scratch\7faedce5-fc3c-4ab5-bf6d-95910fa4f600\command-naming')
def sha(data):return hashlib.sha256(data).hexdigest()
def load(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def save(rel,obj):
 p=evidence/rel;assert not p.exists(),f'Immutable evidence already exists: {p}'
 p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(obj,indent=2)+'\n',encoding='utf-8')
def copy(source,rel):
 p=evidence/rel;assert not p.exists(),f'Immutable evidence already exists: {p}'
 p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(source.read_bytes())
assert not (evidence/'manifest.json').exists(),'Never overwrite manifest'
inspection=load(scratch/'owner-inspection-final.json')
baseline=load(evidence/'baseline-manifest.json');by={r['path']:r for r in baseline['files']}
history=load(evidence/'historical-manifest.json')
for rec in history['files']:
 assert (root/rec['path']).is_file() and sha((root/rec['path']).read_bytes())==rec['sha256'],f'Historical movement: {rec["path"]}'
for rec in inspection['protected_unchanged']:
 assert sha((root/rec['path']).read_bytes())==rec['sha256'],f'Protected movement: {rec["path"]}'
# Recompute the exact inverse presentation/callback/prefix checks before snapshots.
source=(scratch/'inspect.py').read_text(encoding='utf-8');read_only=source.split('\npath=scratch/')[0]
ns={};exec(compile(ast.parse(read_only),'<publish-owner-guard>','exec'),ns)
assert len(ns['checks'])==17
for p in sorted(scratch.rglob('*')):
 if not p.is_file():continue
 rel=p.relative_to(scratch)
 target=Path('logs')/rel if p.suffix=='.log' else Path('tools')/rel if p.suffix in ['.py','.ps1','.rs'] else Path('observations')/rel
 copy(p,target)
copy(scratch.parent/'queue12-baseline.py',Path('tools/baseline.py'))
# Stable convenient result paths, separate from immutable raw originals.
copy(scratch/'owner-inspection-final.json',Path('owner-inspection-final.json'))
copy(scratch/'final-verification-results.json',Path('final-verification-results.json'))
gates=load(scratch/'source-gate-results.json')
for rec in gates:rec['evidence_log']='evidence/logs/'+Path(rec['log']).name
save('source-gate-results.json',gates)
existing=[];new=[]
for rel in inspection['owned_existing']:
 rec=by[rel];before=(task/rec['snapshot']).read_bytes();after=(root/rel).read_bytes()
 current=Path('current')/rel;dest=evidence/current;assert not dest.exists();dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(after)
 a=before.splitlines(keepends=True);b=after.splitlines(keepends=True);ops=[]
 for tag,i1,i2,j1,j2 in difflib.SequenceMatcher(a=a,b=b,autojunk=False).get_opcodes():
  ops.append({'kind':tag,'i1':i1,'i2':i2,'j1':j1,'j2':j2,'before_b64':base64.b64encode(b''.join(a[i1:i2])).decode(),'after_b64':base64.b64encode(b''.join(b[j1:j2])).decode()})
 diff=''.join(difflib.unified_diff(before.decode('utf-8-sig').splitlines(keepends=True),after.decode('utf-8-sig').splitlines(keepends=True),fromfile='live-task-start/'+rel,tofile='own-delivery/'+rel))
 diff_path=Path('own-diffs')/(rel.replace('/','__')+'.txt');dest=evidence/diff_path;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_text(diff,encoding='utf-8')
 existing.append({'path':rel,'before_snapshot':rec['snapshot'],'after_snapshot':'evidence/'+current.as_posix(),'before_sha256':sha(before),'after_sha256':sha(after),'diff':'evidence/'+diff_path.as_posix(),'ops':ops})
for rel in inspection['new_owned']:
 current=Path('current')/rel;dest=evidence/current;assert not dest.exists();dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes((root/rel).read_bytes())
 new.append({'path':rel,'snapshot':'evidence/'+current.as_posix(),'sha256':sha(dest.read_bytes())})
save('own-deltas.json',{'basis':'Exact live dirty task-start bytes, never HEAD; equal and changed opcode spans replay both directions','existing':existing,'new':new})
external=[]
for rec in inspection['independent_changed_observations']:
 p=root/rec['path'];observed=p.read_bytes();rel=Path('independent-observations')/rec['path'];dest=evidence/rel;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(observed)
 external.append({**rec,'final_observed_sha256':sha(observed),'snapshot':'evidence/'+rel.as_posix(),'observed_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'ownership':'Independent, not edited or credited by audit08; observation only'})
save('independent-observations.json',external)
original_paths=set(by);new_sources=[]
for p in sorted((root/'rust/apps/nxtools-desktop/src').rglob('*.rs')):
 rel=p.relative_to(root).as_posix()
 if rel in original_paths or rel in inspection['new_owned']:continue
 dest=evidence/'independent-new-sources'/rel;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(p.read_bytes())
 new_sources.append({'path':rel,'sha256':sha(p.read_bytes()),'snapshot':dest.relative_to(task).as_posix(),'ownership':'Independent new source observation, not owned'})
save('independent-new-sources.json',new_sources)
compiled=[]
for p in sorted(scratch.glob('*.json')):
 try:rec=load(p)
 except (ValueError,UnicodeError):continue
 if not isinstance(rec,dict) or 'command' not in rec or 'exit' not in rec or 'log' not in rec:continue
 log=Path(rec['log']);s=log.read_text(encoding='utf-8',errors='replace')
 compiled.append({**rec,'record':p.name,'evidence_log':'evidence/logs/'+log.name,'log_sha256':sha(log.read_bytes()),'summaries':re.findall(r'test result:.*',s),'error_headings':re.findall(r'^error(?:\[[^\]]+\])?:.*',s,re.M)})
save('compiled-results.json',compiled)
s=(scratch/'downloads-final.log').read_text(encoding='utf-8');m=(scratch/'main-window-related.log').read_text(encoding='utf-8')
keyboard=re.findall(r'^test downloads::new_download_view::keyboard_tests::.* \.\.\. ok$',s,re.M)
picker=re.findall(r'^test downloads::directory_picker_tests::.* \.\.\. ok$',s,re.M)
assert len(keyboard)==8 and len(picker)==14
save('protected-subsets.json',{'keyboard8':len(keyboard),'picker14':len(picker),'keyboard_names':keyboard,'picker_names':picker,'tool_no_bubble_pass':'test main_window::tests::tool_card_toggles_and_configure_opens_without_toggling ... ok' in m,'floating_pass':re.findall(r'^test main_window::floating_assistant_tests::.* \.\.\. ok$',m,re.M),'currency_pass':re.findall(r'^test main_window::currency_tests::.* \.\.\. ok$',m,re.M),'scope':'Extracted actual successful lines; broad MainWindow remains failed, no whole-suite claim'})
# Explicit blockers current snapshots, no mutation/allow/bypass.
blockers=[]
for rel in ['rust/apps/nxtools-desktop/src/extensions/storage_management.rs','rust/apps/nxtools-desktop/src/tools/floating_assistant/media.rs','rust/apps/nxtools-desktop/src/main_window.rs']:
 current=(root/rel).read_bytes();before=(task/by[rel]['snapshot']).read_bytes()
 blockers.append({'path':rel,'baseline_sha256':sha(before),'current_sha256':sha(current),'whole_file_baseline_identical':current==before,'note':'main_window.rs owns only cfg(test) module append; its unused SPACING_12 import is exact task-start source. Original production import was not modified.' if rel.endswith('/main_window.rs') else 'No task edit; independent/current or task-start-identical blocker, not suppressed'})
save('verification-blockers.json',{'strict':blockers,'main_window_failed':'206 PASS /6 FAIL resource-limit admission in independent Extensions storage fixtures, retained original log','workspace_fmt_failed_files':list(dict.fromkeys(re.findall(r'Diff in ([^\n]+)',(scratch/'workspace-fmt-final.log').read_text(encoding='utf-8',errors='replace'))))})
refs=[Path(r'D:\Code\ab-download-manager-master\desktop\app\src\main\kotlin\com\abdownloadmanager\desktop\cli\download\remove\RemoveDownload.kt'),Path(r'D:\Code\NxTools\Views\MainWindow.axaml.cs'),Path(r'D:\Code\NxTools\Views\SettingsWindow.axaml.cs'),Path(r'D:\Code\NxTools\Services\ClipboardHistoryService.cs'),root/'csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.cs',root/'csharp/NxTools.PlatformHost/Windows/BrowserExtensionExportDialog.cs',root/'rust/crates/nxtools-download/src/engine.rs']
save('reference-source-observations.json',{'level':'Read-only source observation, not native/product acceptance; reference-order limitation in outcomes','files':[{'path':str(p),'sha256':sha(p.read_bytes()),'bytes':p.stat().st_size} for p in refs]})
checks=[]
for rel in ['prd.md','command-matrix.md','outcomes.md','evidence/verify.py']:
 text=(task/rel).read_text(encoding='utf-8');trailing=[i for i,line in enumerate(text.splitlines(),1) if line.rstrip(' \t')!=line]
 assert not trailing and text.endswith('\n') and not text.endswith('\n\n'),f'Task text whitespace: {rel}'
 checks.append({'path':rel,'trailing_whitespace_lines':trailing,'one_final_newline':True})
ast.parse((evidence/'verify.py').read_text(encoding='utf-8'))
proc=subprocess.run(['python','tests/architecture/spec-contracts.py'],cwd=root,capture_output=True)
assert proc.returncode==0
(evidence/'spec-final.log').write_bytes(proc.stdout+proc.stderr)
save('task-checks.json',{'exit':0,'checks':checks,'verify_python_ast':'PASS','final_specs_exit':proc.returncode,'scope':'Task document whitespace/AST/navigation only, not product/native acceptance'})
save('noncargo-preparation-attempts.json',{'items':['Edit old_string payload rejected ops required, no write; corrected line ops','KeyDownEvent prefer_character_input compile error retained in Cargo log','TestSupportExt missing import compile error retained in Cargo log','New fixture duplicate line corrected before compilation, no product delta','DefaultGBK read failed after old-prefix byte check already returned true; UTF8 correction passed','finish-download-checks.ps1 inline Python/PowerShell failed before production formatting/test; separate check-polish-prefix.py and resume succeeded','Several early runner calls rejected natural-idle admission before Cargo/log creation; bounded admission observations saved for later commands'],'level':'Preparation/tooling errors, never behavioral reds'})
# Freeze live bytes once more after copy, so concurrent own-path writes cannot silently publish.
for rec in existing:assert sha((root/rec['path']).read_bytes())==rec['after_sha256'],f'Own target moved during publication: {rec["path"]}'
for rec in new:assert sha((root/rec['path']).read_bytes())==rec['sha256']
for rec in history['files']:assert sha((root/rec['path']).read_bytes())==rec['sha256']
artifacts=[{'path':p.relative_to(task).as_posix(),'sha256':sha(p.read_bytes()),'bytes':p.stat().st_size} for p in sorted(task.rglob('*')) if p.is_file()]
save('manifest.json',{'created_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'IMPLEMENTED_BOUNDED_VERIFICATION_BLOCKED_NOT_GLOBAL_AUDIT08_CLOSED','basis':'Task-start live dirty bytes, not HEAD','owned_existing':16,'new_owned':1,'protected_unchanged':len(inspection['protected_unchanged']),'history_files':len(history['files']),'artifacts':artifacts})
print(json.dumps({'manifest_sha256':sha((evidence/'manifest.json').read_bytes()),'artifacts':len(artifacts),'deltas':len(existing),'new':len(new),'protected_unchanged':len(inspection['protected_unchanged']),'history_files':len(history['files']),'status':'IMPLEMENTED_BOUNDED_VERIFICATION_BLOCKED_NOT_GLOBAL_AUDIT08_CLOSED'},indent=2))
