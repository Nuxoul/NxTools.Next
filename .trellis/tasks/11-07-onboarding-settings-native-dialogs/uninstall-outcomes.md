# R6 uninstall cleanup outcomes

## Final contract

The real Inno uninstall uses one correlation and runs protected-component removal before current-user cleanup. `NxTools.PlatformHost.exe --uninstall-protected-components <install-root> <correlation>` verifies the current signed package and caller, fixed service registrations/accounts/start modes/ACL grants, the complete protected release tuple, archive/helper approval, updater enrollment, active signed snapshot and absence of update/maintenance recovery state. It acquires the existing global deployment gate, revalidates registration and roots, stops only fixed owned services and exact protected executable paths, confirms service deletion, then removes only validated protected trees.

A tree-deletion failure after service deletion is retryable without a new journal. A later `Run` accepts only a registration-free `Service` or `Updater` remainder that still exactly validates against the current signed release, under the same package/caller and deployment-gate boundaries. Foreign files and update/maintenance recovery records refuse deletion.

An exact-owned `NxToolsUpdateV1` registration whose whole fixed `Updater` root is absent is removable. Authority is limited to the signed current package/caller plus the original fixed ImagePath, LocalSystem account, supported start mode, service ACL grant, publisher-key hash and signed bootstrap hash, rechecked under the deployment gate. A foreign tuple is retained. An Indexer registration with a missing Service root remains invalid.

Standard uninstall removes provably owned personal/runtime state; interactive **No** explicitly retains personal data and Cancel exits. Silent uninstall uses standard removal. Upgrade/repair never invoke either uninstall cleanup path.

## Conservative data preservation

A final download can survive after its task row and `download_artifacts` record are removed, and its configured default/category directory can later move or disappear. No remaining production fact can then attribute an arbitrary file to the downloader. Cleanup therefore never recursively removes a payload subtree merely because it is under ProjectDirs or an application-named custom subtree.

The cleanup removes these exact owners:

- `config.json`, its lock and matching atomic temp files;
- downloads, quick-launch and clipboard databases plus WAL/SHM;
- clipboard asset files still referenced by the clipboard database and located under that storage root's `clipboard-images` subtree, unless protected by a current committed/configured download exclusion;
- extension `catalog.json`, `catalog.tmp`, `initialized`, `quota.lock`, namespace locks and namespace/pending JSON files named by the valid catalog, with unattributed payloads retained;
- archive session `manifest.json` and retire records whose GUID path/name and recorded lease identity agree, plus fixed temp-root inventory records; session payload files and unknown entries remain;
- the current configured managed FFmpeg executable only when it is inside the fixed `tools\ffmpeg\managed-*` layout and is not a committed/configured download exclusion;
- desktop/indexer instance lock and hint files;
- daily desktop/download-host/browser-host/indexer logs, PlatformHost JSONL logs, startup-failure log and fixed installer diagnostic filenames;
- exact-owned Shell and native-messaging registrations under their existing compare-before-remove boundaries.

Unknown and unattributed files are retained in built-in `clipboard-images`, `extensions-data`, `browser-blob-staging`, `tools`, default archive cache/sessions, custom clipboard assets and recorded custom archive sessions. Custom parent directories are never deleted wholesale. If a clipboard database or extension catalog is unreadable, it cannot authorize payload deletion: exact database/catalog metadata is removed, while unattributed payloads remain.

This policy intentionally leaves some application runtime payloads after uninstall. Browser Blob staging has no durable per-file inventory after process shutdown, extracted managed-tool companions are not fully enumerated by configuration, and archive manifests do not authorize deleting unrelated session payloads. Preservation is preferred over deleting a supported download output. No hash ledger, schema, migration framework or new persistence owner was introduced.

## Source ownership inspected

- `.trellis/spec/platformhost/installer-preparation.md:45-97`: signed caller/package, exact registration, deployment gate, missing-root/recovery preservation and protected parent validation.
- `.trellis/spec/platformhost/native-contracts.md:83-123,467-510`: signed complete-release deployment, exact protected tuple and demand-service lifecycle.
- `docs/reference-code/archive-shell.md:260-289`: exact-owned updater registration with genuinely missing whole root and recovery constraints.
- `docs/reference-code/system-shell.md:333-341`: current Indexer/Updater service lifecycle and no trust/recovery reset.
- `rust/crates/nxtools-download/src/engine.rs:2505-2547`, `storage.rs:1794-1800` and `organization.rs:82-122`: remove-with/without-artifact, task-row cascade and category output resolution.
- `rust/crates/nxtools-clipboard/src/lib.rs:498-550,1076-1100`: clipboard asset inventory tables and referenced asset paths.
- `rust/crates/nxtools-extensions/src/storage.rs:899-949,1111-1176`: extension catalog, namespace file/lock naming and fixed metadata files.
- `rust/apps/nxtools-browser-host/src/blob_upload.rs:572-675`: transient UUID Blob staging naming without a durable uninstall inventory.
- `csharp/NxTools.PlatformHost/Windows/Archive/ArchiveEntryCache.cs:49-164,257-337` and `ArchiveEntryCache.TempRoots.cs:36-105`: session manifests, lease identity and recorded custom roots.
- Existing protected owners: `TrustedMissingBootstrapRepair`, `TrustedUpdateBroker`, `TrustedUpdateTransaction`, `TrustedUpdateEnrollment`, `TrustedDeploymentGate` and `IndexerServiceInstaller`.

## Final verification

- `python .\tests\architecture\installer-uninstall-cleanup.tests.py` — `uninstall cleanup fixtures passed: 9/9`.
  - Covers standard removal, explicit retention, upgrade/repair preservation, process-stop failure, removed history/configured-root overlap, moved/deleted categories, unknown ProjectDirs content, and downloaded sentinels in every formerly recursive built-in/custom payload subtree.
  - Proves exact DB-referenced clipboard assets, extension catalog-named files, archive manifests/inventory and current managed FFmpeg executable are removed while unattributed sentinels remain.
  - Session-manifest and retirement-record deletion failures propagate their native error code, preserve subsequent cleanup state and never emit successful completion.
- `python .\tests\architecture\protected-uninstall-entry.py` — `protected uninstall source entry: passed`.
- Parent C# verification — build `0 warnings / 0 errors`; registered protected uninstall fixtures `4/4 PASS`.
- Scoped `git diff --check` — passed.

## Acceptance limits

No product, UAC prompt, SCM/Registry mutation, real uninstall, current-user data read/delete, external process termination or packaging was performed. Source and fake-effect fixtures do not establish installed acceptance. Separately authorized disposable-install acceptance remains required for normal/retain paths, active background downloads, protected services/processes, retry after protected-tree failure, missing updater root, recovery refusal and preservation of arbitrary outputs in built-in/custom runtime subtrees.
