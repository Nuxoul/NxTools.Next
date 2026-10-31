from pathlib import Path
import json,hashlib,sys
root=Path(r'D:\Code\NxTools.Next-main'); task=root/'.trellis/tasks/10-26-download-virtual-projection'
base=json.loads((task/'evidence/baseline-manifest.json').read_text())['files']
owned={'rust/apps/nxtools-desktop/src/downloads.rs','rust/apps/nxtools-desktop/src/main_window.rs','rust/apps/nxtools-desktop/src/main_window/download_page.rs','docs/reference-code/download.md'}
changed=[]
for rec in base:
 p=root/rec['path']
 if hashlib.sha256(p.read_bytes()).hexdigest()!=rec['sha256']:changed.append({'path':rec['path'],'owned':rec['path'] in owned})
print(json.dumps(changed,indent=2))
