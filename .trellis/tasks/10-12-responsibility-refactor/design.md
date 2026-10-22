# Refactor design and invariants

## Smallest gap
Current externally visible behavior is the baseline. The gap is maintainability: unrelated workflows share central orchestration files, repeated artifact terminal copy is spread across notifications, and unused compatibility predicates may remain callable without production responsibility.

## Planned organization
- Download crate: existing DownloadEngine interface and shared state remain. Browser handoff/admission, queue operations and read-model projection receive cohesive private implementation modules only where actual call-chain ownership supports them. No strategy/factory/service layer is added.
- Desktop: downloads/artifact_actions.rs owns artifact target resolution, caller-ID dispatch, pending terminal/timeout/disconnect, drag and delete presentation lifecycle. General notification routing delegates into the same DownloadApplication entity. A small private failure kind consolidates exact existing copy, not a generic application-wide operation framework.
- Other owners: act only on source-backed unnecessary wrappers/duplicate decision logic. Preserve schema/legacy fields and native/test adapters where they have actual compatibility or controlled-variable responsibilities.

## Unchanged contracts
One shared engine/app owner; no new cross-entity callbacks. Preserve public interface, task IDs/queues/store commit-memory-event order, manual/runtime/per-task minimum cap, exact errors/log event names, same window generation success-only close, quiet OpenWith cancellation, original deadlines, OLE worker and pending-before-send order. Rendering/native show/hide and approved UI composition are not redesigned.

## Verification
Run existing compiled Desktop650 tests as before baseline; compare behavior after source rebuild. Download unit and engine/runtime/browser/queue integration contracts remain enabled. Keep source tests but point them at real new owning modules rather than deleting assertions. Strict scoped lint, workspace test compile, changed-file format and relevant architecture gates. Function-extraction hashes/mapping distinguish relocation from actual simplification; no claim of whole-repository perfection or installed/native visual certification.
