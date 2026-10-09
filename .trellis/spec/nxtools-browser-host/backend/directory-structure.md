# Browser host structure

Evidence: `repo:rust/apps/nxtools-browser-host/src/main.rs`, `repo:rust/apps/nxtools-browser-host/src/lib.rs`, `repo:rust/apps/nxtools-browser-host/src/blob_upload.rs`.

main frames stdin/stdout and initialization; lib validates browser protocol2/max256KiB, bounded request fields then converts typed Download commands, readiness/interactive/background start policy and response identity; Blob module owns loopback capability/chunk staging only. Engine owns persistent tasks, C# owns registry installation, extension owns browser item until confirmed acceptance. BrowserRequest payload Value is decoded/validated at this boundary, not passed raw into multiple handlers. [Architecture](../../shared/architecture.md).
