# R1 bounded per-volume publication

The service coordinator retains a single serial snapshot worker. Each volume's worker result reaches the coordinator as soon as its scanner returns; the production scanner persists and exactly reopens the mapped base before returning (`rust/apps/nxtools-indexer/src/mft.rs:585`, `mft/cache.rs:497`). The coordinator applies each result to the existing immutable search snapshot set and emits a status update without setting global Ready for an initial build still processing later volumes. Per-drive InitialBuilding/Ready and last-good failures remain distinct. Existing USN journal identity, pre-enumeration cursor, catch-up, durable base/delta restore and enabled missing-only scheduling are unchanged.

Per-volume maintenance correlation IDs span worker scan/stage logs and coordinator publication. Bounded `indexer.snapshot.native_enumeration` records enumerated rows, metadata-read rows, total native enumeration and metadata hydration durations; `indexer.snapshot.enumeration`, `construction`, `scan`, `persist`, `reopen`, `volume_scanned`, `volume_published`, `first_search_ready` and `all_volumes_complete` record rows/counts and stage durations at their owning boundaries. No item names, paths or queries are added to these events.

## Native baseline and remaining work

`mft.rs:774` still calls `hydrate_record_times_while` once per enumerated record; `mft/time_metadata.rs:28` obtains true LastWriteTime through OpenFileById/GetFileInformationByHandleEx/CloseHandle. This first-build per-record native open/read/close cost is not removed by per-volume publication. A separately authorized isolated first-build baseline must capture actual native stage timings and counts before selecting the raw-MFT standard-information parser change; no real-volume baseline or native comparison is available in this slice. Installed LocalSystem restart with base plus delta, real search and idle CPU/I/O remain separate authorized release gates. Listary source review reaches its managed session/status boundary only; SwiftList's inspectable per-volume raw-MFT/mapping chain informs lifecycle, not a Listary-native performance claim.

## Verification

Ten distinct focused synthetic tests passed: completed-first-volume publication while a later volume is blocked, failed initial build without a snapshot, slow-build handshake/ping liveness, six `mapped_publication_` cases, and pending maintenance after worker panic. The publication regression also verifies that global Ready is withheld until final worker completion, then becomes Ready. Scoped Rustfmt and diff checks passed.

All Cargo verification used one build job, disabled incremental compilation and `-C codegen-units=256`. These fixtures establish coordinator publication, durability admission and liveness; they are not measurements of a real NTFS first build or packaged LocalSystem acceptance.
