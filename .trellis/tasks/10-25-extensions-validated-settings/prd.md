# P1 — Validated settings

## Authority and scope
Child of approved `.trellis/tasks/10-25-extensions-capability-expansion/`; user explicitly approved all seven capabilities/final summary with “开始”2026-10-25. Implement parent R1/A1 serially after P0. No new native UI/service/user-data/deployment/commit/delegation authority.

## Requirements
- Optional text/file/folder validation: required(defaultfalse), Unicode scalar min/max under existing32768UTF8byte cap, full-string regex<=512bytes/compiled1MiB/DFA1MiB. Exact alreadylocked regex1.13.1 only; no filesystem/network existence probes, password/bool/custom script validation.
- Choice retains legacy string arrays/duplicates/canonical bytes; new homogeneous `{value,label}` arrays require unique values and bounded control-free strings. UI displays label, stored/default is value. Unknown persisted selection stays explicit, not first-option fallback.
- Required-unconfigured declaration may import/load. Original Save and invocation readiness reject missing/invalid values before any secret operation/process effect. Valid defaults apply; no trimming/normalizing data. Core locked current-manifest save remains authoritative.
- Existing persistent draft/official Input+Select/Settings/SaveCancel/path picker revisions/blank-preserves explicit secret clear/credentials compensation remain.

## Acceptance
- [x] Formal ImportPreview/registry corpus: legacy serialization/hash, Unicode scalar count, regex/range/default/unknown/duplicate/mixed choice validation, value-label roundtrip.
- [x] Required-unconfigured import/load succeeds; Save/invoke rejects invalid/missing; temp Core locked rejection leaves bytes/unrelated settings unchanged.
- [x] Fake SecretStore has zero reads/writes for invalid nonsecret draft; old blank/clear/compensation tests pass.
- [x] Actual official settings draft controls display label/store value, preserve entities/cancel/current content, inline errors and usable bounds; source/GPUI fixtures only, no live native UI.
- [x] Docs/schema/offline parity; focused tests/scoped strictlint/workspace check. Corefull88/1Archive failure retained separately in outcomes. No final fullDesktop until P8.

## Change boundary and evidence
Domain contract lives in extensions script.rs; original registry.set_settings and settings_transaction enforce saves, runtime before process dispatch. Core original section transaction composes it; Desktop extensions_settings only projects original draft. Expected files: new validation module/tests, script/lib/registry/settings_transaction/runtime; existing Core tests, existing Desktop settings tests/view, schema/authoring. No broad refactor/catalog layout, native picker/credential implementation, framework/vendor/backend or runtime installation changes.

UI selection: preserve Settings->SettingGroup->SettingItem with public SettingField::render; official Input/Select defaultMedium32DIP, nonbool vertical layout, original footer outlined Buttons outside Toolbar.child. Exact Component0.8.0 select public SelectItem value/title separates labels; official Alert inline feedback. No new focus/window/controller. Reference: WinShun PluginSettings.cpp choiceOptions59+, settings projection605+, setSetting755+; parent research amendment records differences. Approved parent design intentionally uses label (not reference title) and rejects unknown values instead of reference first-option fallback.
