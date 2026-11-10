# Archive worker source and relinking

`archive-worker-source.zip` contains the current NxTools archive worker application
source, the two NxTools crates it uses, the corresponding Cargo lock, and complete
published dependency sources and licenses. The complete release distribution ZIP
contains the normal installer and these materials under
`third_party/archive-worker/`. Keep them together when redistributing the release.
## Library licenses

WIM reading uses unmodified `wim-format 0.1.1`, `wim-types 0.1.1` and
`ms-compress 0.2.1`. The two WIM crates are used under LGPL-2.1-or-later;
ms-compress retains its original combined license notices, including LGPL-2.1.
Their complete notices and source files are in `vendor/` inside the source archive;
license copies are also under the adjacent `licenses/` directory.

- https://github.com/CaddyGlow/wim-rs
- https://github.com/CaddyGlow/ms-compress

NxTools application source retains its MIT OR Apache-2.0 licensing. No additional
restriction is imposed on modification for your own use or reverse engineering
for debugging modifications to the LGPL-covered libraries. Library authors and
contributors retain the copyright notices in their source and license files.

## Build the supplied worker

Use Windows x64, Rust 1.96.0 for MSVC, Visual Studio C++ Build Tools and a Windows
SDK. Python is needed only if updating Cargo's per-file metadata after modifying a
vendored library. Extract the archive to a short writable path such as `C:\nxsrc`.
The source archive is a standalone workspace; no desktop source or release private
key is needed. From that directory, in PowerShell:

```powershell
$env:CARGO_BUILD_JOBS = '1'
$env:CARGO_INCREMENTAL = '0'
$env:RUSTFLAGS = '-C codegen-units=256'
$env:CARGO_PROFILE_RELEASE_DEBUG = '0'
$env:CARGO_PROFILE_RELEASE_LTO = 'off'
cargo build --release --offline --locked -p nxtools-archive --bin nxtools-archive-worker --features isolated-codecs
```

The executable is `target\release\nxtools-archive-worker.exe`. All dependency
sources are local, selected through `.cargo/config.toml`; the Rust compiler and
Microsoft system toolchain are external prerequisites. `DEPENDENCIES.json` lists
bundled package names, versions and upstream license declarations. The workspace
omits unrelated desktop packages but preserves current worker source and locked
dependency versions. This kit supports rebuilding the worker binary, not the full
product's tests or signed release pipeline.

## Modify an LGPL library and relink

Edit the corresponding `vendor\wim-format-0.1.1`, `vendor\wim-types-0.1.1` or
`vendor\ms-compress-0.2.1` source. Before rebuilding, update Cargo's metadata for
the library you changed so its original per-file records do not reject your edits:

```powershell
python -c "import json,pathlib; p=pathlib.Path('vendor/wim-format-0.1.1/.cargo-checksum.json'); d=json.loads(p.read_text()); d['files']={}; p.write_text(json.dumps(d))"
```

Use the same command with the relevant library directory for other changes.
Then run the offline build above to relink the application with your modified
library. No published source or signing key is contacted by this build.

The production Desktop verifies its worker's release identity. A rebuilt worker is
not interchangeable with the signed installed worker: replacing a file inside the
signed package invalidates its existing approval. The rebuilt executable remains
usable directly through its stdin/stdout worker interface; host integration uses
the application's development or owner-signed build workflow. Public protocol and
host source are in the NxTools source repository. This kit does not grant the
original publisher's signing authority or modify the production trust boundary.
