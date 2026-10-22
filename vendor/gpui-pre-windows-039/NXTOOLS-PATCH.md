# NxTools passive Windows preview patch on0.3.9

User-authorized2026-10-21 Kit0.8.0 upgrade carries only the existing approved2026-10-10 Windows message-policy fix to published gpui-pre-windows0.3.9 (archiveSHA256513d4f0fc7d596d3bdef2a73a524ffb83dabc95bf484f2855a698e7f431290a9). Apache-2.0 LICENSE-APACHE retained; NXTOOLS-UPSTREAM.json records every shipped file hash. Old0.3.8vendor remains inactive historical evidence, not active code.

Only src/events.rs differs: WM_MOUSEACTIVATE reads live GWL_EXSTYLE; WS_EX_NOACTIVATE -> MA_NOACTIVATE, otherwise originalMA_ACTIVATE. C# still applies Windows policy, Rust owns staging/generation and GPUI owns the in-process message. No other backend behavior, registry edits, injection, click-through, primarypanelnonactivation or focusstealing. Matcheddefault backend remains gpui-pre0.3.9, no Fast.

Sourceprovenance gate: python tests/architecture/gpui-release-contracts.py; independent --upstream compares shippedsource hashes. Existing tests/native/gpui-kit-acceptance.ps1 owns separately authorized real HWND response3/ordinary1/flagdisable1/re-enable3 and Quick/Clipboard reuse. Source/compiledtests do not imply this newbackend passed physical/native/DPI/reader/installed acceptance. No new runtime logs.
