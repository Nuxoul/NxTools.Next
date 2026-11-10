# PlatformHost input isolation and extension gesture trust

## Scope
Fix H5, H4 and M1 and implement U3/U5 from the2026-10-31 audit/proposal. The implementation request defines the seams and authorizes isolated tests only.

## Requirements
- H5: detach launch_process and ensure_indexer_service through the existing dispatcher; dispose returned Process without changing results. Verify install serialization, Rust caller/result ordering and detached Drain.
- H4: install/run/uninstall WH_KEYBOARD_LL on one dedicated background thread with blocking GetMessage and bounded WM_QUIT disposal. Hook-thread-only double-tap detectors consume an immutable Volatile-published configuration and reset generation. Callback has no IO/log/lock wait; recognized gestures still PostMessage to EventWindow. Configure DoubleControl retries unavailable installation before returning the existing error.
- U3: paired typed C#/Rust observable install_failed/thread_exited/reinstalled event and current Hello capability snapshot; no silent-removal guesses, polling or heartbeat. Existing Quick Launch settings show official Alert and one deferred re-enable action using ConfigureQuickLaunch; one change notification; recovery removes Alert. Correlated metadata/native-code diagnostics exclude key content.
- M1/U5: require isTrusted at direct click/drop/dragover and download-producing overlay click entries. Synthetic events send no download and do not prevent browser defaults; native browser hand-back remains. Preserve existing videoPosterUrl/pageThumbnailUrl changes. Document Cookie scope, page-hook trust and independent drop-target semantics.

## Acceptance
Behavior-red/green at detached dispatcher, hook ownership/reset/disposal, paired event serialization and production TestApp Alert/re-enable/recovery seams. All Node suites and isolated real-DOM/CDP trusted-input gates with blocked network. C# reviewed fixture registration/effect classification; no live native input/UAC or real profile access. Required protocol/IPC tests, filtered quick-launch desktop tests, strict scoped Clippy, scoped rustfmt, diff/spec/GPUI gates. Report exact counts, known unrelated failures and native/visual limitations.

## Constraints
Preserve existing dirty work. No commits/push, packaging/install/product launch, real services/network/browser profiles/user data, dependency/vendor/palette changes, page-hook trust redesign, drop-target semantics change or unrelated audit fixes. Every Cargo process uses one job, no incremental, codegen-units256 and offline; only one Cargo process at a time. Desktop tests use the requested single-package debug=0 profile override.
