# Download icon clarity and capability inquiry

## Scope

The user reports unclear download-list icons, supplies Ghost Downloader3 as a reference, asks which archive formats are supported and reports missing NxTools context menus for ordinary files/folders/multiselection.

- Improve actual native icon resolution through the existing ExtractShellIcon/PNG cache/STA owner. Preserve40DIP list geometry, current official image rendering, file/type association, bounded cache, lifecycle and all download state/actions.
- Use Ghost only as a clean-room behavioral reference. Compare suggested features with existing capabilities; do not implement unrelated download features.
- Report archive read/test/extract separately from creation/editing and supported split/encrypted variants.
- Diagnose current Shell desired settings, applied facts and registration read-only. Preserve deliberately saved menu selections and original explicit Apply/Revert/grouped selection. No registry write, installer, deployed-binary replacement or service action is authorized by this inquiry.

## Verification boundary

Use the existing ArchiveTypeIconTests native attribute-only query with one GUID-owned cache on the original bounded STA. Large requests must yield native image pixels sufficient for their requested sizes, without consuming legacy small-image cache entries; small/type safety, cancellation and cache bounds remain. Build and run the exact reviewed icon/cache/STA fixtures. Reference/spec changes use the document gate. Compiled PNG evidence is not installed/DPI/user visual acceptance.

## References

[Ghost and original icon boundary](../../../docs/reference-code/ghost-downloader.md), [download reference](../../../docs/reference-code/download.md), [Shell chain](../../../docs/reference-code/archive-shell.md), [format readers](../../../docs/reference-code/archive-formats.md). Existing Shell onboarding R4 keeps its owner and operational restrictions.

## Outcome

[Source delivery, current capabilities and operational follow-up](outcomes.md). Native icon/cache and pure grouped-menu fixtures pass13/13 with no skips; code/document checks pass. Installed icon appearance and actual Shell Apply/Explorer invocation remain separate operational acceptance.
