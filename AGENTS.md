# NxTools.Next development rules

## Reference-driven migration

Reference review is a blocking prerequisite, not an optional follow-up. For any feature that
already exists in NxTools, do not design, patch, or "improve" the new implementation from memory
or from the current NxTools.Next code first. The old implementation is the behavioral source of
truth unless the user explicitly requests a changed behavior.

When a feature has a checked-in phased plan, its phase names and order are canonical. Before
reporting or starting a “next phase,” reread that plan and use its exact phase title. Stabilization
or catch-up work remains part of the current phase; do not renumber it or pull later-phase work
forward unless the user explicitly approves the reorder and the plan is updated.

Before implementing or changing a migrated interaction:

1. Read the matching note in `docs/reference-code/` first.
2. Follow its recorded files in `D:\RiderProject\NxTools` and the supplied reference projects.
3. Search source only to fill an index gap or verify changed code; update the note with every newly
   verified entry file, call chain and invariant before implementing.
4. Write down the layout, lifecycle, cancellation, placement, focus, and failure-state invariants that must survive the migration.
   Trace the implementation through constructors and base classes to the real native show/hide
   boundary; leaf view/controller files alone are not sufficient lifecycle evidence.
5. Add a red-capable regression test at the real boundary before changing code.
6. Preserve the architecture boundary: Rust owns state and layout, C# owns Windows APIs, and GPUI owns rendering.
7. Do not declare the slice complete from compilation or screenshots alone; verify its interaction lifecycle.

For every GPUI surface, an entity callback must not synchronously update an application/entity that
can write back into the currently leased entity, including cache-hit paths. Update local view state
in the current callback, schedule cross-entity work for the next application turn, and protect the
boundary with a lifecycle regression gate.

For clipboard hover preview specifically, keep one passive preview window, update/reposition it in place, hide it when inactive, and never synchronously re-enter an already leased GPUI entity.

For download behavior and interaction, inspect AB Download Manager first, then the old NxTools
implementation for product-specific compatibility. Do not combine two competing interaction
frameworks when the reference uses one native gesture boundary; Windows Shell drag-and-drop is one
OLE loop owned by C#, initiated directly from the still-pressed GPUI pointer callback. Because C#
runs in PlatformHost instead of the visible GPUI process, the command must carry the source HWND;
C# must cancel that window's existing pointer capture and verify the initiating button is still
pressed before entering Ole32 `DoDragDrop` on a dedicated STA worker. A native drag may outlive a
single pointer callback and must never block the serial platform-command reader or the shared
Windows event thread; Shell open/reveal and hotkey/clipboard processing remain independently
responsive.

For Quick Launch preview, keyboard and pointer selection must feed the same generation-scoped preview
controller. Create the auxiliary preview HWND hidden, apply `WS_EX_NOACTIVATE` through the C# Windows
adapter, atomically apply its final device-pixel bounds, then show it; a hidden GPUI window may still
have `CW_USEDEFAULT` native bounds. After first creation, restore both the primary Windows HWND and
GPUI search input focus; do not make the primary search panel non-activating.

## Diagnostic contract

Every new user-facing feature and every cross-process operation must define its diagnostic events
before the slice is considered complete. Carry one correlation ID across Rust, IPC and C#; record
accepted/start, meaningful state transitions, terminal success or failure, elapsed time and native
error codes at the boundary that owns them. Never log clipboard contents, credentials, request
headers, cookies or other payload secrets.

Diagnostics must remain cheaper than the feature. Do not log animation frames, pointer movement,
download byte ticks or other high-frequency updates. Coalesce those streams into state changes,
bounded milestones or rate-limited repeated failures. Logging uses bounded/non-blocking sinks and
must never change the result of the operation it observes. Temporary probes use a unique
`DEBUG-...` marker and must be removed after the real interaction turns green; the low-noise
production lifecycle events remain.

When formatting PowerShell diagnostic output, assign the output of a statement-level `foreach`
loop to a variable before piping it to `Format-Table` or another cmdlet; direct `foreach { ... } |
...` composition is rejected by the PowerShell parser.

Do not use PowerShell automatic or preference variable names for script-local state. In particular,
avoid `$Host`, `$PID`, `$Error`, `$HOME` and case-only variants; use task-specific descriptive names.

On this workstation, every Rust verification and Release-package wrapper (including a background
PowerShell process) must use `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`, and
`RUSTFLAGS=-C codegen-units=256`. The checked-in Windows wrappers initialize these values themselves;
direct Cargo commands must still inherit them. Release packaging must pass the committed-memory
headroom preflight before starting Cargo. Setting the variables only for an earlier command is not
enough; default package concurrency can exhaust Windows commit memory and make unrelated `rustc`
targets exit with `0xc0000409`. When the user declines packaging or building because of current
memory pressure, do not start Cargo verification either unless the user separately authorizes it;
use source/static gates and report the deferred compiled verification explicitly.

Give long Cargo verification and packaging commands a timeout sized for the full operation. Use the
tool's resumable yield/poll mechanism for progress instead of a short process timeout; after any
wrapper timeout, inspect surviving Cargo/rustc processes and never start a duplicate build.

## Indexer release gate

`Ready` and successful search are not sufficient evidence for an indexer release. After changing
MFT, USN, service-session or cache code, run `tests/performance/indexer-idle-cpu.ps1` against the
packaged LocalSystem service with `-RestartService -AssertRegression`, then run the real service
search smoke test. A service restart must restore a versioned memory-mapped base plus its persisted
delta; it must not rebuild all volumes merely because the desktop reconnects. State explicitly
which reference source was inspected, and never imply that Listary's native engine source was
available when only its managed boundary could be observed.

A bounded `RebuildRequired` decision while a last-good snapshot remains searchable is maintenance,
not a user-facing failure. Keep its reason in the maintenance event/log and reserve `last_error`
for actual poll, compaction, persistence, or snapshot-worker failures.

Before overwriting the default Release package, stop only a running desktop whose executable path
exactly matches `artifacts\win-x64\NxTools.exe`, and let the package script stop/restart only the
installed indexer service whose registered binary matches that same package. Never terminate an
unrelated process merely because its process name is similar.

The protected desktop, PlatformHost, and indexer form one atomic release. Startup may redirect to
the protected copy only when all three protected binaries match their packaged counterparts, and a
service smoke test must reject a protected indexer whose approved/runtime hash differs from the
current package.
