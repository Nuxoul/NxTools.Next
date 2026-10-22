# Clipboard first-tier efficiency improvements

## Goal and authorization
User2026-10-19 approves the first tier in this exact delivery order: derived list thumbnails, number/page shortcuts, lightweight list projection, temporary capture pause. User explicitly confirms regression seams: temporary SQLite/derivatives; actual production GPUI panel; C# pause configuration/result/reconnect. Synthetic data only. Coordinate session c15113dd-0357-4913-adab-8ffe48d5aa23; no concurrent shared-file writes or Cargo/.NET/resource probes.

## Requirements
- List thumbnails retain original assets and native-format paste semantics, use bounded background work, deterministic derivative identity and reference-safe cleanup/migration behavior. Old records gain derivatives lazily; render performs no decode or disk write.
- Ctrl+1..9 activates the corresponding currently displayed history result, PageUp/Down moves by a viewport-sized group, with actual search/IME/editor/modal/pending guards and one dispatch. Existing Enter/Ctrl+Enter, selection, pagination and failure retry remain.
- Worker-facing summary page APIs bound list text while searching the complete stored body. Action/edit/preview/extension/open-target paths resolve active full item by immutable ID; missing/deleted/stale completion cannot act on another row or dispatch truncated text.
- Temporary pause is runtime state, not persistent enabled=false. Offer15minutes and until resumed, explicit Resume and durable paused/transition status. Native acknowledges before UI claims paused; no capture reads while acknowledged paused. Rust guards in-flight captures. Existing history and paste remain usable; deadline/reconnect reapply desired state, no restart persistence. Failure remains explicit/retryable.
- Keep300x440/default resize, existing Kit Default themes/official controls, single passive preview, native bounds/frame lifecycle, original query controller and deferred cross-entity work.

## Exclusions
No search/tag/cloud/OCR/AI/queue feature, dedup algorithm change, palette/theme/library upgrade, new clipboard listener, archive implementation or shared ZIP change. No package/install/service/live clipboard operation, automatic commit or task archive.

## Acceptance
- Red-capable real boundary regressions precede each change; capture actual failed run when the shared verification lane is released.
- Synthetic large-text tail search and exact full actions, stale/deleted/cancel cases; image bounds/alpha/original bytes/cache lifecycle; actual GPUI number/page/IME/modal/pending/held behavior; C# native-read guard and typed pause application fixtures.
- Relevant owner tests and full desktop suite once, scoped strict lint/check/fmt, Clipboard architecture and spec gates, no duplicate build or unrelated file rewrites.
- Record measured synthetic projection/decode byte/time results without calling them installed benchmarks. Visual/native/mixed-DPI/user acceptance remains separately reported.
