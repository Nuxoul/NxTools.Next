from pathlib import Path
import hashlib,json,difflib
root=Path(r'D:\Code\NxTools.Next-main');task=root/'.trellis/tasks/10-26-download-detail-async';scratch=Path(__file__).parent
base=json.loads((task/'evidence/baseline-manifest.json').read_text())['files']
changed=[]
for rec in base:
 p=root/rec['path'];b=(task/rec['snapshot']).read_bytes();a=p.read_bytes()
 if a!=b:
  delta=''.join(difflib.unified_diff(b.decode('utf-8').splitlines(True),a.decode('utf-8').splitlines(True),fromfile='dirty-baseline/'+rec['path'],tofile='current/'+rec['path']))
  (scratch/(rec['path'].replace('/','__')+'.diff.txt')).write_text(delta,encoding='utf-8')
  changed.append({'path':rec['path'],'before_sha256':hashlib.sha256(b).hexdigest(),'after_sha256':hashlib.sha256(a).hexdigest(),'diff_lines':len(delta.splitlines())})
print(json.dumps(changed,indent=2))
