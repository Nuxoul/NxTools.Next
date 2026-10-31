# Extensions API and usability repair

## Approval and scope
User2026-10-24 explicitly said “开始按照你说的来” after the [source-backed audit](../../../docs/audits/2026-10-24-extensions-api-ux-review.md). Implement F1–F9 in the proposed order. This is implementation approval, not packaging/install/commit/archive or installed acceptance authorization.

## Requirements
- Preserve bounded plugin error explanations through runtime to UI; consistent success/handled/not_handled/error semantics.
- Confirm extension removal with affected configuration/runs/resolver-chain disclosure; cancel/stale confirmation changes nothing.
- Show bounded automatic-event terminal metadata and actionable recovery without replaying old events.
- Add backward-compatible bounded download/Clipboard continuation APIs, explicit search scope and consistent discoverable method/error contracts.
- Supply source-backed authoring schemas, small typed Python/JS helpers and disconnected validation/fixture tooling; required capabilities fail early.
- Declare/manual-check input applicability so incompatible Clipboard actions do not launch unnecessarily.
- Make settings entry direct; typed native file/folder selection preserves current draft; show existing resolver hosts for selection.

## Acceptance
Actual script/runtime/manual UI, GPUI pointer+keyboard/destructive cancel+confirm/current identity, temporary config transaction, event pause/recovery, >32-item traversal/cursor invalidation, legacy methods/packages, capability/schema/SDK validation and path-picker late/cancel boundaries have regression coverage. No source/test-only claim of visual/native/installed acceptance.

## Constraints
Rust state/layout, GPUI rendering, C# Windows API. Original trust/enable/secret/cancel/process cleanup/receipts/causality remain authoritative. No new framework/dependency/vendor patches, no raw stderr/payload diagnostics. No image/blob APIs, proxy HTTP/storage/progress services, resident W10 or arbitrary UI injection. No live window manipulation, real user clipboard/credentials/config/native picker acceptance, package/service/product launch, delegation, commit/push/archive. Preserve concurrent work.
