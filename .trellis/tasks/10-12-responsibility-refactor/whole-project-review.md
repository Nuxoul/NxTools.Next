# Whole-project responsibility and overdesign review

## Scope and evidence
The user clarified that inspection covers the whole project, not only downloads. Reviewed all12 Rust owner families and the C# PlatformHost, extension, installer and five build/deployment scripts. Inspection used module interfaces, call chains, state/IO ownership, existing tests and current dirty source; file length was a navigation signal, not a defect by itself. Generated binaries/manifests, vendor upstream and historical artifacts are not targets for cosmetic restructuring. This is a targeted source-backed review, not an exhaustive correctness/security proof of every line.

Starting snapshot: session scratch `whole-project-refactor-start/{status.txt,working.diff,inventory.json}`. Existing feature/UI/installer changes are retained. Earlier capability acceptance is separate.

## Concrete changes
| Area | Finding | Targeted treatment | Responsibility retained |
|---|---|---|---|
| Core | Directory discovery, process locking/hints and log initialization/retention share configuration entry file | Private paths/instance/logging modules; root exports unchanged | Configuration schema/defaults/normalization and locked atomic section save remain one configuration owner |
| Clipboard library | Category/source maintenance and SQL query/paging obscure capture/retention workflows | Private categories/query implementations on same ClipboardHistory | One SQLite connection, same captures/native formats/assets/retention/archive/storage ownership;33 function bodies preserved |
| Download engine | Browser admission/Blob compensation, queue scheduling and read projection intermixed with transfer orchestration | Existing cohesive private browser/queue/projection modules integrated; repeated snapshot search predicate unified | Same DownloadEngine/shared state/store-before-memory-before-event ordering; no strategy/factory/manager layer |
| Desktop Downloads | Artifact request/result/timeout/disconnect logic spread across controller | artifact_actions private same-entity implementation; four repeated failure presentations consolidated | Same pending IDs/HWND generations/success-only close/quiet chooser cancellation/OLE gesture |
| IPC | Ordinary and summon entry points duplicate pending registration and queue rollback | One private CommandRouter::enqueue implementation | Caller-specific duplicate/error mapping, same generation/terminal owner; no second queue |
| C# | Failure method merely forwards all arguments; obsolete initial-authorization compatibility predicate has no production role | Remove forwarding method and obsolete predicate/helper-only test; actual deployment decision tests retained | Existing failure executor/cancellation/result admission, protected tuple and UAC owner |
| Extension | Unreachable preemptive-download owner and unused background Blob-reader wrapper remain alongside active safe handoff | Remove dead state/classifier/dispatch/reader branch; preserve active capture and content fallback | Demand-only Native Messaging, accepted-before-browser-cancel, identity/Blob lease and partial-success policies |
| BrowserHost | Request ID and kind helpers each decode the same full JSON | One bounded diagnostic metadata decode, followed by unchanged typed request validation | Same independent field bounds/fallback/error precedence and response identity |
| Desktop Link panel | Session stores identical GPUI handles in two fields | One session window handle; reuse passes that same identity into restaging | Native HWND, view handle, timer and generation remain distinct owners; close/rebuild regression extended |

Relocation and semantic simplification are distinguished. Clipboard is a responsibility relocation, not a claim that its SQL needs less logic. Core extraction does not rewrite persistence. Download/IPC/extension/C# also remove actual duplication/dead code.

## Useful complexity retained
- Native STA runners, HWND resize coordination, OLE ownership, authorization/session adapters and browser registration compensation enforce real boundedness/lifetime/readback/rollback contracts. Removing them would spread required ordering into callers.
- Indexer MFT/USN/mmap base+delta, generations/cancellation, grouped name scoring and candidate fallback preserve persistence and query semantics. Listary native source is unavailable; no proprietary engine review claimed.
- Finder prepared match material is derived catalog state built at mutation, not an independent authoritative catalog. History/ranking and query parsing remain existing owners.
- Network operation snapshots freeze route choices during asynchronous IO. The snapshot is not a redundant copy of live configuration.
- Protocol DTOs and transport admission stay separate; wire compatibility is not coupled to GPUI entities.
- Separate Clipboard worker and native preview presentation owners keep SQLite off render callbacks and prevent leased-entity reentry. No new cross-entity controller is introduced.
- Browser cache ID/URL lookup indexes and sampled-vs-pending theme publication represent different needs; similar fields/classes are not enough evidence to delete them.
- Installer exact-path/same-handle/hash checks and replacement/recovery records are necessary; upgrade and uninstall ownership intentionally differ.
- Large test files contain fixtures and regression evidence, not release behavior. `#[test]` marks a test run by Cargo; `#[cfg(test)]` excludes its enclosing code from ordinary product builds. Tests are not an overdesign finding and are not deleted to shorten files.

## Separately recorded follow-ups, not silent behavior changes
1. Extension cache alias/capacity issue was separately reproduced and corrected2026-10-13: unique-resource capacity, object-scoped removal in both indexes, preserved capture/handoff identity and restore aliases.16production-entry regressions/full44Node tests pass; see [cache-alias-outcomes.md](cache-alias-outcomes.md). Installed MV3 acceptance remains unverified.
2. C# verification selection was separated2026-10-13:189reviewed fixtures default/build,23native and5currently unisolated fixtures outside default; explicit authorization/filters, zero-match rejection and honest SKIP summary. Original211tests retained plus6selection regressions. See [native-test-selection-outcomes.md](native-test-selection-outcomes.md). Production CRX signing/cache and fixed temporary-path fixture isolation remain separate follow-ups.
3. CRX signing test isolation and duplicate signing identity were completed2026-10-13:explicit tempPEM export,one internal path/SPKI-ID/read-existing owner,read-only registration,distinct source fallbacks and original Firefox/native transaction preserved.7new signing regressions plus original package fixture/selection contracts pass(16safeRUN/PASS);see [browser-signing-outcomes.md](browser-signing-outcomes.md). Current224catalog197Fixture/23native/4restricted. Worker transport/cache extraction and remaining icon-cache/fixed-temp-path isolation remain separate candidates.
4. Indexer persistence-error visibility was separately reproduced and corrected2026-10-13: per-volume/settings fault state survives unrelated success, current snapshot save retry and same-settings retry clear only matching committed faults.9new maintenance/temp-file/wire regressions and13existing filtered regressions pass; see [indexer-persistence-outcomes.md](indexer-persistence-outcomes.md). LocalSystem release gates remain deferred.
5. CLI reported-fault duplication was simplified2026-10-13 with one private helper used only by schedule/add/list,retaining Add=true/others=false and empty-message convention.36captured synthetic-pipe process cases preserve text/JSON/requestID/single emission/exits; intentional duplicate mutation went red and was removed. Original10tests plus output matrix pass,strictClippy/workspace/fmt pass;see [cli-reported-fault-outcomes.md](cli-reported-fault-outcomes.md). Watch/batch/connection policies remain unchanged.

## Verification
Final compiled/static results are recorded in outcomes.md. No packaging/install/live service/Shell/browser/clipboard/real-volume operations or automatic commit/push are authorized by this refactor.
