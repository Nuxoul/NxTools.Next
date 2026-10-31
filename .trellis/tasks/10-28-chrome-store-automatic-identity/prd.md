# Published Chrome extension automatic identity

## Approved request
Remove the extension-ID input from normal download settings after publication. Published Chrome Web Store ID: `eldfplghejhopcmgfkacimanhaojakph`.

## Boundary
- Preserve the existing Settings/Page/Group/Item composition, Save/Cancel, deferred cross-entity callbacks and native registration lifetime.
- Replace the removed ID row with a Chrome store action using the existing official Small outline Button, direct composition (no Toolbar.child adaptation), and original browser-store pending/result/timeout/disconnect owner.
- Empty/missing/malformed legacy config uses the published identity; retain the old serialized field and valid custom values without exposing an input.
- C# registration always includes the exact published origin beside command, pinned source and existing exported-key identities. No wildcard, new protocol or signing change.
- Existing correlated registration/store events and terminal failure guidance remain; no IDs, URLs or PEM bytes in diagnostics.

## Reference and invariants
Read download.md upload/identity note, then ABDM BrowserMessageRequest.kt (request identity only), then legacy BrowserExtensionPackageService.cs CreateCrx3Bytes/LoadOrCreateSigningKey/ResolveSigningKeyPath. Local per-user key reuse and fixed manifest identity are preserved. Registration remains read-only for PEM. Settings window lifecycle and native show/hide are unchanged. No browser/service/registry/product launch, packaging or release acceptance in this task.

## Acceptance
- Production reconnect command uses the published ID without manual configuration, including missing old field and normalized malformed field.
- Old valid custom identity and pinned/local CRX remain allowed along with published origin, deduplicated.
- Actual settings render has no ID input; Cancel does not write; Save preserves hidden legacy value and unrelated sections.
- Chrome action dispatches exact published URL through original correlated Shell owner; Firefox remains functional.
- Run isolated C# signing fixtures, desktop chrome_store regressions, scoped formatting/lint and browser/UI/spec gates where feasible. Report compiled vs static vs unexecuted live acceptance separately.
