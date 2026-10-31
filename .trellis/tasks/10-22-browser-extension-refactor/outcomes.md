# Outcomes — browser extension refactor and refinement (2026-10-22)

## Delivered
- **R1 defects (all red-first):** deferred `suggest` without `return true` (Chrome Save As during handoff); Firefox background missing every policy module (`importScripts` absent); drop target reporting rejected handoffs as sent; no sender trust in the runtime router and unsanitized settings writes; browser download borrowing the last active tab's resource; Content-Disposition names lost on worker suspension.
- **Found by the new gates and fixed (pre-existing, reproduced on untouched baseline):** Firefox XPI export threw on .NET 8 (`JsonSerializerOptions` without `TypeInfoResolver`); content-script Blob handlers sat inside the storage callback, so every page load threw `ReferenceError` and every Blob handoff fell back to the browser.
- **R2 harness:** `tests/harness/` loads the real production background in Chrome (importScripts) and Firefox (manifest list) modes with a programmable WebExtension double, fake desktop host and virtual clock. No marker-sliced source fixtures remain.
- **R3 refactor:** `background.js` 4904 → 155-line entry + 16 `bg/` modules (max 706). `content-script.js` 2495 → ~1500 lines + 6 idempotent `content/` namespace modules (option A chosen by the user: only stateless parts). `popup.js` 1114 → 13-line entry + 4 `popup/` modules. Both splits were lossless-checked line by line.
- **R4 refinement (direction A):** header 165 → ≤120px; capture tool area 4 → 2 layers; compact resource rows with SVG type icons (6 visible instead of ~2–3 in the default viewport); merge placeholder removed; idle status bar hidden, tone-coded feedback; task progress + actions on one line with semantic status; settings rows ≤50px; page chooser rows with the same icons; one added success token. Popup renders without `innerHTML`.

## Unchanged by design
Permissions, host permissions, `key` (extension ID), CSP, gecko settings, web-accessible resources, Native protocol version 2 and every message type/payload (verified against the baseline manifest). Background `popup_merge_resources` stays.

## Checks (2026-10-22, final run)
- Node `node --test` (recursive): **65/65**; Blob direct assert: pass.
- Real DOM (headless Chrome, mocked API, network blocked): content **11/11** (incl. re-injection), refine **14/14**, layout **38/38**, redesign **34/34**.
- C# `browser signing fixture *`: **7/7** (red first for the XPI script list).
- `browser-native-messaging.ps1`: extension assertions pass; the full gate still fails on two **pre-existing desktop assertions** about `rust/apps/nxtools-desktop/src/main.rs` (reproduced with the baseline gate and baseline extension; other in-flight work owns that file).
- `spec-contracts.py`: PASS (109 documents). All `node --check` clean; no owned Chrome process left.
- Gallery regenerated at `artifacts/ui-extension-redesign/` and inspected as contact sheets (main, more, states, busy). Inspection fixed one gallery-only issue (busy state missing its selection) and one feedback-tone issue (“请重试” shown as success).

## Limits
Synthetic DOM and harness tests do not certify installed Chrome/Edge/Firefox behavior, real pages/iframes/site CSP, Native Messaging with the real host, high-DPI, physical input or screen readers. Not done (not authorized): install/reload the extension, native registry, CRX/XPI export, packaging, Cargo, commit/push. User visual approval of the refinement is still pending.
