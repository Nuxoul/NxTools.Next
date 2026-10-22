# Isolated CRX signing tests and shared identity (2026-10-13)

## Outcome and scope
CRX/XPI test now explicitly uses a GUID-owned temporary signing PEM; no test calls the production-key Export entry. Production public Export still resolves the same LocalAppData/NxTools/browser_extension_key.pem; registration only reads an existing identity. Existing dirty work preserved. No production PEM read/create/update, actual registry/native registration/browser/install/Release/package wrapper/commit performed. Synthetic test CRX/XPI artifacts are temporary fixtures, not product packaging.

Read whole-project report, PlatformHost/extension/shared specs and download reference first. ABDM Desktop NativeMessaging BrowserMessageRequest.kt inspected first for message identity only; legacy NxTools Services/BrowserExtensionPackageService.cs CRX3/SPKI/RSA/key path inspected for product compatibility. Current call chains and preserved fallback/error rules recorded in docs/reference-code/download.md. No UI/layout/focus/native-window behavior changed.

## Red evidence
Pure reflection fixture failed before change: isolated ExportWithSigningKey with explicit path absent (1RUN/1FAIL,exit1; no exporter/PEM invocation). Source guard run against saved dirty pre-task package/registrar source also failed exit1; current source passed. Logs browser-signing-red.log and browser-signing-source-red-green.log. No unsafe original CRX test was executed before isolation.

## Responsibility and preservation
- BrowserExtensionSigningIdentity is a small internal concrete owner of legacy relative key path, SHA256first16SPKI bytes, a..p Chromium mapping and read-existing PEM identity. No key creation or writes, no generic identity/storage framework/dependency.
- BrowserExtensionPackageService retains sole LoadOrCreateSigningKey responsibility. Production public/internal normal Export uses shared production path; internal ExportWithSigningKey carries explicit path through CreateCrx3Bytes and loader. Tests never evaluate ProductionKeyPath. Existing RSA2048/PKCS1/SHA256/PKCS8, reuse, failure and package publication behavior retained. Removed unused SigningKeyGate only; no concurrency behavior changed.
- Registrar ChromiumAllowedOrigins production entry passes shared production path; parent-private path/source overload tests real origin projection without Ensure/registration. Command first, pinned manifest next, exported-key last; union/dedup/order retained. Missing key no origin/no create; IO/Format/InvalidOperation fallback remains, malformed ASN.1/crypto or JSON errors retain original non-swallowed failure categories. Existing keys are never silently regenerated. Public-only PEM remains readable identity.
- Export source resolver retains sibling,parent,current-working-directory candidates; registrar retains sibling,parent only. Parameterized internal resolver seams test those original policies separately; no conflation of fallbacks.
- Existing CRX/XPI test name and assertions preserved with explicit work/fixture-signing.pem path, source/output cleanup including exception paths. Moved from RestrictedFixture to reviewed Fixture; seven new pure/temp-IO signing fixtures registered explicitly in TestEffects. Current224entries197Fixture/23NativeAcceptance/4RestrictedFixture; listing RUN0. Other icon-cache/fixed-temp-path restricted tests remain unmodified.
- Snapshot comparison verified11key methods unchanged: registrar EnsureTransaction/Ensure/ValidateExtensionId/WriteRegistrationLog; exporter CreateFirefoxManifest/BuildCrx3SignatureInput/EncodeMessage/WriteVarint/WriteUInt32LittleEndian/CreateZipFile/MovePackageToDestination. Thus native integrity/transaction, Firefox identity, signed-data envelope and encoding untouched.

## Regression coverage and actual final checks
Seven new tests plus original isolated package fixture cover:
1. Reflection explicit-path seam and source guard forbidding production entry/path in test code.
2. Fixed checked-in public SPKI vector -> haohhnpjfggpdadhcbjakjpkmkljkpif, digest16=70E77DF9566F30372190A9FACAB9AF85; expected independently computed, not implementation self-comparison.
3. Missing PEM lookup creates neither file nor directory; existing private/public PEM read identity and bytes/time unchanged.
4. Actual locked-temp-PEM IOException: registrar preserves command origin, exporter fails, bytes unchanged.
5. Command/pinned/exported origin union/order/dedup, missing manifest, badBase64 fallback, malformedJSON failure, absent PEM no creation.
6. Real temporary CRX3 parsed length-delimited header/proof/signed data:embeddedSPKI matches isolated PEM, ID matches SPKI, RSA signature verifies; registration origin matches actual export; second export reuses PEM unchanged.
7. Corrupt ASN.1 PEM registrar/export failure remains non-swallowed; bytes never replaced; Firefox/precancelled export creates no PEM directory.
8. Distinct source fallback ordering and existing Firefox XPI ID/permission behavior.

Executed precise safe filters:browser signing fixture, browser extension packages produce, test selection, browser registration compensates, browser native messaging command preserves. Final16RUN/16PASS,0skip/fail/not_run,exit0. C# build --no-restore:0warnings/0errors. No full197fixture suite or native acceptance executed.

browser-signing-isolation.ps1, platformhost-test-selection.ps1, browser-native-messaging.ps1:passed. New signing gate added to ordinary build's source checks; actual scripts/build/package wrappers not run. spec-contracts87docs/12owners passed;git diff --check exit0. Logs browser-signing-final.log/browser-signing-closeout.log. C# build/test only, no Cargo needed for unchanged Rust.

## Limits
No remaining compiled/test/source-gate blocker for this slice. Actual Chrome/Edge/Firefox installation, native registry transaction and signing-file permissions in user installation remain separately authorized acceptance; not certified by temp signature tests or unchanged-method comparisons. Production key was deliberately not read or hashed to establish evidence. Other4restricted fixtures still need isolation independently. No product installer or release artifact delivered.
