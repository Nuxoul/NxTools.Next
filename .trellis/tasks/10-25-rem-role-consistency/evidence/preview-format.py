from pathlib import Path
import subprocess, difflib, hashlib, json
s=Path(__file__).parent; root=Path.cwd(); results=[]
for path in ['rust/apps/nxtools-desktop/src/ui/metrics.rs','rust/apps/nxtools-desktop/src/downloads/schedule_view.rs','rust/apps/nxtools-desktop/src/downloads/polish_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_page.rs','rust/apps/nxtools-desktop/src/main_window/download_page/kit_migration_tests.rs']:
 p=root/path; b=p.read_bytes(); target=s/(path.replace('/','__')+'.format.rs'); target.write_bytes(b)
 subprocess.run(['rustfmt','--edition','2024','--config','skip_children=true',str(target)],check=True)
 a=b.decode('utf8').splitlines(); f=target.read_text(encoding='utf8').splitlines(); changes=[(i+1,j,k+1,l) for t,i,j,k,l in difflib.SequenceMatcher(None,a,f,autojunk=False).get_opcodes() if t!='equal']
 print(path, 'format ranges',changes)
 baseline=next(r for r in json.loads((s/'baseline-manifest.json').read_text()) if r['path']==path)
 if 'tests.rs' in path: print('formatted_old_prefix=',target.read_bytes().startswith(Path(baseline['baseline']).read_bytes()))
 results.append({'path':path,'before_sha256':hashlib.sha256(b).hexdigest(),'formatted_sha256':hashlib.sha256(target.read_bytes()).hexdigest(),'ranges':changes})
(s/'format-preview.json').write_text(json.dumps(results,indent=2)+'\n',encoding='utf8')
