# Built-in extensions correction and sequence11

## Root cause / actual behavior
The user expected built-in extensions on a new computer. The previous delivery generated3 standaloneZIPs only, omitted production resources and registration, and tested archive import rather than installation initialization. Sequence10 payload inspection found0 matching extension files despite all3 local ZIPs existing. Real AppConfig::load_or_create regression then failed with0 entries versus3 expected. This is the agent's integration/acceptance omission, not a user mistake or a missing Python explanation.

## Delivered
- Production nxtools-extensions::bundled_previews embeds the exact existing manifest/main.py/protocol.py/test_main.py/README.md for3 packages in the signed executable. No runtime checkout/ZIP/network dependency. Script and per-package README bytes are unchanged; independent ZIPs remain valid and unchanged.
- Core load_or_create now holds the original sibling configuration lock through existence recheck, load/default, normal immutable staging, registry/one-time marker commit. Startup services and existing Extensions catalog consume that returned committed registry, with no second store/controller.
- Fresh users and prior uninitialized configs receive all missing bundles. Same-ID user packages always win with their settings/content/trust/enablement retained. Once initialized, repeated startup does not write config or resurrect removals. Ordinary default/load_from do not seed or write.
- New bundles are disabled/untrusted, with no selected runtime, events/providers or proxy opt-in. No automatic Python install, trust, enable, process, network or clipboard effect. Whole-user script approval remains explicit and not a sandbox.
- Correlated accepted/staged/terminal diagnostics carry counts/elapsed only, no paths/code/payloads. New source/packaged-binary gate is wired into package.ps1 to reject missing production resources. No UI/menu/native/engine changes; previous lower-slot feedback remains visually unaccepted.

Reference read: docs/reference-code/winshun.md then read-only Catalog.cpp14–29/35–75 (user folder first/same-ID priority, enable independent). No reference binary/plugins were run; Next uses its immutable store instead of copying Qt discovery/watchers.

## Red/green and verification
- bundled-first-run-red.log: actual first-run production config test failed0-vs3, then green.
- bundled-package-red.log: actual sequence10 Desktop rejected for missing github-public-proxy/extension.json. Final package gate passes all15 exact embedded resource byte sequences for3 packages.
- Core56/56 including6 startup regressions: fresh untrusted defaults and no-op repeat; legacy/same-ID/other-section preservation; removal not restored; corrupt/stage failure retains bytes and retry; concurrent bootstrap; explicit existing settings/runtime/trust/enablement retained. Temporary dirs only; runtime identity fixture inspects the owned test binary without executing it.
- Extensions48/48 includes2 real directory/ZIP/embedded preview/stage tests; exact manifests/code/content hashes compared with source. Desktop759/759 (95.59s), BrowserHost31/31;0failed/ignored.
- Strict Clippy for Core/Extensions/Desktop --tests -D warnings; workspace --tests; fmt pass. Serialized Rust1.96/jobs1/incremental0/codegen256 throughout.
-7 source gates plus specs94documents13owners/catalog79pages78mappings pass. Source gates are not native/visual acceptance. Existing unused Desktop extension_adapter warning retained, not suppressed.

## Corrected Release artifact
User's failed-install report continues the requested acceptance package; corrected sequence11 built without automatic installation/deployment/service mutation.

- artifacts/installer/NxTools.Next-setup-0.7.0.exe; version0.7.0, sequence11.
- Started2026-10-16T11:14:06.8169229+08:00, finished11:19:23.0573825; installer timestamp11:19:23.0518505+08; exit0.
-18,421,932bytes (~17.57MiB); SHA25635E54F5104B7B1ABB693AB2457547725F410D19A7AA9A74755108824400FE219.
- ReleaseIDf3b434b4-ccd5-444f-a7d4-581d57713636; manifestSHAEF08C966E1A29FAC0B975815F8D0FDD7D9213CBEB11EE16B4CC5F27EEB25C0B8.
- Same publisher publicSPKI03e979d5c6babda29aa217b7ce72268aab12870bda46a9072397fe52ba083b69. Actual ledger10 before,11 after; isolated fixture reservations are not production reuse.
- Complete sequence10 package/installer/evidence backed up under scratch release-sequence10-before-bundled-fix-backup, installer hashA14B7060C4423A4805B325DF9D0B0D35332CE1C2766A78B95514EF5C863E970B verified.
- Release preflight25.39GiB and actual pre-Cargo25.14GiB >=6. One build, no survivors. package.ps1 Release/SkipVerification uses safe owner checks instead of unrestricted real-volume wrapper; build/publish/PE/icon/new embedded-resource gate/sign/Inno and10isolated installer/5sequence/2rollback assertions all pass.
- Independent RSA3072-PSS/SHA256 verifies38 file sizes/hashes, embedded key and3 approved Rust identities.5 programs match build outputs;30 browser files/3licenses match; every packaged file scanned for private keys/PEM markers, none. Production graph excludes test-support. Independent Python verifier reconfirms all15 embedded resources and3 unchanged ZIPs.
- All observed protected processes/services unchanged: Desktop16460, IndexerRunning5784, UpdaterRunning5492 plus other observed host processes. No exact-package process stopped. Authenticode NotSigned; manifest signing is not certificate signing.

Logs: bundled-{first-run-red,first-run-green,core,extensions,strict,desktop,browser-host,workspace,fmt,gates,package-red}.log and release-seq11-{preflight.json,package.log,status.json,verified.json,artifact.json,content.json,runtime-observed.json,production-features.log}. Final installer evidence/checksum/docs consistency and scoped UTF8/JSON/diff/context gates checked at handoff.

## Remaining limits
No actual installation on this or another computer, normal product launch, user configuration/Clipboard/credential/plugin execution, deployment, service restart, commit/push/archive. Isolated startup and binary-resource checks directly protect this omission but do not certify actual foreign-machine installation, visual/physical/mixed-DPI/reader or packaged LocalSystem restart/CPU/search. User acceptance remains pending; W10 remains deferred.
