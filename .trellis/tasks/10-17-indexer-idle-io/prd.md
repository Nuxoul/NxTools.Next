# Indexer sustained idle write amplification

## Request and scope
User reports NxTools.Indexer background I/O sustained around10MB/s while Listary settles to2–5KB/s after startup. Diagnose and fix the concrete source regression; preserve unrelated official-notification/UI/extensions/download work. No package/install/deployment/service restart/rebuild or unrestricted disk scan authorized. Read-only installed service observation is performed; final installed verification remains separate.

## Measured red baseline
Running LocalSystem NxToolsIndexerV1 PID5288 matches packaged sequence15 SHA A024538504BBCE5957BEE0666C4A3D34215F093F75D104547D31FB5832F52379. Read-only15sample/16.79second counter loop:0reads,2,499,201write bytes/s average,15/15above1MiB/s; spotcounter8–9MB/s. Process logical I/O includes files/IPC, not direct physical disk attribution. Cache metadata shows C-drive2,378,166-byte delta replaced almost everysecond. Read-only delta diff categorizes2self-cache rows plus5agent-scratch rows over4seconds; no user paths/content output. Listary.exe absent, only Listary.Service available; no equivalent live main-process comparison claimed.

## Acceptance
- Real temp-file regression: after>=2MiB durable overlay, three one-row commits write<32KiB to the test process, not multiple complete overlays.
- Current searches and cursor/metadata/rename/delete behavior remain; durable restart restores latest complete committed delta; failure visible/last-good searchable.
- Existing version3base/delta readable; no full volume rebuild just for format migration or desktop reconnect. Crash/torn tail cannot advance cursor without corresponding delta.
- Bounded growth/checkpoint, partial write/failure retry, base identity/journal validation and no-change/cursor-only behavior tested at actual boundary.
- No special exclusion of user files to fake idle; no timeout-only throttling as sole fix. No native service authorization changes or new dependency.
- Focused synthetic/cache/admission regressions, strict/workspace/fmt/source gate; real installed post-fix idle/restart/base+delta/search/hash gate only after separately authorized package/deployment. Source green is not current installed service green.
