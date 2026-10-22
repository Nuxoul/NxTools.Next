# Installed lifecycle feedback — integrated outcome

## Delivered source fixes
- Clipboard native user resize now reaches the actual GPUI logical viewport observer, deferred to original ClipboardApplication. Confirmed-visible/session/HWND guards exclude hidden startup/default and retired windows. Memory size updates immediately; one350ms stable-revision background commit mutates only clipboard_panel with locked reread/merge. Settings/migration cache refresh preserves live size and ordinary clipboard settings no longer overwrite the panel section. Safe temporary-config tests cover logical125/150 scaling, drag coalescing, hide/reopen/reload, latest other settings/storage preservation and visible save failure.
- Download completion Reveal was already success/current-identity gated in Rust. Corrected ExplorerShellActivation's erroneous duplicate COM interface by reusing existing IWebBrowser2 IExplorerBrowserDispatch (GUID/DispIDs/type). Matching native success closes actual task window; failure/cancel/stale keeps retry. Reflection red and fake native green do not prove installed Explorer activation.
- Download monitor Small Toolbar vertically centered within existing44px footer; no native/client/control inflation. Copy now distinguishes 活动连接8 / 总分段11; nontransfer active count0 and unknown progress explicit. Actual control center/hover regression and semantic projection pass.
- Quick Find right-side history icon uses released Button::icon slot instead of normal child content. A temporary real paint content-mask repro measured only8px of a15px glyph. Removed temporary canvas/fields after correction; final test preserves28px target/header/input containment/accessible name and original pointer action. No dependency modification.
- Explorer activation and known-dialog return now share one generation/epoch-scoped capture owner. Held activation capture no longer competes with consumed return; one fresh return read is serialized, stale/destroyed/foreground-lost/first-observation cases cannot publish navigation. Real EventWindow caller barrier repro failed before repair and passes after. No new global poll/cache/persistent STA or weakened bounded-runner admission.

## First-preview / first-switch timing limits
Clipboard preview still intentionally waits800ms, lazily constructs hidden GPUI view and performs four acknowledged native steps. Added bounded no-payload hover-ready/native-step/confirmed/reuse/end timings and real799/800ms cancellation fixture plus25ms injected acknowledgement milestones. **No preview speedup was implemented or claimed.** No safe warmup improvement was established; image first-decode/frame and actual installed timing remain unmeasured.

File-dialog source race is proven/fixed, but actual installed cold COM/recognition/GPUI/frame latency is unconfirmed. Fixtures stop at typed switch publication, not verified native navigation or bar-visible readback. Existing source evidence is in dialog-outcomes.md; no end-to-end native timing claim.

## Executed parent verification
- Final Desktop658/658,0failed/ignored. New3resize+2preview+2monitor+1history tests and expanded actual keyed-window lifecycle included.
- Strict Desktop Clippy--tests -D warnings, cargo check--workspace--tests, cargo fmt--all--check: exit0.
- Final C# build--no-restore -m:1:0warnings/errors. Exact reviewed-safe union14RUN/PASS,0SKIP/FAIL/not_run (6source caller+4native-action fake+4existing tracker/selection fixtures). Never unrestricted/native harness.
- Six architecture gates each explicit0: clipboard-ui,download-ui,quick-launch,file-dialog-search-lifecycle,file-dialog-window-follow,platformhost-test-selection. Spec contracts87documents/12owners; git diff--check0.
- Executed product-specific red: wrong COM GUID failed1/1; history mask8x15 vs15x15 failed1/1; temporarily restored missing resize dispatch failed300x440 vs512x688; removed footer center failed actual control-center assertion; source-return barrier failed no switch. Temporary injected resize early-return and history canvas/fields removed. Initial constant typo and concurrent-edit intermediate compile failure were not product bug evidence.
- All Rust commands jobs1/incremental0/codegenunits256 with verified1.96.0 command-local PATH and21600s timeout; oneCargo. Source/static checks do not certify visual/native lifecycle.

## Delivery boundaries
No Release package/install/product launch/live Explorer/COM/UIA/Shell/clipboard/registry/service/browser/volume operation, commit or push. The10:45 installer from the previous turn **does not include these repairs**. Existing broad dirty changes preserved. Actual installed resize/restart, Explorer close behavior, screenshots/DPI/physical input and first-use timing remain acceptance work; first-preview speed optimization remains open rather than fabricated.

## Logs
Session scratch: installed-feedback-history-red.log, installed-feedback-resize-red.log, installed-feedback-footer-red.log, installed-feedback-desktop-first.log, installed-feedback-quality-final.log, installed-feedback-explorer-red.log, installed-feedback-explorer-green.log, installed-feedback-csharp-final.log. Child dialog red/green/logs are named in dialog-outcomes.md. Reference/owner/component records: clipboard-outcomes.md, download-outcomes.md, quick-icon-outcomes.md, native-outcomes.md, dialog-outcomes.md.
