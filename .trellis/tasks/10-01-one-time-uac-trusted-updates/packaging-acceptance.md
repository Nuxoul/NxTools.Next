# Signed acceptance package authorization

User requested: "那就打包，我来亲自验收" after being told a persistent publisher signing key must be provisioned first. This authorizes key provisioning outside repository/package and generating a signed Release installer for user-run acceptance, not agent installation/service/UAC mutation.

Publisher key: current user's `.nxtools-release-keys/nxtools-next-release.pem`, RSA3072, protected owner-only ACL, never packaged/committed. Generated UpdateTrust stores SPKI only. Preserve this key and its release sequence ledger for future releases; do not regenerate on each build. Initial release sequence1.

Safe verification before packaging: C# Release223/223,0warnings/errors; Desktop678/678; fmt passes;7safe source gates and spec contracts pass. First Desktop run677/678 red exposed stale Light-button test after approved dropdown catalog change. Only test interaction was adapted to actual official DropdownMenu keyboard path (current Dark -> Up Light -> Enter), preserving all save/cancel/reopen/palette assertions; focused and integrated runs green.

Formal package wrapper uses -SkipVerification only to avoid its unfiltered real-volume workspace test command; explicit safe verification above was performed. Mandatory memory preflight, Rust Release build, generated Rust hash manifest, PlatformHost publish, PE/icon checks, signing and10isolated installer scenarios remain enabled. Required one-Cargo/jobs1/incremental0/codegen256/locked1.96PATH apply inside the background wrapper. Existing packaged-path processes alone may be stopped; protected installed service is not a target.

Package/signature/hash results and real installed acceptance remain to be appended after actual execution. No installer/product/native service operation has been run by the agent.

## Delivered package

Persistent RSA3072 publisher key created at the stated off-repository path with protected
owner-only ACL (one ACE); public trust generated, no private contents logged/packaged.
First background attempt exited before Cargo without status; sequence1 remains reserved.
Confirmed no surviving Cargo/rustc, then direct wrapper used sequence2 and exited0.
2026-10-14T10:36:29+08:00 installer17,043,110bytes,
SHA256 D3F4E0E65D07020C8138678FD40931B7C19F74A291F0DCBED8B29B9FFB34322A.
Release aad1bdaa-7c78-4c3d-aa58-c2531d807486,sequence2,38signedfiles.
Public SPKI SHA256 03e979d5c6babda29aa217b7ce72268aab12870bda46a9072397fe52ba083b69.
Independent signature/all-file/hash/embedded-key/3Rustapproval checks passed;5buildoutputs,
30extensionfiles,3licenses match.10installerfixtures pass.28.97GiBpreflight>=6.
Windows Authenticode remains NotSigned; release-manifest signature is valid and distinct.
Protected live Indexer still Running/PID5504; no installation/UAC/service mutation performed.
User acceptance instructions and precise current identity: artifacts/installer/INSTALL.md.
Next-version no-UAC acceptance requires a higher sequence signed by this SAME persistent key.
