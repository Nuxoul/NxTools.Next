# Installed feedback corrections

## Owners and behavior

| Slice | Existing owner | Required outcome |
| --- | --- | --- |
| 7z Open | ArchiveReadSession / ArchiveWorkerRunner / run_sevenz | Read metadata at the original 64 MiB work limit without allocating payload dictionaries; full decoder admission remains mandatory for payload work. |
| Packaged icons | QuickLaunchApplication / original ExtractShellIcon / IconCache | Valid AppsFolder results request real Shell artwork; file-only actions remain unavailable. |
| Update download | About / DownloadApplication / DownloadEngine | Explicit update check adds and starts the supported newer installer with configured download defaults; no installer execution. |
| Markdown outline | Reader / official Tree and TreeState | Heading click navigates without toggling descendants; selection, keyboard navigation and Root mouse release remain intact. |
| Fonts | Existing app-private DirectWrite collection | Assessment only; preserve selected family and package contents. |

## Reference evidence

- [Archive admission and Bandizip boundary](archive-design.md): shipped-binary reconstruction separates catalog Open from later payload decoding. No proprietary native source claim.
- [Listary icon boundary](icons-design.md): managed PIDL/image-factory path, physical size, ICONONLY and native lifetime.
- [Markdown interaction](markdown-design.md): Winc heading navigation and released Base Tree click behavior.
- [Font assessment](fonts.md): local metadata/sizes, upstream OFL 1.1 and current private-font loader.

The approved Kit/Base/Component/Assets 0.8.0 and default gpui-pre 0.3.9 remain unchanged. Existing controls, geometry, native windows and transaction owners are retained. There is no new downloader, icon cache, tree controller, IPC command or font policy.

## Lifecycle and diagnostics

Register pending requests before dispatch and defer cross-entity application work to the next turn. Retire stale/closed owners; cache hits obey the same leased-entity boundary as asynchronous completion. Preserve original cancellation, source ownership, bounded worker admission and cleanup-bound terminal results. Correlated production events use stages, outcomes, elapsed time and native codes rather than payloads.

## Verification boundary

Run serial Cargo with the workstation memory settings. Use production-boundary regressions, injected native leaves and owned temporary files. The supplied archive may be opened read-only with zero payload/output work. No packaging, installed-program replacement, automatic installer execution, real AppsFolder operations or global desktop interaction is included. Compiled/fixture results and installed/native/visual acceptance are reported separately.
