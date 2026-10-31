# HatchZip five-capability research handoff

2026-10-27 user requested detailed source review and durable documents for another conversation to implement later. **This deliverable is documentation only.** All five subjects, including destructive in-place reclamation, are included; no feature is implemented or accepted by this research. No source archives, services, installed product or protected state were mutated.

## Entry point and deliverables

Start with [HatchZip reference index](../../../docs/reference-code/hatchzip.md), then:

| Work topic | Source-backed document | Intended future task |
| --- | --- | --- |
| HZ1 | [Space preflight](../../../docs/reference-code/hatchzip/space-preflight.md) | Extend original native capacity observation and Rust plan/GPUI preview; distinguish estimates, reserve and bomb limits |
| HZ2 | [Progress](../../../docs/reference-code/hatchzip/progress.md) | Compact actual stage UI, readable units/time, smoothed rate/credible ETA, truthful terminal summary |
| HZ3 | [Batch summary](../../../docs/reference-code/hatchzip/batch-summary.md) | Batch membership and exact per-job summary around original concurrent queue, not a second scheduler |
| HZ4 | [Crash/recovery tests](../../../docs/reference-code/hatchzip/crash-recovery-tests.md) | Actual owned child crash/restart tests at real durable boundaries; separate abort/IPC-loss/power-loss evidence |
| HZ5 | [In-place source reclamation](../../../docs/reference-code/hatchzip/in-place-reclamation.md) | Separate destructive mode/capability/journal/format/safety design, smallest isolated proof before production expansion |
| Provenance | [Source manifest](../../../docs/reference-code/hatchzip/source-manifest.json) | SHA256/bytes/line counts and verified anchors; external snapshot has noGitmetadata |

## Required start for the next conversation

1. Read AGENTS.md, latest [single checkpoint](phase-a-checkpoint.md), original PRD/design/implement and applicable specs. Do not infer current source from an old report; other conversations are changing UI, icons, tabs, persistence and installer. Preserve unrelated work.
2. Read corresponding HZnote before searching. Match manifest hashes against `D:\Code\hatchzip-main`; if changed, re-inspect affected entries and update provenance. Version0.1.3 is not a commit.
3. Separate source-established facts from future product decisions (reserve default, batch error policy, speed semantics, persistence, HZ5first format/consent/restore limits). Resolve these before implementation, without asking repository-answerable questions.
4. State minimal owner/file boundary and add red-capable actual production-seam regression first. Keep original Rust/C#/GPUI ownership, IPC/lease/generation/cancellation/finite readmission and output identity contracts.
5. Implement HZ1/HZ2/HZ3 through original owners. Introduce HZ4durable tests with each feature, not as a fake promise of existing resume. HZ5 requires its own approved design and disposable fixture proof; do not weaken ordinary source leases or repurpose post-success recycle.
6. Preserve canonical A–F plan order; HZ1–HZ5 are reference IDs, not newly numbered product phases. No silent phase reorder or acceptance reset.

## Must not copy blindly

- Slint framework, single-threaded Runner/serial queue, reference settings defaults and corrupted-config-overwrite behavior.
- Path-only cleanup of uncommitted output, first-volume-only lock/reopens, sidecar paths/CRC as authority.
- Windows directory-sync no-ops, same-volume prefix comparison or successful punch BOOL as durability/space proof.
- Source-destructive default (`keep_archive=false` upstream), current DeleteSource grants, or 'restore' wording suggesting full original recovery.
- Decimal MB formatting mislabeled MiB, known-total max(1) for unknown output, processed bytes labeled committed/reclaimed.

## Future acceptance boundaries

Each topic has named future HZx-* checks. Core invariants: normal extraction never modifies source; original cleanup checks preserve foreign/replaced files; output becomes durable before any source bytes are surrendered; resume uses original identity and immutable mode; unknown state refuses new mutation and retains evidence. Actual lost output/antivirus/quota/volume changes must be tested, not explained away.

Source-space reclaim needs bounded journal/tail/decoder state and approved native original-object interval mutation. Source content changes intentionally, so source generation/fingerprints and browser/preview/read/drag/modify exclusion must be explicitly designed. CRC is not journal authentication. TAR has no data checksum. Current source-recycle native backend remains unproved/default-off under its original contract; this research does not enable it.

Compiled/unit/source/actualfilesystem/physicalGUI/user acceptance are distinct. No local HatchZip runtime, test, CI run or benchmark was performed. Process abort recovery is not power-loss verification. Destructive real-archive operation, packaging/install/service/Registry/COM/Explorer/commit/push remain separately authorized.

## Verification and reproducibility

Research checks: source-file hash and exact symbol-anchor validation; documentation relative-link validation; `python tests/architecture/spec-contracts.py`; scoped `git diff --check`. Machine-readable final result is `docs/reference-code/hatchzip/research-verification.json`. These validate the handoff, not product behavior.

No Cargo is required for this docs-only round. Future direct/background Rust commands must inherit CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0, RUSTFLAGS=-C codegen-units=256; inspect/wait for original shared lane, no duplicate builds or unrelated process termination. Rust verification and native test scopes follow project restrictions.
