# Download file icons

## Implementation

Download rows request the original Windows Shell icon at48px and render it inside the existing transparent40DIP Contain slot. Unfinished tasks use attribute-only type targets; completed files use their original file target. Status/action glyphs remain independent from file identity.

Each high-resolution Shell query owns a raw IImageList interface reference and releases only that reference. The original bounded STA runner still permits eight real operations and uses its existing logical deadline. Every acquired HICON is destroyed after PNG conversion.

The existing cache retains128memory entries,2048disk entries and per-key generation deduplication. Persisted PNGs are decoded before admission and unreadable entries are regenerated through the same owner. Memory hits retain the original existence-only fast path. Native failures preserve Win32/HRESULT with fixed `shell_icon_*` categories; diagnostics exclude targets, filenames and raw exception messages.

Failed-resource retry budgets are scoped to the live PlatformHost connection. A real disconnect retires the failures and pending commands, permitting unresolved icons to retry after reconnection. Duplicate connection notifications do not rearm failures or duplicate in-flight requests. Existing retry tokens and resource assignments exclude retired work; successful resources remain reusable. Until a PNG resolves, rows use official `IconName::File` in the existing30DIP fallback slot.

## Verification

The native icon/cache selection runs12fixtures with12passes, no skips or failures. Coverage includes48cold-cache queries at concurrency8, corrupt persisted PNG recovery, native error-code retention, completed executable extraction, folder/type raster bounds and all six existing cache lifecycle fixtures. Test roots are isolated; no Registry, service, Shell-open or installed-app changes occur.

## Acceptance boundary

Native extraction and controlled GPUI tests do not establish installed visual, physical-input or mixed-DPI acceptance.
