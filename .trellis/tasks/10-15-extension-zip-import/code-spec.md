# Executable ZIP import and confirmation contracts

## 1. Scope / Trigger
Ordinary Extensions import now selects one ZIP and opens a Kit confirmation, replacing loose JSON/directory UI. Lower-level legacy import APIs and installed registry remain compatible.

## 2. Signatures
`ImportPreview::from_archive(&Path)->Result<Self,PackageError>`; existing `stage_into(&Path)` and Core `import_extension_preview` retained. Ordinary UI uses `AppConfig::import_extension_preview_checked(path,preview,packages_root,source,expected_previous:Option<String>)`, comparing fresh disk under original transaction lock before stage/install. Native wire name `pick_extension_manifest` retained, C# picker ZIP filter. Desktop `ImportCandidateIdentity=(generation,id,content_hash,source)` and `confirm_import_current`/`cancel_import_current`/`retire_import_preview`; explicit `import_failure`, pending validation generation and original busy owner.

## 3. Contracts
Rust owns archive/registry/generation; C# only native picker; GPUI rendering uses released0.7.1 Dialog/DescriptionList/GroupBox/Accordion/DialogFooter/Action/Close under original Root. No reentrant cross-entity updates: candidate local, defer original app transaction. Install only exact current candidate/expected installed identity, no stale overwrite or duplicate confirm. All terminal import failures reset submission and render local error; accepted commit cannot be cancelled by merely hiding UI. Leaving module retires validation generation; worker retains busy admission until actual completion and discards late candidate.

ZIP root or unique one-folder wrapper; classic single disk Stored/Deflate, input256MiB, manifest1MiB, remaining actual decompressed256MiB incl ignored extras,4096files incl manifest/4096dirs incl implicit/8192explicit entries, names1024UTF8bytes/64levels. Raw directory preflight before ZIP map allocation; duplicate/case/reserved/ADS/absolute/traversal/link/special/encrypted/unsupported/corrupt/CRC/size-forgery rejects. Read all entries under budget, retain declared files only, no extraction on preview. Normalized manifest and files sync then publish immutable content-addressed snapshot; config lock/reread/merge/rename remains original. Failed private stages cleaned; published snapshot not blindly deleted because another importer may share it.

Risk warning remains visible outside scroll above32px footer. Only technical ID/hash/sourcepath/entry/budgets collapse. KitDefault themes/fonts/window budgets unchanged. Ordinary title/footer/close/focus semantic owners remain Kit; narrow natural-width wrappers prevent DialogAction/Close size_full from stretching buttons, no recreated controls.

## 4. Validation & Error Matrix
Bad ZIP -> fixed ArchiveInvalid/ArchiveUnsafe/ArchiveTooLarge/local retry, no live config write. Cancel/Escape/close -> clear only matching candidate, zero installation. Stale candidate/new disk version -> reject before write. Failed config -> keep dialog/draft/retry; no auto trust/enable. Success -> close, list installed disabled package, source ZIP removable. Correlated validation accepted/completed/failed/cancelled and management terminal logs carry bounded IDs/elapsed/fixed reasons, never paths/archive content.

## 5. Good / Base / Bad
Good: ZIP -> validate -> confirm -> managed directory -> delete original ZIP -> still valid. Base: one wrapper or declarative manifest. Bad: shell unzip into live directory, preview executes code, copy external source path only, overwrite newer package with stale confirmation, failure latch depends on Chinese message prefix, risk warning scrolls offscreen.

## 6. Required Tests
Archive7 actual ZIP fixture cases; Core actual ZIP confirm/reopen/source deletion/configfailure/old package and fresh-disk checked replacement; GPUI actual pointer/Enter/Escape/duplicate confirm/failure retry/stale generation/competing install/retired validator/minimum footer; C# ZIP currentpath/filter/cancel/failure/detached reader. FullDesktop/scopedstrict/workspace/fmt/lifecycle/spec, actual isolated Light/Dark default/min/replace/long screenshots. Evidence levels separate; no installed/physical/mixedDPI acceptance implied.

## 7. Wrong vs Correct
Wrong: trust only cached previous hash, `status.starts_with(...)` controls admission, early release while validator running. Correct: fresh locked expected_previous compare; typed import_failure resets every failed import; retire logical generation but retain worker admission. Source tests and one bounded review findings repaired; see outcomes.md for exact logs and remaining native/release limits.
