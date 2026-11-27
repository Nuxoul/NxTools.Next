# Reference code knowledge base

This directory is the durable index for behavior already investigated in NxTools and the supplied
reference repositories. It prevents each change from rediscovering the same implementation through
keyword search.

## Persistent Listary reference

The canonical local archive for Listary 6.3.5.94 is `D:\Code\Listary.Decompiled\6.3.5.94`.
Its `README.md` indexes `managed\Listary` (C#/XAML), `expanded\Listary` (explicit compiler-generated state machines), the original binary snapshot, inventory and repeatable export script. The archive is independent of this repository, Trellis tasks and session scratch. Resource-enabled window code uses `.xaml.cs` filenames. Native engine binaries are retained as inputs, not represented as recovered C# source.

## Required workflow

1. Read the matching feature note here before searching source code.
2. Follow its recorded entry files and call chain.
3. Search only when the note has a gap or the referenced source changed.
4. Update the note with newly verified behavior, exact paths and failure invariants before editing
   NxTools.Next.
5. Never treat an unverified guess as a reference fact. Mark it `not inspected` instead.

Current framework lookup follows [GPUI Kit0.10.0/defaultpre0.3.12](gpui-kit-0100.md): `vendor/gpui-component-0100`, `vendor/gpui-base-0100`, `vendor/gpui-pre-0312` and `vendor/gpui-pre-windows-0312`. Earlier versioned source snippets and outcomes in feature notes are dated evidence, not active lookup pins. Their behavior, lifecycle and native ownership rules remain in force unless explicitly amended; current API availability is not adoption or acceptance.

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
| Markdown reader, editing, export and versions | [markdown-reader.md](markdown-reader.md) | mdview Pro 2.0.21 resources/official behavior, locked Kit example and APIs; native reader behavior not yet verified |
| Graphite default themes | [graphite-theme.md](graphite-theme.md) | Approved project Light/Dark via public Kit theme APIs; optional official themes retained, actual96DPI captures separate from user acceptance |
| GPUI Kit current0.10.0 release/API migration | [gpui-kit-0100.md](gpui-kit-0100.md) | Current Kit/Base/Component/Assets0.10.0/defaultpre0.3.12; approved Component1/Base5/core2/Windows4 union retains notification, editor-host, noactivate and material amendments; Fast disabled; read before historical tables |
| GPUI Kit component table and AI rules | [gpui-kit-component-table.md](gpui-kit-component-table.md), [offline searchable viewer](gpui-kit-components.html), [short mandatory rules](../../.trellis/spec/shared/ui-component-selection.md) | Current0.10.0 authority precedes preserved2026-10-15 website0.7.3/79-page and source0.7.1/78-entry audit; variants/Toolbar/tile/lifecycle rules remain; no new visual audit |
| GPUI Kit historical API discovery | [gpui-kit-component-index.md](gpui-kit-component-index.md) | Detailed0.7.1 source map; current0.10.0 route linked first; historical Base/Shell navigation not refreshed; availability is not adoption |
| GPUI Kit historical release/native contracts | [gpui-kit-080.md](gpui-kit-080.md), [gpui-kit-071.md](gpui-kit-071.md) | Dated release/API/native evidence; current versions and active vendor provenance are in gpui-kit-0100.md |
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
