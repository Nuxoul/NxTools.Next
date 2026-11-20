# Implementation

1. Verify the indexed AppsFolder/native reference and existing Finder/Quick Launch/Shell boundaries. Add an actual Finder search regression for Calculator/Paint before changing implementation.
2. Add complete installed-application snapshot merge, aliases and original deferred refresh/IPC routing. Add C# enumeration and native launch-boundary fixtures before its changes.
3. Run Finder/Protocol tests, scoped desktop installed-app refresh/action regressions, reviewed exact C# fixtures, relevant strict lint, source/spec and scoped formatting gates. Record unresolved preexisting checks separately.

All Cargo commands are serialized with CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256. No packaging, installation, live Shell/Registry/service operations or commit. Physical native installed-app acceptance remains separate.
