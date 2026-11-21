# Clipboard feature enablement

## Boundary

The persistent feature Switch saves the existing Clipboard section, then ConfigureClipboard reaches the original EventWindow owner. Temporary capture pause remains independent. ApplyClipboardCaptureEnabled clears pending retries, samples a metadata-only baseline and changes the feature gate; it does not read clipboard contents or backfill the disabled interval. A zero sampled baseline is retained as a discard baseline rather than rejecting feature enablement. Real operation exceptions still keep capture closed and return the original native error code.

The existing registered EventWindow fixture covers disable -> zero-sequence enable -> queued zero update/retry with no reads -> changed sequence captured once. It also checks a genuine sequence exception remains failure/native5. Original temporary pause, bounded queue, cancellation, retry cleanup and diagnostics owners remain unchanged.

## Verification

The zero-sequence enable assertion failed before the repair. C# build passed0warnings/0errors. Reviewed execution selected8/run8/pass8/skip0/fail0: six Clipboard pause/enable fixtures and two owned-window corner fixtures. Clipboard operations are injected through the production EventWindow/WndProc seam; no live clipboard, listener, hotkey, timer or global input is used. Native platform acceptance and the report machine's reason for returning zero are separate.
