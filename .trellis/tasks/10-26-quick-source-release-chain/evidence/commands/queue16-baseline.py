from pathlib import Path
import datetime,hashlib,json,subprocess
r=Path(r'D:\Code\NxTools.Next-main');t=r/'.trellis/tasks/10-26-quick-source-release-chain';s=Path(r'C:\Users\Administrator\.pi-desktop\scratch\7faedce5-fc3c-4ab5-bf6d-95910fa4f600\quick-source-release-chain')
assert not t.exists() and not s.exists();s.mkdir(parents=True);ev=t/'evidence';(ev/'baseline').mkdir(parents=True)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
paths=set()
for folder,pattern in [('scripts','*.ps1'),('tests/architecture','*'),('csharp/NxTools.PlatformHost','*.cs'),('rust/apps/nxtools-indexer/src','*.rs'),('rust/apps/nxtools-desktop/src','*.rs'),('rust/crates/nxtools-ipc/src','*.rs'),('rust/crates/nxtools-protocol/src','*.rs'),('.trellis/spec','*.md')]:
 for p in (r/folder).rglob(pattern):
  if p.is_file() and not any(x in {'bin','obj','__pycache__'} for x in p.parts):paths.add(p)
for p in ['AGENTS.md','CONTEXT.md','docs/reference-code/quick-launch-research.md','docs/reference-code/system-shell.md','Cargo.toml','Cargo.lock']:paths.add(r/p)
files=[]
for p in sorted(paths):
 rel=p.relative_to(r).as_posix();dest=ev/'baseline'/rel;dest.parent.mkdir(parents=True,exist_ok=True);dest.write_bytes(p.read_bytes());files.append({'path':rel,'sha256':sha(p),'snapshot':dest.relative_to(t).as_posix()})
h=[]
for folder in (r/'.trellis/tasks').iterdir():
 if folder==t or not folder.is_dir():continue
 if folder.name.startswith(('10-24-download-enter','10-25-archive-prompt','10-25-archive-settings','10-25-button-accessible','10-25-clipboard-stable','10-25-default-submit','10-25-download-picker','10-25-download-sort-keyboard','10-25-extensions-compile','10-25-rem-','10-26-extension-preview','10-26-rem-','10-26-cursor-semantics','10-26-command-naming','10-26-download-virtual-projection','10-26-download-detail-async','10-26-quick-retained-text')):
  h.extend({'path':p.relative_to(r).as_posix(),'sha256':sha(p)}for p in folder.rglob('*')if p.is_file())
now=datetime.datetime.now(datetime.timezone.utc).isoformat()
for name,obj in [('baseline-manifest.json',{'created_utc':now,'basis':'live dirty bytes, not HEAD','files':files}),('historical-manifest.json',{'created_utc':now,'files':h})]: (ev/name).write_text(json.dumps(obj,indent=2)+'\n',encoding='utf-8')
(ev/'task-start-git-status.txt').write_bytes(subprocess.run(['git','status','--short'],cwd=r,capture_output=True,check=True).stdout)
print(json.dumps({'created_utc':now,'files':len(files),'history':len(h)},indent=2))
