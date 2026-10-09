# Engineering and UI standards foundation — 2026-10-09

## Outcome and boundary
The approved standards-only deliverable is implemented. First-party navigation, practical package topics, native/web guidance, UI target/current/exception matrix and a small deterministic spec checker are present. This is **not** a product UI migration, blanket declaration of code compliance, or native/release acceptance. Foundation and subsequent UI stages were not started.

Single parent owner wrote and integrated all docs/scripts. One SOL read-only helper collected bounded C#/extension evidence; no implementation delegation or extra review loop. All existing dirty changes retained, no reset/reparent/commit/archive. Current child remains in_progress under workflow; parent and existing staged UI remain planning, old bootstrap task unchanged. No Cargo/.NET build, product launch, package/install, service/registry/DOpus action or real user payload inspection occurred.

## Coverage and authority
- New entry: [.trellis/spec/index.md](../../.trellis/spec/index.md), linked from AGENTS and README. Historical README status and UI/reference numeric intent are identified as historical, not silently rewritten.
- Twelve real Cargo owners retain all72 existing backend topic paths: Core, Network, Protocol, IPC, Download IPC, CLI, Download, Clipboard, Finder, Desktop, Indexer and BrowserHost. Source-backed structure/persistence/errors/logging/quality replace generic template boilerplate. All package indexes route pre-development and quality commands.
- Standalone [C# PlatformHost](../../.trellis/spec/platformhost/index.md) and [browser extension](../../.trellis/spec/browser-extension/index.md) topics describe actual native/MV3 layers without fake Rust packages or Trellis registry/runtime edits.
- Shared [architecture](../../.trellis/spec/shared/architecture.md), [engineering](../../.trellis/spec/shared/engineering.md) and [verification](../../.trellis/spec/shared/verification.md) centralize cross-layer rules rather than duplicate them across owners. Typed errors/IDs, GPUI lease/cached completion, generation cancellation, locked config/storage authority, bounded workers/queues and payload-safe diagnostics point to actual sources/tests.
- SQL accurately limited to Download/Clipboard/Finder owners. Indexer versioned mmap base+delta is not SQL. Other topics explain non-applicability and actual persistence owner; no invented ORM/schema/WAL guarantee.
-15 original concrete scenario bodies retained across9 documents; one earlier custom-client HWND invariant list additionally gains seven-section form without dropping its list. Total current tree87 Markdown docs with20 executable scenarios and259 repo evidence markers before final records. Automated evidence markers validate file existence, not symbol/semantic truth; source excerpts were inspected separately.

## UI resolution and handoff
[UI contract](../../.trellis/spec/shared/ui-design.md) makes the later approved staged design authoritative: approved palette,4px layout grid,24/28/32 control roles and3/6/8/pill radius. Existing old Zed palette,18/22 controls,2/6 layout offsets and4px radius remain observed differences, not automatic approved exceptions. Existing native content/geometry/focus/lease constraints survive; no mechanical pixel/color replace.

Required actionable-state matrix covers default/hover/pressed/focus-visible/selected/disabled/loading/empty/error/success, keyboard/pointer equivalence, actual hit bounds, stable geometry and actionable saved-vs-applied feedback. Actual keyboard/provider accessibility, contrast and DPI need real evidence; token/tooltip presence isn't certification. Reduced-motion common desktop plumbing is not implemented; popup CSS support doesn't imply shadow-overlay support.

[Gap ledger](../../.trellis/spec/shared/foundation-backlog.md) has13 explicit evidence-backed entries. Main handoff: palette/density/radius mismatch, focus/disabled/keyboard/provider proof, reduced-motion missing seam, link timer/topmost/alpha acceptance, extension filter ARIA/shadow motion, config initialization distinction, large-module scope discipline and native log disposal distinction. They are unresolved product/verification work, not suppressed violations or permission to implement now.

Canonical staged order remains **Foundation → Downloads → Quick Find → Clipboard + Tools → Global settings + supporting surfaces**; existing owner plans and prior-stage user visual acceptance remain required. No utility implementation or evaluation started.

## Checker and regression evidence
`tests/architecture/spec-contracts.py` uses Python3.11+ standard library, reads only repo specs/manifests/evidence files, never executes product/Cargo/external requests and doesn't write. Bounded250 docs/512KiB per doc. It validates root/owner/topic navigation, local Markdown links, explicit source-file evidence, template residue, seven nonempty contract sections and code fences, plus presence of approved UI target/current matrix values. It cannot prove symbols, semantic code compliance, UI behavior/accessibility/native/release acceptance; no all-old-code allowlist.

`tests/architecture/spec-contracts.tests.py` uses PI_SCRATCH_DIR temporary synthetic repositories; tests real CLI return0/1 and read-only behavior. Initial7 passed; added empty-section/unclosed-fence tests were observed red before implementation. A legal nested Wrong/Correct subheading exposed a false-positive and was separately red/green fixed without weakening document examples. Final10/10 pass.

## Verification actually run
| Check | Result |
| --- | --- |
| `python tests/architecture/spec-contracts.tests.py -v` |10/10 pass; good/bad fixtures entirely in scratch |
| `python tests/architecture/spec-contracts.py` |87 docs/12 workspace owners pass; no template/local-link/evidence/scenario violation |
|13 existing static architecture gates |All pass; no check removal/weakening |
| PowerShell-expanded `node --test` extension files |21/21 pass,0 skip/fail |
| direct Blob policy assert script |Pass |
| Original source/asset SHA256 comparison |503/503 unchanged; covers rust/csharp/browser_extension/vendor/manifests/installer, excludes bin/obj/node_modules |
| Original scenario text retention |15/15 bodies across9 docs retained,0 loss |
| `git diff --check` |Pass |

Static gates: ui-token-boundaries, ui-consistency, ui-state-lifecycle, download-ui-lifecycle, clipboard-lifecycle, clipboard-ui-lifecycle, quick-launch-lifecycle, desktop-shell-lifecycle, file-dialog-window-follow, file-dialog-search-lifecycle, browser-native-messaging, package-memory-policy and indexer-lifecycle. These are source gates, not live product certification; package-memory-policy execution does not package.

Session-local logs: standards-checker-tests.log, standards-checker-empty-fence-red.log, standards-checker-subheading-red.log, standards-final-checker.log, standards-architecture.log, standards-extension.log and standards-diffcheck.log. Source/scenario snapshots in scratch establish scope preservation; not checked-in release artifacts. Detailed source mapping is [task research](../../.trellis/tasks/10-09-quality-standards-foundation/research.md).

## Remaining limits and next eligible work
No blocker remains for the bounded documentation/checker deliverable. Product code does not thereby satisfy approved future UI target. The engine103/103 fixture-only closeout is current evidence, not an unresolved blocker; original historical failure evidence is in [SOL report](2026-10-08-sol-review-remediation.md). Real user pressed-pointer/probe/release/DPI and LocalSystem idle/restart/base+delta/search/runtime-hash gates still pending; actual registry/installer and development/Validation service compatibility unverified. No compile/native/product checks were rerun in this docs-only turn.

Phase3.3 knowledge is recorded in the specs. Commit/archive remain unauthorized. Existing Foundation owner must refresh scoped baseline and review using these specs before product work; don't auto-advance from this standards handoff.
