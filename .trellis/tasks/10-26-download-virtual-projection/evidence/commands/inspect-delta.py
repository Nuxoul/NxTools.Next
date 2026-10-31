from pathlib import Path
import difflib,json
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-download-virtual-projection';scratch=Path(__file__).parent
recs=json.loads((task/'evidence/baseline-manifest.json').read_text())['files']
for rec in recs:
 p=root/rec['path']; old=(task/rec['snapshot']).read_text(encoding='utf-8');new=p.read_text(encoding='utf-8')
 if old!=new:
  delta=''.join(difflib.unified_diff(old.splitlines(True),new.splitlines(True),fromfile='baseline/'+rec['path'],tofile='current/'+rec['path']))
  dest=scratch/(rec['path'].replace('/','__')+'.diff.txt');dest.write_text(delta,encoding='utf-8');print(rec['path'],len(delta.splitlines()))
