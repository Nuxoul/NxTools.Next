# Archive worker source distribution — 2026-11-06

## Delivered materials

The isolated worker retains WIM reading through wim-format0.1.1, wim-types0.1.1
and ms-compress0.2.1. The normal package now includes their complete published
license notices, corresponding library sources, current archive/IPC/Protocol
application sources, the locked dependency closure and offline rebuild/relink
instructions. Application source remains MIT OR Apache-2.0; library license
choices and modification rights are stated in `THIRD_PARTY_NOTICES.md` and
`third_party/archive-worker/REBUILD.md`.

`scripts/package-archive-source.py` creates a standalone three-crate workspace
from the current source and product lock, retaining existing dependency versions.
Cargo's official vendor command supplies directory-source metadata. The delivered
configuration resolves offline. `scripts/package.ps1` generates these materials
before signing; failure stops signing rather than distributing an incomplete kit.
The complete release ZIP accompanies the normal installer with the source ZIP and
adjacent licenses. The original signed installation inventory stays unchanged.
No publisher PEM, private key or other session material is included.

## Verification

| Boundary | Result |
| --- | --- |
| Source-distribution regressions | 4 passed: standalone manifest/lints, content with old upstream timestamps, signing-order/materials contract, and complete installer/source distribution |
| Standalone application source comparison | 102 files match current archive/IPC/Protocol source byte for byte; three crate manifests match |
| Published dependency sources | 147 packages, all existing product-lock versions |
| LGPL license content | WIM and compression library license files match original published source |
| Private PEM entries | Zero |
| Offline directory-source resolution | Passed with `--offline --locked` |
| Extracted source ZIP Release rebuild | Exit0,315.15seconds; actual `nxtools-archive-worker.exe` produced from the delivered sources |

The rebuild used Rust1.96.0/MSVC, a fresh scratch target directory, jobs1,
incremental0, codegen-units256, Release debug0/LTOoff, `--offline --locked`, and
`--features isolated-codecs`. It compiled the library sources from the archive's
vendor directory rather than the machine registry. No source or binary was
substituted into the installed product and no archive member was executed.

Evidence: `D:\.pi-desktop\data\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\source-distribution-check/`
contains the source ZIP, content verification, actual compiler log and rebuild
result. This is source-distribution and build verification, not a new codec
behavior, installed service or full-product acceptance.

## Integration boundary

The production Desktop retains release-identity verification. A recipient can
modify the library and relink the supplied worker without the publisher's private
key. The standalone worker's stdin/stdout protocol is available in the supplied
sources; integration with a trusted host follows the owner's development or
owner-signed release workflow. The source kit does not authorize replacement of
protected binaries or change the publisher trust boundary.
