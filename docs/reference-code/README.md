# Reference code knowledge base

This directory is the durable index for behavior already investigated in NxTools and the supplied
reference repositories. It prevents each change from rediscovering the same implementation through
keyword search.

## Required workflow

1. Read the matching feature note here before searching source code.
2. Follow its recorded entry files and call chain.
3. Search only when the note has a gap or the referenced source changed.
4. Update the note with newly verified behavior, exact paths and failure invariants before editing
   NxTools.Next.
5. Never treat an unverified guess as a reference fact. Mark it `not inspected` instead.

## Catalog

| Feature | Knowledge note | Reference status |
| --- | --- | --- |
| Clipboard capture, image compatibility and paste | [clipboard.md](clipboard.md) | Indexed |
| Clipboard candidates: ClipKeep and TieZ | [clipboard-clipkeep-tiez.md](clipboard-clipkeep-tiez.md) | Both cloned/source-reviewed at fixed revisions; [comparison](../audits/2026-10-19-clipboard-reference-comparison.md); no runtime/benchmark/direct visual acceptance |
| Currency catalog, aliases and searchable target | [currency.md](currency.md) | User-requested fawazahmed0 paired catalog/CNY schema, CC0 snapshots and official searchable Select verified; source/test evidence, not installed acceptance |
| Download manager | [download.md](download.md) | Indexed; phase 0 contracts and test fixture complete |
| Quick Launch / Finder | [quick-launch-research.md](quick-launch-research.md) | Investigation in progress |
| Product shell, startup and notification area | [system-shell.md](system-shell.md) | Indexed |
| Floating assistant | [assistant.md](assistant.md) | Legacy constructors/show/close/native styles, image/GIF, pointer/drop/edge/download source inspected; new slice planning, no runtime acceptance |
| GPUI/Zed rendering patterns | [gpui.md](gpui.md) | Indexed for P0 visual components |
| Unified Markdown display/edit | [markdown-editor.md](markdown-editor.md) | Fixed Apache0.1.0 editor, passive/interactive four-surface integration; original payload/native ownership retained |
| Graphite default themes | [graphite-theme.md](graphite-theme.md) | Approved project Light/Dark via public Kit theme APIs; optional official themes retained, actual96DPI captures separate from user acceptance |
| GPUI Kit current0.8.0 release/API migration | [gpui-kit-080.md](gpui-kit-080.md) | Current Kit0.8.0/defaultpre0.3.9, two narrow carried patches, API/behavior delta and evidence limits; read before historical tables |
| GPUI Kit component table and AI rules | [gpui-kit-component-table.md](gpui-kit-component-table.md), [offline searchable viewer](gpui-kit-components.html), [short mandatory rules](../../.trellis/spec/shared/ui-component-selection.md) | Current 0.8.0 authority precedes preserved 2026-10-15 website 0.7.3/79-page and source 0.7.1/78-entry audit; historical Diff limitation is superseded; variants/Toolbar/tile rules remain |
| GPUI Kit historical API discovery | [gpui-kit-component-index.md](gpui-kit-component-index.md) | Detailed 0.7.1 source map; current 0.8.0 delta linked first; historical Base/Shell navigation not refreshed; availability is not adoption |
| GPUI Kit historical release/native contracts | [gpui-kit-071.md](gpui-kit-071.md) | Historical published API/ownership and narrow native exceptions; current active versions/provenance are in gpui-kit-080.md |
| ShellRS GPUI desktop composition | [shellrs-ui.md](shellrs-ui.md) | Source-reviewed visual precedent; gpui-kit/version/license boundaries explicit |
| Steward launcher UI/features/performance | [steward.md](steward.md) | Supplied MIT snapshot/source + static screenshot reviewed; benchmark reports not rerun |
| WinShun launcher / plugin candidates | [winshun.md](winshun.md) | Supplied MIT0.7.3 source reviewed; W1–W10 deferred backlog, no implementation/runtime/benchmark acceptance |
| Archive manager (Bandizip-style compress/extract) | [archive.md](archive.md) | Bandizip7.32 public resources (no decompilation); Archiver/SupaZip reviewed; proposed design only, no implementation |
| HatchZip space/progress/batch/crash/reclamation reference | [hatchzip.md](hatchzip.md) | Local0.1.3 snapshot without Git metadata; five source-backed topics and SHA256/symbol manifest; no runtime/tests/native acceptance, destructive mode remains separate design |
| Multi-format archive editing local-source review | [archive-editing.md](archive-editing.md), [pinned manifest](archive-editing-manifest.json) | bit7z/7-Zip/NanaZip/Ark/PeaZip actual add/delete/rename chains, solid/encryption/volume restrictions, unsafe replacement patterns and per-file licenses; source only, no new backend approval |

## Entry format

Every feature note records:

- user-visible behavior and invariants;
- exact source files and each file's responsibility;
- the verified call chain;
- what belongs to Rust, C# and GPUI in NxTools.Next;
- tests and diagnostics that prove the migrated behavior;
- open questions, explicitly separated from verified facts.

This is a navigation module, not a generated symbol dump. Its interface is the feature catalog;
implementation details stay in focused notes so callers learn one small entry point.
