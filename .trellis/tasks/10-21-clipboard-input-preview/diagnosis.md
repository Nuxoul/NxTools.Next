# Read-only runtime evidence and causal reproduction

## Existing installed events
Read on2026-10-21 from `%LOCALAPPDATA%\NxTools\NxTools.Next\data\logs`; outputs were filtered to lifecycle IDs/events/timestamps/booleans/elapsed, never copied Clipboard payload/config/history. Running Desktop path `C:\ProgramData\NxTools\NxTools.Next\Service\NxTools.exe`, SHA256 `276E06DECC76BB7FC2F740EB0AE3712BD1A728DB0AA33021301365655CC55D16`, matched `artifacts\win-x64\NxTools.exe` at observation. This establishes protected/package equality, not equality to unbuilt working source.

2026-10-21T14:21:07.986397Z preview.hover_ready ->14:21:08.010169Z preview.reuse_reveal_queued correlationa3b6fb48-bb3e-4f43-85b2-17aca96538c6.14:21:08.039143Z capture_pause.started ->14:21:08.039571Z completed paused=true/success=true correlationd031a21c-e920-45d9-a06d-4f3c55c5442c.14:21:10.484549Z hover_ready ->14:21:10.510073Z reuse_reveal_queued correlation8ac095f7-93c0-41ea-aff0-3319918bd9c7, then14:21:10.525576Z capture_pause.started ->14:21:10.527108Z completed paused=false/success=true correlatione1e580f7-f89a-4113-9fb1-dbc384f1333d. Preview reveals actually preceded those toggles by milliseconds; do not claim logs prove the toggle created them. They prove existing preview and pause coexist and native pause succeeds. Recent18000desktoplines had0action.started/0action.failed; no old shortcut rejection event existed, so exact installed key failure is not directly logged.

## Red-capable command
Mandatory-memory-env command: `cargo test -p nxtools-desktop --locked --offline input_repair_ -- --test-threads=1`.
- `clipboard-input-preview-red.log`:3tests,1pass/2fail. No-focused-descendant Ctrl2 never creates pendingPaste; control-focus `a` leaves search empty.
- `clipboard-input-preview-red2.log`:5tests,1pass/4fail. Added queuedpreviewcandidate->Pause and queuedcandidate->Ctrl2 both leave candidate alive; normal pointer-move-to-header path passed, so it was not presented as a red.
- Minimized each controlled variable: real production panel/temporary history worker/fakeIPC retains original action; only focus or queuedcandidate changes. Exact fullPasteText/target42 asserted, not any IPC. No native paste into a live application.

## Ranked predictions and resolution
1. Element capture lacks a path when no focused descendant -> same-window interceptor admission/restored search allows matching release to dispatch. Original red now green.
2. Noninput focus has no platform text recipient -> public InputHandler forwarding to original InputState preserves first text/IME, not key-label synthesis. Original ASCII red now green; Unicode mark/commit/editor/activation regressions passed.
3. Nonitem action lacks preview retirement -> deferred original dismiss clears candidate/generation/pending stage; both queuedcandidate reds green. Existing noactivate/readback/hiddenframe/reuse owner remains.

Read-only Windows ToUnicode test with synthetic keyboard state for Ctrl0..9 returned count0for all ten. No production keyboard state/global hook/clipboard/UI mutation. Therefore no speculative WM_CHAR digit-suppression patch was added. This is current input-layout API evidence, not every IME/layout guarantee.
