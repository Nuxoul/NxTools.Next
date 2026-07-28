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
| Download manager | [download.md](download.md) | Indexed; phase 0 contracts and test fixture complete |
| Quick Launch / Finder | [quick-launch-research.md](quick-launch-research.md) | Investigation in progress |
| Floating assistant | `assistant.md` | Not inspected for the current slice |
| GPUI/Zed rendering patterns | `gpui.md` | Not inspected for the current slice |

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
