# Download IPC quality

Evidence: `repo:rust/crates/nxtools-download-ipc/src/lib.rs#tests`, `repo:rust/crates/nxtools-download-ipc/src/lib.rs#DownloadHostReadiness`.

Exercise actual temporary pipe/typed frames, incompatible Hello version, matching command identity, event/snapshot sequence and host draining/readiness. A successful pipe connect isn't Ready, nor does an Accepted command mean applied/finished. Caller tests preserve CLI ConnectOnly and browser acceptance identity. Existing readiness fields have serde defaults for compatibility. Run scoped library/lint/check/fmt per [verification](../../shared/verification.md), synthetic data only.

The destination-conflict contract uses download command protocol9 and IPC handshake2. The pipe name remains stable so an old Hello receives explicit ProtocolIncompatible before commands, rather than appearing host-unavailable. New DestinationExists/RetryWithAutoRename enum values require matched clients; CLI and BrowserHost consume shared constants. Test prior-version rejection and current Hello roundtrip. No C# PlatformHost DTO or SQL schema migration is implied. Owning executable behavior: [download recovery](../../nxtools-download/backend/quality-guidelines.md#scenario-nonblocking-events-and-no-replace-destination-recovery).
