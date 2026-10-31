from pathlib import Path
import subprocess,json,hashlib,datetime
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-gpui-guides-final-handoff';ev=task/'evidence'
assert not task.exists()
reports=['docs/audits/2026-10-26-gpui-guides-final-review.md','docs/audits/2026-10-26-gpui-guides-summary-zh.md']
assert all(not (root/p).exists()for p in reports)
paths=subprocess.run(['git','ls-files','-z','--cached','--others','--exclude-standard'],cwd=root,capture_output=True,check=True).stdout.decode('utf-8').split('\0')
files=[]
for rel in sorted(set(p for p in paths if p)):
 p=root/rel
 if p.is_file():files.append({'path':rel,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
ev.mkdir(parents=True)
(ev/'workspace-baseline.json').write_text(json.dumps({'created_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'basis':'current Git-visible regular files, including existing dirty/untracked files; no HEAD rollback basis','excluded':'Git-ignored outputs, directories and external repositories are not claimed covered','files':files},indent=2)+'\n',encoding='utf-8')
(ev/'git-status-before.txt').write_bytes(subprocess.run(['git','status','--short'],cwd=root,capture_output=True,check=True).stdout)
(ev/'git-diff-names-before.txt').write_bytes(subprocess.run(['git','diff','--name-only'],cwd=root,capture_output=True,check=True).stdout)
(ev/'baseline.py').write_bytes(Path(__file__).read_bytes())
print(json.dumps({'protected_files':len(files),'bytes':sum(x['bytes']for x in files),'planned_new_reports':reports},indent=2))
