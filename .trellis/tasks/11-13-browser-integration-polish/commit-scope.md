# Browser integration commit scope

Proposed commit: `feat: improve browser integration reliability and workflows`

## Included files

```text
.trellis/spec/browser-extension/contracts.md
.trellis/spec/browser-extension/ui-guidelines.md
.trellis/spec/nxtools-browser-host/backend/logging-guidelines.md
.trellis/tasks/11-13-browser-integration-polish/check.jsonl
.trellis/tasks/11-13-browser-integration-polish/commit-scope.md
.trellis/tasks/11-13-browser-integration-polish/design.md
.trellis/tasks/11-13-browser-integration-polish/implement.jsonl
.trellis/tasks/11-13-browser-integration-polish/implement.md
.trellis/tasks/11-13-browser-integration-polish/outcomes.md
.trellis/tasks/11-13-browser-integration-polish/prd.md
.trellis/tasks/11-13-browser-integration-polish/task.json
browser_extension/nxtools-interceptor/background.js
browser_extension/nxtools-interceptor/bg/capture.js
browser_extension/nxtools-interceptor/bg/context-menu.js
browser_extension/nxtools-interceptor/bg/handoff.js
browser_extension/nxtools-interceptor/bg/link-collection.js
browser_extension/nxtools-interceptor/bg/media.js
browser_extension/nxtools-interceptor/bg/native-transport.js
browser_extension/nxtools-interceptor/bg/popup-state.js
browser_extension/nxtools-interceptor/bg/request-context.js
browser_extension/nxtools-interceptor/bg/resource-cache.js
browser_extension/nxtools-interceptor/bg/router.js
browser_extension/nxtools-interceptor/bg/state.js
browser_extension/nxtools-interceptor/bg/tab-bridge.js
browser_extension/nxtools-interceptor/content/media-inspection.js
browser_extension/nxtools-interceptor/content-script.js
browser_extension/nxtools-interceptor/handoff-policy.js
browser_extension/nxtools-interceptor/manifest.json
browser_extension/nxtools-interceptor/popup.css
browser_extension/nxtools-interceptor/popup.html
browser_extension/nxtools-interceptor/popup.js
browser_extension/nxtools-interceptor/popup/actions.js
browser_extension/nxtools-interceptor/popup/dom.js
browser_extension/nxtools-interceptor/popup/format.js
browser_extension/nxtools-interceptor/popup/render.js
browser_extension/nxtools-interceptor/README.md
browser_extension/nxtools-interceptor/tests/connection-diagnosis.behavior.test.js
browser_extension/nxtools-interceptor/tests/content-modules.test.js
browser_extension/nxtools-interceptor/tests/frame-workflow.behavior.test.js
browser_extension/nxtools-interceptor/tests/harness/chrome-api.cjs
browser_extension/nxtools-interceptor/tests/harness/load-content.cjs
browser_extension/nxtools-interceptor/tests/link-collection.behavior.test.js
browser_extension/nxtools-interceptor/tests/media-inspection.behavior.test.js
browser_extension/nxtools-interceptor/tests/media-inspection-content.behavior.test.js
browser_extension/nxtools-interceptor/tests/polish-browser.mjs
browser_extension/nxtools-interceptor/tests/polish-popup.behavior.test.js
browser_extension/nxtools-interceptor/tests/popup-contract.test.js
browser_extension/nxtools-interceptor/tests/popup-fixture.js
browser_extension/nxtools-interceptor/tests/redesign-browser.cjs
browser_extension/nxtools-interceptor/tests/request-method.behavior.test.js
browser_extension/nxtools-interceptor/tests/resource-cache.test.js
browser_extension/nxtools-interceptor/tests/workflow-boundaries.behavior.test.js
docs/reference-code/browser-integration-polish.md
rust/apps/nxtools-browser-host/src/browser_wire_tests.rs
rust/apps/nxtools-browser-host/src/lib.rs
```

## Excluded worktree changes

All paths outside the included-file list remain excluded, including:

- `Cargo.lock`.
- `csharp/NxTools.PlatformHost/**` and `csharp/NxTools.PlatformHost.Tests/**`.
- `rust/apps/nxtools-desktop/**`.
- `rust/crates/nxtools-archive/**`, `nxtools-core/**`, `nxtools-download/**`, `nxtools-download-ipc/**`, `nxtools-protocol/**`.
- Other `.trellis/spec/**` and `.trellis/tasks/**` changes.
- Existing modified `docs/reference-code/download.md`, `mister-download-manager.md`, archive/clipboard/quick-launch/system reference documents, `docs/reference-improvement-coordination.md` and `docs/reference-improvement-taskbook.md`.
- `tests/architecture/archive-tree-root-drag.py` and `tests/architecture/quick-launch-lifecycle.ps1`.

No push, package, browser registration or installation is included.
