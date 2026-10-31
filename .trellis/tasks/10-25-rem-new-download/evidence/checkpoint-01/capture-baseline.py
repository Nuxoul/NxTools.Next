from pathlib import Path
import json,hashlib,datetime
R=Path.cwd();S=Path(__file__).parent;S.mkdir(parents=True,exist_ok=True);h=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
paths=['rust/apps/nxtools-desktop/src/downloads/new_download_view.rs','docs/reference-code/download.md']
rows=[]
for p in paths:
 b=(R/p).read_bytes();t=S/(p.replace('/','__')+'.baseline')
 if t.exists():raise SystemExit('refuse recapture '+p)
 t.write_bytes(b);rows.append(dict(path=p,sha256=h(t),baseline=str(t),bytes=len(b)))
(S/'baseline-manifest.json').write_text(json.dumps(rows,indent=2)+'\n',encoding='utf8')
protected=[
'rust/apps/nxtools-desktop/src/ui/metrics.rs','rust/apps/nxtools-desktop/src/ui/typography.rs','rust/apps/nxtools-desktop/src/theme.rs','rust/apps/nxtools-desktop/src/ui/components/field.rs',
'rust/apps/nxtools-desktop/src/downloads/schedule_view.rs','rust/apps/nxtools-desktop/src/downloads/polish_tests.rs','rust/apps/nxtools-desktop/src/main_window/download_page.rs','rust/apps/nxtools-desktop/src/main_window/download_page/kit_migration_tests.rs',
'rust/apps/nxtools-desktop/src/downloads.rs','rust/apps/nxtools-desktop/src/downloads/browser_handoff_impl.rs','rust/apps/nxtools-desktop/src/downloads/directory_picker.rs','rust/apps/nxtools-desktop/src/downloads/directory_picker_tests.rs','rust/apps/nxtools-desktop/src/downloads/new_download_keyboard_tests.rs','rust/apps/nxtools-desktop/src/downloads/tests.rs',
'rust/apps/nxtools-desktop/src/archive/forms.rs','rust/apps/nxtools-desktop/src/archive/forms_tests.rs','rust/apps/nxtools-desktop/src/archive/progress.rs','rust/apps/nxtools-desktop/src/archive/job_tests.rs','rust/apps/nxtools-desktop/src/main_window/archive_page.rs','rust/apps/nxtools-desktop/src/main_window/archive_page_tests.rs','rust/apps/nxtools-desktop/src/main_window/extensions_page.rs',
'rust/apps/nxtools-desktop/src/ui/history.rs','rust/apps/nxtools-desktop/src/ui/history_identity_tests.rs','rust/apps/nxtools-desktop/src/ui/components/action.rs','rust/apps/nxtools-desktop/src/ui/components/action_name_tests.rs']
(S/'protected-baseline.json').write_text(json.dumps([dict(path=p,sha256=h(R/p)) for p in protected],indent=2)+'\n',encoding='utf8')
history=[]
for d in ['10-24-download-enter-popover','10-25-download-picker-identity','10-25-clipboard-stable-focus','10-25-button-accessible-names','10-25-download-sort-keyboard','10-25-archive-prompt-object','10-25-default-submit-keyboard','10-25-rem-role-consistency']:
 for p in sorted((R/'.trellis/tasks'/d).rglob('*')):
  if p.is_file():history.append(dict(path=p.relative_to(R).as_posix(),sha256=h(p)))
(S/'frozen-history.json').write_text(json.dumps(history,indent=2)+'\n',encoding='utf8')
print(json.dumps(dict(captured_at=datetime.datetime.now().isoformat(),candidates=len(rows),protected=len(protected),history=len(history))))
