"""Independent tiny 7z fixtures/oracles for the bounded risk/readmission slice; no product backend."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--tool',type=Path,required=True)
p.add_argument('--root',type=Path,required=True)
a=p.parse_args(); tool=a.tool.resolve(strict=True); root=a.root.resolve(strict=True)
fixtures=root/'fixtures'; fixtures.mkdir(exist_ok=False)
inputs=fixtures/'inputs'; inputs.mkdir()
payloads={'first.bin':bytes(range(256))*16,'second.bin':b'B'*2048}
for name,data in payloads.items(): (inputs/name).write_bytes(data)
commands=[]
def call(label,args):
    result=subprocess.run([str(tool),*map(str,args)],cwd=inputs,capture_output=True,timeout=20)
    (fixtures/(label+'.stdout')).write_bytes(result.stdout)
    (fixtures/(label+'.stderr')).write_bytes(result.stderr)
    commands.append({'label':label,'exit':result.returncode,'args':list(map(str,args))})
    assert result.returncode==0,(label,result.returncode)
    return result.stdout
for name,solid in [('nonsolid',False),('solid',True)]:
    archive=fixtures/(name+'.7z')
    call(name+'-produce',['a','-t7z',archive,'first.bin','second.bin','-m0=LZMA2','-md=64k','-mmt=off','-mhc=off','-ms='+('on' if solid else 'off'),'-y'])
    listing=call(name+'-list',['l','-slt',archive]).decode('utf-8',errors='replace')
    call(name+'-test',['t',archive])
    for member,data in payloads.items(): assert call(name+'-decode-'+member,['x','-so',archive,member])==data
    assert ('Solid = +' if solid else 'Solid = -') in listing
raw=bytearray((fixtures/'solid.7z').read_bytes()); raw[35]^=0x40
(fixtures/'corrupt-solid.7z').write_bytes(raw)
manifest={'producer':{'path':str(tool),'sha256':hashlib.sha256(tool.read_bytes()).hexdigest()},'literal_payloads':[{'name':n,'bytes':len(d),'sha256':hashlib.sha256(d).hexdigest()} for n,d in payloads.items()],'archives':[{'name':n,'bytes':(fixtures/n).stat().st_size,'sha256':hashlib.sha256((fixtures/n).read_bytes()).hexdigest()} for n in ['nonsolid.7z','solid.7z','corrupt-solid.7z']],'commands':commands,'note':'Corrupt copy mutates only packed byte35 of own independent small solid sample; no bomb or enlarged declaration.'}
(fixtures/'manifest.json').write_text(json.dumps(manifest,indent=2),encoding='utf-8')
print(json.dumps(manifest,indent=2))
