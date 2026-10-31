from pathlib import Path
import hashlib, json, datetime
root=Path.cwd(); scratch=Path(r'C:\Users\Administrator\.pi-desktop\scratch\da6a2ca3-52f0-41f1-a333-3b5548154f0b\item06')
paths=[
'rust/apps/nxtools-desktop/src/ui/metrics.rs',
'rust/apps/nxtools-desktop/src/ui/typography.rs',
'rust/apps/nxtools-desktop/src/downloads/schedule_view.rs',
'rust/apps/nxtools-desktop/src/downloads/polish_tests.rs',
'rust/apps/nxtools-desktop/src/main_window/download_page.rs',
'rust/apps/nxtools-desktop/src/main_window/download_page/kit_migration_tests.rs',
'docs/reference-code/download.md',
'docs/reference-code/gpui-kit-080.md',
]
protected=[
'rust/apps/nxtools-desktop/src/downloads/new_download_view.rs',
'rust/apps/nxtools-desktop/src/downloads/new_download_keyboard_tests.rs',
'rust/apps/nxtools-desktop/src/downloads/directory_picker.rs',
'rust/apps/nxtools-desktop/src/downloads/directory_picker_tests.rs',
'rust/apps/nxtools-desktop/src/downloads/browser_handoff_impl.rs',
'rust/apps/nxtools-desktop/src/downloads/tests.rs',
'rust/apps/nxtools-desktop/src/archive/forms.rs',
'rust/apps/nxtools-desktop/src/archive/forms_tests.rs',
'rust/apps/nxtools-desktop/src/archive/progress.rs',
'rust/apps/nxtools-desktop/src/archive/job_tests.rs',
'rust/apps/nxtools-desktop/src/ui/components/action.rs',
'rust/apps/nxtools-desktop/src/ui/components/action_name_tests.rs',
'rust/apps/nxtools-desktop/src/ui/history.rs',
'rust/apps/nxtools-desktop/src/ui/history_identity_tests.rs',
'rust/apps/nxtools-desktop/src/ui/components/dialog.rs',
'rust/apps/nxtools-desktop/src/main_window.rs',
'rust/apps/nxtools-desktop/src/theme.rs',
]
scratch.mkdir(parents=True,exist_ok=True)
manifest=[]
for p in paths:
 b=(root/p).read_bytes(); target=scratch/(p.replace('/','__')+'.baseline')
 if target.exists(): raise SystemExit('refuse recapture '+p)
 target.write_bytes(b)
 manifest.append(dict(path=p,sha256=hashlib.sha256(b).hexdigest(),baseline=str(target),bytes=len(b)))
(scratch/'baseline-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n',encoding='utf8')
(scratch/'protected-baseline.json').write_text(json.dumps([dict(path=p,sha256=hashlib.sha256((root/p).read_bytes()).hexdigest()) for p in protected],indent=2)+'\n',encoding='utf8')
history=[]
for d in ['10-24-download-enter-popover','10-25-download-picker-identity','10-25-clipboard-stable-focus','10-25-button-accessible-names','10-25-download-sort-keyboard','10-25-archive-prompt-object','10-25-default-submit-keyboard']:
 for p in sorted((root/'.trellis/tasks'/d).rglob('*')):
  if p.is_file(): history.append(dict(path=p.relative_to(root).as_posix(),sha256=hashlib.sha256(p.read_bytes()).hexdigest()))
(scratch/'frozen-history.json').write_text(json.dumps(history,indent=2)+'\n',encoding='utf8')
print(json.dumps(dict(captured_at=datetime.datetime.now().isoformat(),candidates=len(manifest),protected=len(protected),frozen_history=len(history))))
