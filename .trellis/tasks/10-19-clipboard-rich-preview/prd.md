# Clipboard Markdown and code preview

## Goal / authorization
User approves existing passive hover preview automatically classifying ordinary text, Markdown and code snippets, using official Kit Markdown rendering/syntax highlighting. User additionally approves local Trellis task records and implementation. Current-session only: no subagents, cross-session messages, product/package/install/service/live clipboard/history operations, implicit commit or archive.

## Requirements
- Ordinary and uncertain text stays literal. Markdown renders headings/emphasis/lists/tables/code; naked code uses conservative local language detection and official syntax highlighting.
- Offer original source and manual language correction without changing stored text or native copy/paste/edit semantics.
- No execution, automatic formatting, WebView, AI/network inference, remote image/resource loads, arbitrary local-file reads or HTML interpretation. Rendering projection only; preserve original bytes.
- Bounded classification/parse on snapshot/mode changes, never per list-row render. Large/deep/unsupported content falls back explicitly to original text.
- Keep one passive preview HWND, hidden->noactivate->final native bounds->show, reuse/frame-before-reveal, hover generation/cancellation, search focus and existing popup geometry/theme. No competing native resize or render-time parent lease.

## Acceptance
Red-capable unit plus actual production GPUI preview regressions for conservative detection, safe AST projection, exact source/readonly, real styled descendants/highlight, source/manual mode, bounds/scroll and stale lifecycle. Focused tests, full Desktop once after fixes, scoped strict lint/check/fmt and architecture/spec gates. Source/synthetic evidence does not certify physical/native/DPI/reader/user acceptance.
