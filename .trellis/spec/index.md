# NxTools.Next engineering and UI standards

This is the first-party standards navigation entry. Read [authority and architecture](shared/architecture.md), [engineering contracts](shared/engineering.md), [verification](shared/verification.md) and the applicable owner below before implementation. Specs describe actual source-backed contracts; approved target rules and known gaps are labeled separately. They are not a claim that every product surface has passed release acceptance.

## Authority
1. [AGENTS.md](../../AGENTS.md): architecture, migration references, diagnostics, build-memory and native/release constraints.
2. Verified [reference notes](../../docs/reference-code/download.md) preserve migrated behavior; intentional behavior change requires an approved task. Download references inspect ABDM before legacy NxTools. Listary evidence is managed boundary only.
3. Approved owning task defines product scope and stage order. [Staged UI design](../tasks/10-02-staged-desktop-ui/design.md) is the approved visual target; [UI target/current matrix](shared/ui-design.md) resolves earlier design/code differences.
4. This tree supplies executable local guidance; historical audits/plans remain evidence, not a silent override of approved intent.

## Owners
| Owner | Navigation | Source responsibility |
| --- | --- | --- |
| Desktop | [backend](nxtools-desktop/backend/index.md), [UI](shared/ui-design.md) | GPUI views, application state, worker composition, native command routing |
| Download | [backend](nxtools-download/backend/index.md) | Request/task/plan, HTTP/media, scheduling, artifact/SQLite ownership |
| Archive | [backend](nxtools-archive/backend/index.md) | Host-neutral bounded metadata/codecs/control; partial Phase A ZIP and isolated RAR4/5+7z native-output fixtures, not full safety/UI completion |
| Clipboard | [backend](nxtools-clipboard/backend/index.md) | History, query cursors, retention, archive/assets, migration staging |
| Finder | [backend](nxtools-finder/backend/index.md) | Catalog/ranking/query/command preparation and usage history |
| Extensions | [backend](nxtools-extensions/backend/index.md) | Host-neutral package identity/registry/manifest and bounded cross-feature invocation contracts |
| Indexer | [backend](nxtools-indexer/backend/index.md) | Rust MFT/USN, mmap base+delta, authenticated service sessions |
| Core | [backend](nxtools-core/backend/index.md) | Settings/defaults, section persistence, paths, instance/log guards |
| Network | [backend](nxtools-network/backend/index.md) | Immutable operation route snapshot and client construction |
| Protocol | [backend](nxtools-protocol/backend/index.md) | Typed versioned PlatformHost/indexer wire contracts |
| IPC | [backend](nxtools-ipc/backend/index.md) | Bounded frames/queues, command IDs, connection generations, pipes |
| Download IPC | [backend](nxtools-download-ipc/backend/index.md) | Current-user download host/client, readiness and command/events |
| Browser host | [backend](nxtools-browser-host/backend/index.md) | Native Messaging validation, handoff, bounded Blob intake |
| CLI | [backend](nxtools-cli/backend/index.md) | Argument parsing, exit/output contracts, download-host policy |
| C# PlatformHost | [native boundary](platformhost/index.md) | Windows APIs/STA/OLE, hooks, native results and protected deployment |
| Browser extension | [web boundary](browser-extension/index.md) | MV3 capture/handoff/content/popup, browser APIs and local settings |

Native and extension topics are navigated here without inventing Rust workspace packages or changing Trellis runtime registration. Existing backend topic paths remain stable for task JSONL consumers. [Thinking guides](guides/index.md) are prompts for reasoning, not additional product policy.

## Use and quality
- Follow package index pre-development and quality check sections; read linked topic files, not only the index.
- Desktop UI selection starts with [component-first rules](shared/ui-component-selection.md), [current 0.10.0 release/API delta](../../docs/reference-code/gpui-kit-0100.md), then [historical79-entry table](../../docs/reference-code/gpui-kit-component-table.md) and [API index](../../docs/reference-code/gpui-kit-component-index.md). Current 0.10.0/default pre0.3.12 supersedes old pins, not approved layout/variant/parent-transformation or business/native contracts. Type adoption is not visual acceptance.
- Shared rules have one home; package topics name real sources/tests and exceptions. Do not duplicate changing constants across docs.
- [Installed-feedback boundaries](shared/installed-feedback.md) covers stage-specific 7z workspace, namespace icon capability, About download admission and heading pointer behavior.
- [Foundation gaps](shared/foundation-backlog.md) are explicit differences, not exemptions declaring compliance. Do not implement those gaps in a documentation-only task.
- Run `python tests/architecture/spec-contracts.py` for document navigation/template/evidence/scenario checks. This does not validate UI, production behavior, external references or native release operation.
- Documents are English; user-facing Chinese labels/copy remain product behavior. Use `repo:<workspace-relative-file>#Symbol` for source evidence; the checker verifies file existence, not symbol semantics.
