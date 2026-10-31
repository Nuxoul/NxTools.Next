from pathlib import Path
import hashlib,json,datetime,subprocess
root=Path(r'D:\Code\NxTools.Next-main')
task=root/'.trellis/tasks/10-26-rem-quick-clipboard'
scratch=Path(r'C:\Users\Administrator\.pi-desktop\scratch\7faedce5-fc3c-4ab5-bf6d-95910fa4f600\rem-quick-clipboard')
evidence=task/'evidence'
assert not (evidence/'manifest.json').exists(), 'Immutable manifest already exists'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def load(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def save(name,obj):
 p=evidence/name;assert not p.exists(),f'Would overwrite {p}'
 p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(obj,indent=2)+'\n',encoding='utf-8')
def copy(src,rel):
 dest=evidence/rel;assert not dest.exists(),f'Would overwrite {dest}'
 dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(src.read_bytes())
b=load(evidence/'baseline-manifest.json'); h=load(evidence/'historical-manifest.json')
for collection in [b['files'],h['files']]:
 for rec in collection:
  assert digest(root/rec['path'])==rec['sha256'],f'Independent movement: {rec["path"]}; do not overwrite'
for source in scratch.glob('*.log'):copy(source,Path('logs')/source.name)
for name in ['observe-idle.ps1','source-gates.ps1','publish.py']:copy(scratch/name,Path('tools')/name)
copy(scratch.parent/'queue09-baseline.py',Path('tools')/'baseline.py')
idle=list(scratch.glob('idle-observation-*.json'));assert len(idle)==1
copy(idle[0],Path('idle-observation.json'))
copy(scratch/'final-process-observation.json',Path('final-process-observation.json'))
gates=load(scratch/'source-gate-results.json')
for rec in gates:
 rec['evidence_log']='evidence/logs/'+Path(rec['log']).name
save('source-gate-results.json',gates)
whitespace=[]
for rel in ['prd.md','inventory.md','outcomes.md','evidence/verify.py']:
 proc=subprocess.run(['git','diff','--no-index','--check','--','NUL',str(task/rel)],cwd=root,capture_output=True)
 whitespace.append({'file':rel,'command':['git','diff','--no-index','--check','--','NUL',str(task/rel)],'exit':proc.returncode,'stdout':proc.stdout.decode('utf-8',errors='replace'),'stderr':proc.stderr.decode('utf-8',errors='replace')})
assert all(r['exit']==0 for r in whitespace),whitespace
save('task-whitespace.json',{'scope':'Only four new human/task/verifier text files; no Rust fmt performed','exit':0,'commands':whitespace})
reference_paths=[Path(r'D:\Code\NxTools\Views\ClipboardHistoryPopupWindow.axaml.cs'),Path(r'D:\Code\NxTools\Views\ClipboardPreviewWindow.cs'),Path(r'D:\Code\NxTools\Controls\ClipboardHistoryListView.cs'),Path(r'D:\Code\SwiftList-main\App\Views\QuickSearchWindow\Helpers\QuickSearchWindowInputHandler.cs'),Path(r'D:\Code\SwiftList-main\App\Services\QuickLookManager.cs'),root/'.diagnostics/listary-managed/Listary/Main/Views/SearchWindowBase.cs',root/'csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.NativeMethods.cs',root/'vendor/gpui-component-080/src/list/list_item.rs']
save('reference-source-observations.json',{'level':'Read-only source observations, not task-start baselines or native acceptance','files':[{'path':str(p),'sha256':digest(p),'bytes':p.stat().st_size} for p in reference_paths]})
save('verification-status.json',{'effective_root_red':'NOT_RUN','production_implementation':'NOT_IMPLEMENTED','scoped_quick_clipboard_ui_tests':'NOT_RUN','accepted_first_seven_protection_tests':'NOT_RUN','strict_production_clippy':'NOT_RUN','strict_test_clippy':'NOT_RUN','workspace_check':'NOT_RUN','scoped_rustfmt':'NOT_RUN','workspace_rustfmt':'NOT_RUN','source_document_gates':'11/11 PASS','native_physical_visual_user_release':'NOT_RUN / NOT_AUTHORIZED','rust_invocations':0})
artifacts=[]
for p in sorted(task.rglob('*')):
 if p.is_file():artifacts.append({'path':p.relative_to(task).as_posix(),'sha256':digest(p),'bytes':p.stat().st_size})
save('manifest.json',{'created_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'INCOMPLETE_BLOCKED_BEFORE_EFFECTIVE_RED','basis':'Current live dirty bytes, not HEAD','rust_invocations':0,'production_changes':[],'baseline_files':len(b['files']),'historical_files':len(h['files']),'source_document_gates':11,'artifacts':artifacts})
print(json.dumps({'manifest_sha256':digest(evidence/'manifest.json'),'artifacts':len(artifacts),'baseline_files':len(b['files']),'historical_files':len(h['files']),'status':'INCOMPLETE_BLOCKED_BEFORE_EFFECTIVE_RED'},indent=2))
