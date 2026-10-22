# Browser extension complete UI/UX redesign

## Authorization and goal
User explicitly authorizes redesigning the entire browser extension (2026-10-12). Implement production presentation and focused interaction improvements, not a screenshot-only mockup. Desktop GPUI migration remains a separate task.

## Scope
All popup Tasks/Capture/Media tools/Settings, page media entry/resource chooser/status, link drop target. Existing context menus retain action/acceptance owner and wording. Preserve every original field/action, settings schema, four tabs and456px popup width, workspace420..445px. Design may change presentation hierarchy, density, labels and component composition.

## Required preservation
- Background/page-hook/capture/handoff/Blob policies and manifest identity/permissions/CSP unchanged.
- Native transport remains demand-only; popup snapshots never start Desktop.
- Original ID/generation/batch32/partial failure/browser fallback and secret-free projection owners retained.
- Settings auto-save on change and explicit Save retry remain; no new staged persistence model.
- Media frame/index and native actions unchanged; no real browsing, cookies, registry/install/package/data operation.
- Preserve dirty work; one writer/no delegation/reset/commit.

## Acceptance
1. Production HTML/CSS/JS renders all features with a coherent compact neutral workbench, no decorative animation/card wall.
2. Real isolated DOM interaction covers tabs/filter semantics, keyboard resource preview, selection/batch, settings save/failure, feature/media errors, busy guards and unavailable states.
3. Default/long/empty/offline/loading/error/disabled/busy/selection/minimum-drop and Shadow DOM styles inspected in browser. Existing policy/tests pass; feature checks not weakened for aesthetics.
4. Capture a gallery with actual production renderer/scripts plus explicitly mocked Chrome APIs and blocked network. Distinguish synthetic DOM from installed extension/physical browser/reader acceptance.
