# Phase C — Archive browser and entry lifecycle: closeout contract

2026-10-21. Current user authorizes fixing the independently reproduced retirement defect and
finishing window/column UI persistence, expiry/startup cleanup and browser password interactions.
The accepted ListItem/public virtual-list fallback and internal ArchiveReadClient dependency stay;
no spike repeat, D/E/F, package/install/commit or GUI safety bypass. The historical
[partial acceptance](phase-c-browser-acceptance.md) remains byte-preserved. Only
[phase-a-checkpoint.md](phase-a-checkpoint.md) owns current status.

## Cache owner and expiry rule

Keep `ArchiveEntryCache` as the one GUID-owned no-reparse native cache owner. Output deletion is
not full session retirement. Before moving the active manifest out of its root, persist an exact
versioned record of output removal, original root identity, validated output identity/hash/metadata/
MOTW and bounded directory ancestry. Move that held manifest to `<lease-id>.retire.json` in the
fixed sessions parent, using native no-replace rename; retain its exclusive lock until retirement
or disposal. This is narrow recovery authority, not an arbitrary path list. On failure, return
incomplete, retain/reopen that record, and require original root/directory identity on EVERY retry.
After restart a bare GUID directory without a valid matching record is never cleanup authority.
Only empty directories retire; unknown, edited and locked files remain. No recursive delete.

The design's proposed24h retention becomes explicit: a non-retired abandoned lease is eligible
at creation UTC +24h (clock supplied only to isolated fixture factory), not earlier. Active leases,
including child-browser ownership, retain an exclusive manifest lock and are never swept even
past expiry. Explicit browser retirement/release is immediate eligibility; it still cannot bypass
identity/content/metadata/MOTW or external-lock checks. Incomplete directory retirement after the
validated output has gone is immediately retryable because it no longer authorizes payload deletion.

Constructing the production cache through the existing per-pipe ArchiveEntrySession starts one
background bounded batch. At most8 filesystem entries and at most8 locally retired leases are
examined per batch; subsequent normal admissions continue the same enumerator. No timer,
unbounded directory scan, recursive cleanup, replacement worker or process-lifetime inference.
Correlated cache diagnostics contain counts/elapsed only, never names, paths or manifest payloads.
New root/manifest shape is version2; unsupported legacy/malformed state stays intact, never guessed.

## Parallel edit boundaries / verification

Parent owns cache + new cache fixtures and catalog registration. Window persistence task owns
new ui_state.rs and browser snapshot/apply interfaces only. Private password task owns application/
entry composition and original worker/IPC/native interaction seams; no public ArchiveRead password
or write permission. Parent integrates window-state hooks only after password task settles.
No concurrent writes to one path. Other workspace tasks own clipboard/history/preview changes.

Cache regression was added before implementation and observed red at full-retirement return.
Required green boundary includes later legal cleanup AND recreated cache/startup recovery,
unknown-root obstruction, root replacement refusal and no-record orphan preservation. Expiry tests
use small GUID-owned files, controlled UTC clock, actual native locks and real startup task.

Rust lane is shared: inspect machine Cargo/rustc before each command; jobs1/incremental0/
codegen-units256 required. C# fixture LIST and RUN use exact identical selection and report counts.
After final feature-unification checks rebuild matching isolated-codecs worker, then final native
fixtures. Each check log has a unique name; no continuation/overwrite wrapper. Final full Desktop
runs once after source convergence; unrelated failures are recorded without out-of-scope repairs.
Actual GUI/Shell/DPI/UIA/user acceptance remains NOT RUN under the existing safety denial.

## Verified closeout seams

Window presentation uses separate bounded path-free state, one nonblocking wake slot/background
writer, close/F4 snapshot and actual keyed reopen, not just helper round-trip. Password read opts in
through browser-only command naming on the original transport while ArchiveRead schema remains
password-free/read-only. Nonsecret ticket events and bounded redacted private secret replies reuse
authenticated platform pipe and original runner/InteractionPort; no second worker/read framework.
Read password retries retain original native operation control; attempt IDs are diagnostics only.
One bounded cache review was converged/fixed/retested; no recurring broad review. Final outcomes
and unmodified historical failures: [closeout results](phase-c-closeout-outcomes.md).
