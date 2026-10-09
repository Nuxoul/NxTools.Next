# Implementation Plan

1. Read clipboard spec index/guidelines and `docs/reference-code/clipboard.md:179-260`; inspect current exact dirty diff before editing.
2. Add red exact-host/punctuation/case detector test and a GPUI route/click-feedback test at the real row callback.
3. Add red reused-window stale-timer regression; prove first capture cannot remove a newer capture before its own dwell.
4. Implement the smallest timer-generation and topmost/bounds confirmation fix; keep one HWND and current preferences.
5. Add visible action confirmation while preserving copy-only/opt-in browser defaults and typed local Shell routes.
6. Replace only the link-panel opaque Root shell with a local transparent root; test entity lifecycle/no synchronous parent re-entry and content rendering. Keep semantic radius on the inner surface only where Windows surface supports actual alpha.
7. Run focused/full desktop tests, `ui-token-boundaries.ps1`, relevant lifecycle gates, fmt/clippy and PlatformHost tests; then manual native smoke for transparent corners, clicks, topmost and timeout.
8. Update `docs/reference-code/clipboard.md` only for newly verified durable invariants; remove temporary DEBUG probes.

Expected files: `tools/link_detector.rs`, `tools/link_panel.rs`, `clipboard.rs`, relevant desktop tests, and possibly a small local transparent root component/architecture test. Avoid unrelated settings or clipboard capture changes.