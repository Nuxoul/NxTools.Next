from pathlib import Path
import json,hashlib,datetime,subprocess
root=Path(r'D:\Code\NxTools.Next-main'); task=root/'.trellis/tasks/10-26-rem-quick-clipboard'; evidence=task/'evidence'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def load(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def save(name,obj):
 p=evidence/name; assert not p.exists(),f'Immutable file exists: {p}'
 p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(obj,indent=2)+'\n',encoding='utf-8')
b=load(evidence/'baseline-manifest.json');h=load(evidence/'historical-manifest.json')
for records in [b['files'],h['files']]:
 for rec in records:assert digest(root/rec['path'])==rec['sha256'],f'Independent movement: {rec["path"]}'
checks=[]
for rel in ['prd.md','inventory.md','outcomes.md','evidence/verify.py']:
 p=task/rel;proc=subprocess.run(['git','diff','--no-index','--check','--','NUL',str(p)],cwd=root,capture_output=True)
 assert proc.returncode in [0,1] and not proc.stdout and not proc.stderr, 'Unexpected Git diagnostic'
 data=p.read_text(encoding='utf-8');errors=[i for i,line in enumerate(data.splitlines(),1) if line.rstrip(' \t')!=line]
 assert not errors and data.endswith('\n') and not data.endswith('\n\n'), f'Whitespace error: {rel}'
 checks.append({'file':rel,'git_raw_exit':proc.returncode,'git_stdout':'','git_stderr':'','trailing_whitespace_lines':errors,'one_final_newline':True})
save('task-whitespace.json',{'scope':'Four new task/verifier text files only; not Rust formatting','exit':0,'method':'Explicit trailing-whitespace/final-newline scan plus Git no-index diagnostics; raw difference exits retained','files':checks})
paths=[Path(r'D:\Code\NxTools\Views\ClipboardHistoryPopupWindow.axaml.cs'),Path(r'D:\Code\NxTools\Views\ClipboardPreviewWindow.cs'),Path(r'D:\Code\NxTools\Controls\ClipboardHistoryListView.cs'),Path(r'D:\Code\SwiftList-main\App\Views\QuickSearchWindow\Helpers\QuickSearchWindowInputHandler.cs'),Path(r'D:\Code\SwiftList-main\App\Services\QuickLookManager.cs'),root/'.diagnostics/listary-managed/Listary/Main/Views/SearchWindowBase.cs',root/'csharp/NxTools.PlatformHost/Windows/WindowsPlatformAdapter.NativeMethods.cs',root/'vendor/gpui-component-080/src/list/list_item.rs']
save('reference-source-observations.json',{'level':'Read-only source observations, not task-start baselines or native acceptance','files':[{'path':str(p),'sha256':digest(p),'bytes':p.stat().st_size} for p in paths]})
save('verification-status.json',{'effective_root_red':'NOT_RUN','production_implementation':'NOT_IMPLEMENTED','scoped_quick_clipboard_ui_tests':'NOT_RUN','accepted_first_seven_protection_tests':'NOT_RUN','strict_production_clippy':'NOT_RUN','strict_test_clippy':'NOT_RUN','workspace_check':'NOT_RUN','scoped_rustfmt':'NOT_RUN','workspace_rustfmt':'NOT_RUN','source_document_gates':'11/11 PASS','native_physical_visual_user_release':'NOT_RUN / NOT_AUTHORIZED','rust_invocations':0})
dest=evidence/'tools/finalize.py';assert not dest.exists();dest.write_bytes(Path(__file__).read_bytes())
artifacts=[{'path':p.relative_to(task).as_posix(),'sha256':digest(p),'bytes':p.stat().st_size} for p in sorted(task.rglob('*')) if p.is_file()]
save('manifest.json',{'created_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'status':'INCOMPLETE_BLOCKED_BEFORE_EFFECTIVE_RED','basis':'Current live dirty bytes, not HEAD','rust_invocations':0,'production_changes':[],'baseline_files':len(b['files']),'historical_files':len(h['files']),'source_document_gates':11,'artifacts':artifacts})
print(json.dumps({'manifest_sha256':digest(evidence/'manifest.json'),'artifacts':len(artifacts),'baseline_files':len(b['files']),'historical_files':len(h['files']),'status':'INCOMPLETE_BLOCKED_BEFORE_EFFECTIVE_RED'},indent=2))
