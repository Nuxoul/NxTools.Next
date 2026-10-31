# Download IPC quality

Evidence: `repo:rust/crates/nxtools-download-ipc/src/lib.rs#tests`, `repo:rust/crates/nxtools-download-ipc/src/lib.rs#DownloadHostReadiness`.

Exercise actual temporary pipe/typed frames, incompatible Hello version, matching command identity, event/snapshot sequence and host draining/readiness. A successful pipe connect isn't Ready, nor does an Accepted command mean applied/finished. Caller tests preserve CLI ConnectOnly and browser acceptance identity. Existing readiness fields have serde defaults for compatibility. Run scoped library/lint/check/fmt per [verification](../../shared/verification.md), synthetic data only.
