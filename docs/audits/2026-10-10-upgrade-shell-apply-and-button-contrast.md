# Upgrade shell apply and primary contrast — 2026-10-10

## Report and bounded scope
User reports Global settings cannot Apply after updating a machine with older NxTools, displaying desktop_executable_unavailable, and light neutral Apply background with white/unreadable text. No other-machine logs/install identity were collected. Scope: ordinary current-user startup/tray desktop resolver and Global Apply button foreground/disabled presentation; no service admission/deployment, registry auth, IPC or vendor rewrite. Current formal installer/install/protected service untouched by this turn.

## Reproduction
- Temporary-file C# test selects actual resolver seam with new package desktop/hash and old protected bytes. First run1RUN/1FAIL: current valid package rejected when protected approval false. No real Run registry/service mutation.
- GPUI primary fixture records paint-time text_style through actual component and simulated hover. First run1FAIL: primary foreground was overridden by component styling. This is a reproducible state-style defect, not proof of which transient state produced the user's white text screenshot.
- Subsequent recoverable failures: hash rejection threw InvalidDataException before conversion to unavailable; overbroad fake protected approval accepted wrong candidate; duplicate hover styling panicked; missing closing quote in updated error test blocked parsing; full497/1 then exposed one old error-copy expectation. Each corrected locally, no relaxed authentication/disabled/containment assertion.

## Fix
C# production shell resolver validates configured desktop as exact NxTools.exe sibling of running PlatformHost, current embedded DesktopSha256 and stable opened file identity via existing TrustedBinary. Outside-directory env path, wrong filename, malformed hash, tampered/missing file rejected. Protected fallback is separately restricted to the explicit protected candidate and retains IsApprovedProtectedDesktop. Old protected indexer tuple no longer blocks ordinary shell Apply on a genuine new installed package. No indexer service/pipe privilege approval changes, no hand-authored hashes.

Global settings primary_action_button explicitly styles label dark on light neutral background, with darker neutral disabled background and opaque readable secondary text. Retains vendor Button disabled/focus/KeyUp/click boundary and fixed72×32 global Apply slot; no extra hover handler or vendor changes. Actual paint-time normal/hover/disabled color, unchanged bounds, enabled click and disabled no-dispatch tests pass.

Errors now explicitly state config is saved but Windows startup/tray changes were not applied. Existing durable-save→correlated Windows-result semantics remain; no false saved=applied success or automatic close on failure.

## Executed verification
- Final Desktop498/498,0ignored,23.79s; production/test strict Clippy,workspace --tests,fmt pass.
- Exact C#3RUN/3PASS: current-release startup resolver, new old-protected upgrade resolver, bounded shell failure codes. Temporary files and isolated test registry keys only. No unrestricted native/DOpus harness.
- PlatformHost Debug build0warnings/0errors.
- Seven affected source gates and specs87/12 checked; source gates are not installed Windows acceptance.

No package/install/ordinary product launch, actual user startup/tray change, service repair/stop/start, real Clipboard/DOpus/network operation or commit/push. Existing formal09:46 installer does not include this fix. Other-machine installed upgrade/save and actual visual/DPI acceptance require a new authorized formal package. Pinned provider limitations unchanged.
