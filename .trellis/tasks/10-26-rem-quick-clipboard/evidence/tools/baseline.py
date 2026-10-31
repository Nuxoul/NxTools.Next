from pathlib import Path
import hashlib, json, datetime, subprocess
root=Path(r'D:\Code\NxTools.Next-main')
scratch=Path(r'C:\Users\Administrator\.pi-desktop\scratch\7faedce5-fc3c-4ab5-bf6d-95910fa4f600\rem-quick-clipboard')
task=root/'.trellis/tasks/10-26-rem-quick-clipboard'
assert not scratch.exists() and not task.exists(), 'Baseline already exists; never overwrite'
scratch.mkdir(parents=True); (task/'evidence/baseline').mkdir(parents=True)
def sha(data): return hashlib.sha256(data).hexdigest()
paths=set((root/'rust/apps/nxtools-desktop/src').rglob('*.rs'))
for s in ['AGENTS.md','Cargo.toml','Cargo.lock','docs/reference-code/clipboard.md','docs/reference-code/quick-launch-research.md','docs/reference-code/gpui.md','docs/reference-code/markdown-editor.md','docs/reference-code/gpui-kit-080.md','docs/reference-code/gpui-kit-component-table.md','docs/reference-code/gpui-kit-component-index.md']:
 paths.add(root/s)
paths.update((root/'.trellis/spec').rglob('*.md'))
records=[]
for p in sorted(paths):
 data=p.read_bytes(); rel=p.relative_to(root).as_posix()
 dest=task/'evidence/baseline'/rel; dest.parent.mkdir(parents=True, exist_ok=True); dest.write_bytes(data)
 records.append({'path':rel,'sha256':sha(data),'bytes':len(data),'snapshot':dest.relative_to(task).as_posix()})
history=[]
for d in sorted((root/'.trellis/tasks').iterdir()):
 if d==task or not d.is_dir(): continue
 if d.name.startswith(('10-24-download-enter','10-25-archive-prompt','10-25-archive-settings','10-25-button-accessible','10-25-clipboard-stable','10-25-default-submit','10-25-download-picker','10-25-download-sort-keyboard','10-25-extensions-compile','10-25-rem-','10-26-extension-preview','10-26-rem-')):
  for p in sorted(d.rglob('*')):
   if p.is_file():
    data=p.read_bytes(); history.append({'path':p.relative_to(root).as_posix(),'sha256':sha(data),'bytes':len(data)})
created=datetime.datetime.now(datetime.timezone.utc).isoformat()
for name,obj in [('baseline-manifest.json',{'created_utc':created,'basis':'Current live bytes, not HEAD. No Queue09 production changes.','files':records}),('historical-manifest.json',{'created_utc':created,'files':history})]:
 (task/'evidence'/name).write_text(json.dumps(obj,indent=2)+'\n',encoding='utf-8')
status=subprocess.run(['git','status','--short','--untracked-files=normal'],cwd=root,capture_output=True,check=True).stdout
(task/'evidence/task-start-git-status.txt').write_bytes(status)
print(json.dumps({'created_utc':created,'task':str(task),'scratch':str(scratch),'live_files':len(records),'frozen_history_files':len(history),'baseline_manifest_sha256':sha((task/'evidence/baseline-manifest.json').read_bytes()),'history_manifest_sha256':sha((task/'evidence/historical-manifest.json').read_bytes())},indent=2))
