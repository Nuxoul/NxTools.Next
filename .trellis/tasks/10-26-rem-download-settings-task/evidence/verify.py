"""Read-only verification for Queue06 frozen evidence and dirty-baseline replay."""
from pathlib import Path
import hashlib,json,subprocess,sys
T=Path(__file__).resolve().parents[1];R=T.parents[2]
sha=lambda b:hashlib.sha256(b).hexdigest()
load=lambda p:json.loads(p.read_text(encoding='utf-8-sig'))
manifest=load(T/'evidence/manifest.json')
for row in manifest['evidence']:
 data=(T/row['path']).read_bytes();assert len(data)==row['bytes'] and sha(data)==row['sha256'],row['path']
for row in manifest['sources']+manifest['documents']:
 assert sha((R/row['path']).read_bytes())==row['sha256'],row['path']
for row in load(T/'evidence/own-deltas.json'):
 after=(T/row['snapshot']).read_bytes();assert sha(after)==row['after_sha256']
 assert (R/row['path']).read_bytes()==after
 if row['kind']=='new':continue
 before=(T/row['baseline']).read_bytes();assert sha(before)==row['baseline_sha256']
 forward=before.decode('utf8').splitlines(keepends=True);reverse=after.decode('utf8').splitlines(keepends=True)
 for op in reversed(row['operations']):
  i,j,k,l=op['before_start'],op['before_end'],op['after_start'],op['after_end']
  assert forward[i:j]==op['before_lines'] and reverse[k:l]==op['after_lines']
  forward[i:j]=op['after_lines'];reverse[k:l]=op['before_lines']
 assert ''.join(forward).encode('utf8')==after
 assert ''.join(reverse).encode('utf8')==before
result=subprocess.run([sys.executable,str(T/'evidence/inspect.py')],cwd=R,capture_output=True,text=True)
assert result.returncode==0,result.stderr
print(result.stdout.splitlines()[-1])
print(f'PASS {len(manifest["evidence"])} evidence hashes,4live sources,3forward/reverse dirty deltas and new-test snapshot; external workspace fmt failure retained; no files written')
