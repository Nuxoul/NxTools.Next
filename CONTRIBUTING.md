# Contributing

Keep the Rust/C# boundary narrow. A change belongs in C# only when it directly adapts a
Windows user-session API. Domain decisions, persistence and UI state belong in Rust.

Before submitting changes, run:

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
dotnet build .\csharp\NxTools.PlatformHost\NxTools.PlatformHost.csproj
```

Do not copy source from GPL reference projects. Behavioral compatibility must be implemented
from independently written tests and documentation.
