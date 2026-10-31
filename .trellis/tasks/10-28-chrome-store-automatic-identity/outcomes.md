# Automatic published Chrome identity — outcomes (2026-10-28)

## Delivered
- Removed the Chrome extension-ID Input/entity and visible setting. Retained the original serialized optional field as non-editable draft compatibility data; Save/Cancel and registration retries keep original ownership.
- Default/missing/normalized-invalid legacy identity now sends the published Chrome identity. Native registration always unions the published exact origin with legacy command, pinned manifest and existing local signing identities, deduplicated, without wildcard or key creation.
- Added a direct official Small outline Chrome store Button in the existing Settings compound; both stores use the original pending command/result/disconnect/10s deadline and deferred callbacks. Chrome URL is constructed from the same Rust identity constant; a source gate checks Rust/C# agreement.
- Updated user README, reference note and extension contract. No changes to signing implementation, permissions, IPC schema, Core config representation or native registry transaction.

## Verification performed
- Before implementation: isolated C# allowed-origin-union fixture failed because the published identity was absent (1 run, 1 failure). Initial desktop red attempt was blocked by concurrent unrelated archive private-method errors; not represented as a behavioral red result.
- Final `cargo test -p nxtools-desktop --bin nxtools-desktop chrome_store -- --test-threads=1`: **6 passed, 0 failed**. Covers default/missing/invalid and retained identity reconnect, production settings search + actual Button click + deferred Shell URL, hidden-value draft/Cancel/Save, durable persistence failures and stale/deadline/disconnect ownership, both stores.
- `dotnet run --project csharp/NxTools.PlatformHost.Tests/NxTools.PlatformHost.Tests.csproj --no-restore -- "browser signing fixture"`: **7 run, 7 pass, 0 skip/fail**. Actual isolated CRX signature/key reuse, exact-origin union/dedup/locked/missing PEM, key-free store ZIP and Firefox/cancellation compatibility.
- Scoped rustfmt check for the three changed Rust files: PASS. `git diff --check` for task code/docs: PASS.
- `browser-signing-isolation.ps1`, `download-ui-lifecycle.ps1`, `spec-contracts.py` (114 documents/14 owners), `gpui-release-contracts.py`, `markdown-editor-contracts.py`: PASS.
- New published identity/default/URL/native union/no-ID UI portion of `browser-native-messaging.ps1`: independently executed, PASS.
- All Rust commands inherited CARGO_BUILD_JOBS=1, CARGO_INCREMENTAL=0 and RUSTFLAGS=-C codegen-units=256.

## Remaining unrelated checks / limits
- Strict desktop Clippy with tests fails in files not edited by this task: unused SPACING_12 import in main_window.rs, unread DecodedMedia.animated in tools/floating_assistant/media.rs, large Delivery enum variant in archive/readmission.rs. No suppressions or unrelated fixes made.
- Full browser-native-messaging source gate fails at the existing browser-confirmation cold-start focus assertion before reaching new checks. Executing the captured pre-task gate against the same current source reproduces that exact failure; no gate was weakened.
- No full workspace/full desktop suite, physical/DPI/visual/user acceptance or real installed Chrome Native Messaging/registry test claimed. No product/browser/service launch, package/install/release or real registry operation performed. Unrelated dirty changes preserved; no commit.
