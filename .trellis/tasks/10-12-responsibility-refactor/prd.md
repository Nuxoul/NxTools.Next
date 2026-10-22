# Responsibility-focused refactor

## Approval and scope
User explicitly requested a whole-project inspection and immediate targeted refactoring of verified overdesign and mixed responsibilities, keeping current behavior unchanged. Coverage includes all Rust apps/crates, Desktop controllers, C# PlatformHost/tests, browser extension, installer and build scripts. Preserve all accumulated dirty feature/UI/installer work. Do not package/install/run real service, clipboard, Shell, browser or unrestricted scans.

## Findings and intended seams
- DownloadEngine central orchestration mixes browser admission/handoff, queue mutations and read-model projection with transfer/runtime state. Keep the same public interface and one shared state owner; relocate cohesive implementation responsibilities without adding pass-through managers/traits.
- Desktop DownloadApplication mixes artifact Shell lifecycle with configuration, FFmpeg, overlay and general event routing. Consolidate artifact request/result/timeout/disconnect/drag/deletion ownership and duplicated failure copy. Keep the same application/entity/window owner and private internal seam.
- Core: separate application paths, process-instance locking and logging from configuration models/persistence. Preserve root exports, exact locking/atomic-save behavior and existing tests; no generalized storage abstraction.
- Clipboard library: separate cohesive category/source maintenance and query/paging from capture/retention while retaining the same SQLite owner and public interface.
- IPC: consolidate duplicated pending-before-enqueue implementation without changing caller-specific errors/generation ownership.
- Inspect Finder/Indexer/Network/Protocol/CLI/BrowserHost, C# native owners, extension and scripts. Refactor only concrete unnecessary abstractions or duplicate truth; record useful complexity retained and unmodified areas explicitly.

## Acceptance
1. Same public interfaces, command/result serialization, task/window identities and store-memory-event ordering.
2. Function-preserving extraction evidence and explicit record of actual simplifications.
3. Owner unit/integration tests, strict scoped lint/check/fmt and affected source contracts; preserve test names/counts and update source-location assumptions only.
4. Record baseline failures separately from introduced failures. Earlier capability task remains distinct; this refactor does not certify visual/native/release completion.
5. No mechanical line-limit splitting, new dependency, framework replacement, arbitrary universal manager or broad unrelated cleanup.
