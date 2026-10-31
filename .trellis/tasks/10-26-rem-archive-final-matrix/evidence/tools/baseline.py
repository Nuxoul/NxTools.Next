from pathlib import Path
import hashlib,json,subprocess,datetime
root=Path(r'D:\Code\NxTools.Next-main')
scratch=Path(r'C:\Users\Administrator\.pi-desktop\scratch\7faedce5-fc3c-4ab5-bf6d-95910fa4f600\rem-archive-final-matrix')
task=root/'.trellis/tasks/10-26-rem-archive-final-matrix'
assert not task.exists() and not scratch.exists(), 'Never overwrite prior baseline'
scratch.mkdir(parents=True);(task/'evidence/baseline').mkdir(parents=True)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
paths=set()
for rel in ['rust/apps/nxtools-desktop/src','rust/crates/nxtools-archive','rust/crates/nxtools-protocol/src','csharp/NxTools.PlatformHost/Protocol','csharp/NxTools.PlatformHost/Windows/Archive','.trellis/spec']:
 paths.update(p for p in (root/rel).rglob('*') if p.is_file() and p.suffix in ['.rs','.cs','.md','.toml'])
for rel in ['AGENTS.md','Cargo.toml','Cargo.lock','CONTEXT.md','docs/reference-code/archive.md','docs/reference-code/download.md','docs/reference-code/quick-launch-research.md','docs/reference-code/clipboard.md','docs/reference-code/gpui.md','docs/reference-code/gpui-kit-080.md','docs/reference-code/gpui-kit-component-table.md','docs/reference-code/gpui-kit-component-index.md']:
 paths.add(root/rel)
records=[]
for p in sorted(paths):
 rel=p.relative_to(root).as_posix(); dest=task/'evidence/baseline'/rel;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(p.read_bytes());records.append({'path':rel,'sha256':sha(p),'bytes':p.stat().st_size,'snapshot':dest.relative_to(task).as_posix()})
history=[]
for folder in sorted((root/'.trellis/tasks').iterdir()):
 if not folder.is_dir() or folder==task:continue
 if folder.name=='10-16-archive-manager' or folder.name.startswith(('10-24-download-enter','10-25-archive-prompt','10-25-archive-settings','10-25-button-accessible','10-25-clipboard-stable','10-25-default-submit','10-25-download-picker','10-25-download-sort-keyboard','10-25-extensions-compile','10-25-rem-','10-26-extension-preview','10-26-rem-')):
  history.extend({'path':p.relative_to(root).as_posix(),'sha256':sha(p),'bytes':p.stat().st_size} for p in sorted(folder.rglob('*')) if p.is_file())
created=datetime.datetime.now(datetime.timezone.utc).isoformat()
for name,obj in [('baseline-manifest.json',{'created_utc':created,'basis':'Current live dirty bytes, never HEAD','files':records}),('historical-manifest.json',{'created_utc':created,'files':history})]:
 (task/'evidence'/name).write_text(json.dumps(obj,indent=2)+'\n',encoding='utf-8')
status=subprocess.run(['git','status','--short','--untracked-files=normal'],cwd=root,capture_output=True,check=True)
(task/'evidence/task-start-git-status.txt').write_bytes(status.stdout)
print(json.dumps({'created_utc':created,'task':str(task),'live_files':len(records),'history_files':len(history),'baseline_sha256':sha(task/'evidence/baseline-manifest.json'),'history_sha256':sha(task/'evidence/historical-manifest.json')},indent=2))
