# Progress log

## Phase A — Baseline and references (done 2026-10-22)
- Baseline copy + SHA256 (31 files) in session scratch `ext-refactor-baseline/`; unrelated dirty
  archive/clipboard/markdown-editor files recorded and untouched.
- Official evidence recorded in `research/platform-facts.md` (Chrome suggest/return true; MV3
  background.scripts/service_worker per browser). Legacy NxTools called suggest synchronously.
- Baseline Node suite 44/44.

## Phase B — Behavior harness (done)
- `tests/harness/chrome-api.cjs`: programmable WebExtension double (events, native ports, storage
  local/session, tabs, downloads, webRequest, scripting, DNR), no network/native/profile.
- `tests/harness/load-background.cjs`: loads the real production background in a vm realm; Chrome mode
  via importScripts, Firefox mode via manifest `background.scripts` with no importScripts; fake desktop
  host and virtual clock.
- `tests/defects.behavior.test.js`: 11 R1 tests. Pre-fix result: 9 fail (R1.1×2, R1.2×2, R1.3, R1.4×3
  after tightening the foreign-sender case, R1.5, R1.6), 2 controls pass.

## Phase C — Defect fixes (done)
- R1.1 listener returns true; interceptBrowserDownload resolves suggest once in `finally` on every path
  (including settings/load failures).
- R1.2 manifest `background.scripts` (dependencies first, entry last) + `minimum_chrome_version: "121"`;
  entry guards `importScripts`; `CreateFirefoxManifest` keeps the source list, drops service_worker.
  C# regression extended (XPI manifest script list) — red first, then green. While turning it green, the
  same fixture exposed a pre-existing .NET 8 failure: `ToJsonString(new() { WriteIndented = true })`
  threw "JsonSerializerOptions ... TypeInfoResolver" (reproduced on untouched baseline code), i.e. the
  Firefox export itself was broken. Fixed with an explicit `DefaultJsonTypeInfoResolver`.
- R1.3 drop target requires ok+accepted.
- R1.4 router sender contexts: foreign (other extension) → silently ignored; content scripts limited to 8
  content message types; extension pages allowed. `sanitizeSettings` keeps only known keys/types,
  position enum, width 96..360, height 48..220; storage failure now reported.
- R1.5 `resolveTabIdForDownload`: download tabId, observed request tab (`tabIdObserved`), or referrer tab;
  never `lastActiveTabId`.
- R1.6 Content-Disposition names mirrored to `chrome.storage.session` under a hashed URL key (no query
  text, headers, cookies or referrer), bounded 200, restored lazily before handoff.
- Removed one obsolete source-regex test in `safe-handoff.test.js` superseded by R1.1 behavior tests.
- `tests/architecture/browser-native-messaging.ps1`: old literal `importScripts(...)` assertion replaced
  by manifest-list/entry-guard contract.

### Checks (2026-10-22)
- Node: 54/54 pass (43 legacy retained + 11 new). `blob-transfer-policy.test.js` direct: pass.
- C# selected fixtures `browser signing fixture *`: 7/7 pass (`SUMMARY selected=7 run=7 pass=7`).
- `browser-native-messaging.ps1`: **fails on two pre-existing desktop assertions** about
  `rust/apps/nxtools-desktop/src/main.rs` (`focus: launch_mode.main_window_focus()` and hidden main
  window until native minimization). Reproduced with the untouched baseline gate + baseline extension
  files, so not caused by this task; the desktop file belongs to other in-flight work. With only those
  desktop assertions removed in a scratch copy, every extension assertion passes (exit 0).

## Phase D — Background split (done)
- Deterministic scratch script cut `background.js` (4904 lines after Phase C) into 16 classic-script
  modules under `bg/` sharing the worker global scope; entry `background.js` 155 lines (imports +
  lifecycle/navigation/download listeners). Largest module `bg/site-adapters.js` 706 lines.
- Lossless check (`verify-split.cjs` in scratch): every pre-split non-blank line present; only additions are
  module headers, `"use strict"` and the dependency list (plus the trailing-comma change on one list item).
- Manifest `background.scripts` and the entry `BACKGROUND_DEPENDENCIES` list the same 20 files.
  Packaging (`package.ps1` recursive copy, Inno `browser_extension\*`, CRX/XPI `AllDirectories`) already
  carries subdirectories.
