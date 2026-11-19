# Elevated foreground input for Quick Launch and clipboard

## Goal
Keep NxTools.exe and ordinary PlatformHost unprivileged while supporting double Ctrl and clipboard paste into administrator windows through the existing authenticated same-session helper.

## Requirements
- Reuse the protected Indexer → broker → linked-administrator helper and its persistent private C# channel. No new executable, manifest/uiAccess, service role or public Rust protocol.
- Helper keyboard callbacks only detect gestures and PostMessage. Foreground privilege checks and forwarding run on its message thread. Ordinary/elevated sources are mutually exclusive while helper ownership is confirmed; RegisterHotKey behavior stays unchanged.
- Replay enabled/recording/trigger/reset configuration on connection. Ignore obsolete generations; release hook/message resources on EOF, exit and disposal. Preserve ordinary keyboard health independently.
- Clipboard data is written once by ordinary PlatformHost. Only target/focus HWND and correlation ID cross into elevated paste input. Preserve original target, four activation attempts, modifier release, foreground validation and unconfirmed WM_PASTE failures.
- Missing protected deployment/linked token/helper retains ordinary behavior with bounded diagnostics. An input request whose result is unknown must fail rather than replay possibly completed input.
- Keep directory capture/navigation authentication, context ownership and cleanup unchanged.

## Acceptance
- Reviewed isolated fixtures cover actual paste coordinator delegation, copy-only/write failures, unavailable/failure/unknown results, foreground revalidation, source ownership, configuration replay and cleanup.
- C# build, targeted console fixture catalog/execution and relevant architecture contracts pass, or concrete unrelated blockers are reported.
- No installation, packaging, real global hooks, clipboard mutation or administrator-window interaction during implementation verification. Live protected-install/native acceptance is a separate step.
