# NxTools browser layout workbench

Open `index.html` directly; there is no server, install step, build, dependency or CDN. All fixture names, paths, tasks, history and rules are synthetic. All changes are session memory only. Refresh/reset discards them. Layout contract: `design-contract.md`.

## Review controls

The chooser, state selector, size selector and zoom selector are outside the app frame. Actual mode draws the exact CSS-pixel budget at scale 1 (browser zoom must also be 100%). A smaller browser viewport scrolls the workbench; it does not silently shrink app controls. Fit mode explicitly reports its scale. Main pages support default1060×650 and minimum920×480. Fixed windows cannot be enlarged through the API.

- Main four modules: Downloads workspace; Quick Find settings (General, Index, Context, Commands, Diagnostics); Clipboard settings (Trigger/actions, Retention, Privacy, Storage); Tools (Link Detection only).
- Global settings720×480: General, Network, independent per-app Clipboard/Quick rules; add/edit/remove, validation, local draft Apply/Cancel, error-preserving retry, busy guard.
- Quick popup620×410: verified production budget with42px search/40px result rows; search synthetic results, ↑/↓ selection, Enter simulated action, → at query end searchable in-window actions, Esc returns. Native show/focus/DPI is not certified by this prototype.
- Search-only620×44 is a separate chooser view; typing transitions to the synthetic620×410 results and returns input focus. It is distinct from no-match/error states.
- Clipboard popup300×440: category/search/history, keyboard selection, simulated paste, edit sheet with inert background and Escape focus return.
- Download probe560×176 → ordinary confirmation560×278 → only-add paused task / simulated progress. Content-only progress/completion440×184; expanded440×280; failed440×196. Failure has its own page/budget. DASH560×353, HLS560×418, media-error560×456 are explicitly provider wireframes.
- Supporting page: component states, menu/submenu, notifications, passive preview, schedule/delete, shutdown, extension Popup, media/drag. Native/provider operations are labeled wireframes and never execute.

## Automation

Stable controls: `#page-selector`, `#state-selector`, `#size-selector`, `#zoom-selector`, `#app-frame`; actions use `data-action`, navigation uses `data-page`/`data-value`, settings use `data-setting`, rules use `data-rule-toggle`/`data-feature`, tasks use `data-task-check`. Inactive tabs are unmounted; edit-sheet background is inert.

```js
prototypeApp.setPage('downloads');
prototypeApp.setSize('minimum');
prototypeApp.setState('long');
prototypeApp.getAudit();
prototypeApp.setPage('global');
prototypeApp.setSection('应用黑名单');
prototypeApp.setZoom('fit');
prototypeApp.getModel(); // cloned in-memory demo model
prototypeApp.reset();
```

`prototypeApp.pages` includes dimensions and any narrowed applicable state list; `prototypeApp.states` lists normal/long/empty/loading/error/busy. Completed/failed already encode terminal state, so only normal/long apply. Asking for progress/error selects the separate failed budget. Long explicitly injects stress fixtures; it is a design control, not a product operation. `getAudit` reports size, actual size, scale, bounds violations, unexpected horizontal overflow, named scroll regions, footer bounds and current focus. Intended text ellipsis and vertical content scrolling are reported policies, not window enlargement.

## Browser checks

```powershell
node --check .\artifacts\ui-layout-quality\prototype\prototype.js
node .\artifacts\ui-layout-quality\prototype\design-checks.cjs
```

Requires installed Chrome (override `PROTOTYPE_CHROME` to its executable); no npm packages. Test harness uses CDP anonymous pipes and file URLs, no local HTTP server. Fresh synthetic Chrome profile, JSON report and eight browser captures go into `PI_SCRATCH_DIR` (OS temp fallback). Checks cover all applicable page/state/geometry combinations, all subsection layouts, batch/search, rule alignment including many-row scrolling, independent toggles, draft Apply failure/success/Cancel, command save/separate drafts, confirmation retry, clipboard immediate settings/edit failures, CDP Tab/Space/Escape/arrow/Enter, disabled no dispatch, reduced-motion CSS, 600px workbench actual/Fit modes, footer stability and main layout budgets.

Known verification boundary: BrowserPreview opened the file and provides live reload but no screenshot/DOM feedback to this subagent. Chrome DOM/geometry/keyboard checks are executable evidence; captures were acquired, not visually inspected by an image-capable tool. Human visual approval, native HWND/DPI, no-activate preview, OLE, browser extension/runtime/provider behavior and screen-reader-provider acceptance remain unverified. No production source, Cargo, services, package, persisted user data or network operations belong to this prototype.

Parent convergence: final browser harness305/305 after correcting the initial640×420 popup assumption to verified620×410, preserving42px search/40px result rows, adding collapsed620×44 and expanding subsection checks to both default/minimum. Final125 page cases/148 subsection cases/32 action checks. Parent inspected download/global/command/clipboard popup screenshots through the built-in browser; complete visual/user/native acceptance remains open. See `../verification.md` and `../gallery.html`. Prototype directory is ignored by current Git rules; files are delivered on disk, no ignore changes made.