- Tests no longer slice `background.js` by marker: `tests/harness/background-source.cjs` (manifest-ordered
  concatenation + declaration extraction); `native-lifecycle.test.js` rewritten as 7 behavior tests on the
  loaded worker; new `background-modules.test.js` (load-order parity, Firefox listener parity, entry ≤200,
  module ≤900, unique declaration owner, files exist). `browser-native-messaging.ps1` reads the whole
  manifest script set.
- Checks: Node 61/61; extension-only native-messaging gate exit 0 (desktop assertions still pre-existing red).

## Phase E — Content script (in progress)
- New real-DOM gate `tests/content-browser.cjs` (headless Chrome, production page-hook + manifest content
  scripts, mocked extension API, network blocked). First run exposed a **pre-existing production defect**:
  the `content_blob_info` / `content_upload_blob` handlers were inside the `chrome.storage.local.get`
  callback instead of the runtime message listener (present in HEAD and baseline). Every page load threw
  `ReferenceError: message is not defined`, and the background Blob upload path
  (`sendBlobResourceViaContentScript`) never received a response, so every Blob handoff fell back to the
  browser. Fixed by moving both branches into the runtime listener; gate red (4/8) → green (8/8).
- **Design blocker found before splitting (paused for user decision):** `content-script.js` is one
  2495-line IIFE with 132 closure-level names and 16 mutable shared variables. It runs twice in the same
  isolated world by design (manifest `document_start` + `injectContentBridge` `executeScript` on tab
  activation/navigation/focus, guarded by `window.__nxToolsMediaBridgeInstalled`). The planned
  "ordered classic scripts sharing top-level scope" layout (design.md R3, which worked for the background
  worker) would throw `Identifier ... has already been declared` on the second injection and re-run
  side effects, so it is not a safe mechanical move here. Cross-group dependency scan (scratch
  `content-deps.cjs`) shows only constants, runtime messaging, URL/MIME helpers and the page Blob bridge
  are free of shared mutable state; overlay/detection/drop/direct-download/wiring all share closure state.
- **User decision 2026-10-22: option A** (extract only stateless parts). Per-declaration transitive
  analysis (scratch `content-analyze.cjs`) marked every declaration that reaches the 16 shared variables;
  only non-stateful declarations moved. Deterministic `split-content.cjs` aborts if a stateful name is
  assigned to a module or a module depends on a main-only name.
- Result: six idempotent isolated-world namespace modules under `content/` registered on
  `globalThis.NxToolsContent` (constants 140, runtime 53, media-info 354, resource-choice-text 190,
  blob-bridge 280, media-geometry 174 lines; exports frozen; re-injection is a no-op). `content-script.js`
  2495 → 1491 lines, still the single owner of all overlay/drop/capture/direct-download state and the
  `__nxToolsMediaBridgeInstalled` guard.
- Manifest `content_scripts[0].js` and background `CONTENT_SCRIPT_FILES` (re-injection) list the same 9
  files. Lossless check: no pre-split line missing; only added lines are module wrappers, namespace
  imports/exports and header comments.
- Gates: new `content-modules.test.js` (list parity with re-injection, load order, idempotent/frozen
  namespaces in a vm realm, no shared-state references, main ≤1600 / module ≤400). `content-browser.cjs`
  extended with real re-injection: no duplicate listener, no redeclaration error, still responsive.
- Checks: Node 65/65; content real-DOM 11/11; extension-only native-messaging gate exit 0. Pre-redesign UI
  baselines: `layout-browser.cjs` 38/38, `redesign-browser.cjs` 34/34 (previous gallery copied to scratch
  `artifacts-before/` for comparison).

## Phase F — Frontend refinement (done, awaiting user visual approval)
- Red first: `tests/refine-browser.cjs` 14 measurable targets, 12/14 red on the 2026-10-12 UI (header 126px, 2 visible
  resources, merge button, idle footer, always-on batch bar, text thumbnails, …). Green 14/14 after implementation.
- Popup split into `popup/{format,dom,render,actions}.js` + entry; DOM-API rendering only; tokens + `--nt-success`.
- Older UI gates updated to the new structure without dropping checks (script loading in popup.html order; merge
  contract changed from "honestly disabled" to "not offered"; chooser markup with icon column).
- Gallery regenerated and inspected via scratch contact sheets; fixed busy-state fixture and "重试" tone.

## Phase G — Spec and wrap-up (done)
- Updated `.trellis/spec/browser-extension/{index,contracts,ui-guidelines}.md` and extension README (also fixed a
  broken `browser_extension\nxtools-interceptor` path that had become a line break). Outcomes in `outcomes.md`.
