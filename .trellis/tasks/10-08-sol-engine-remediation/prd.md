# Engine/browser defects

Fix adaptive shrink completion being skipped after pending identity clear, DASH adopting a nonempty fragment inconsistent with a completed checkpoint, and Blob Receiving state expiring during active I/O. Preserve resumable identity, signed-URL refresh, retry bounds and capability single-writer guarantees. Parent separately owns Native Messaging registrar transaction.

Acceptance: actual worker->durable plan shrink and resume artifact scenarios tested; receiving claim remains exclusive and immune to idle reap while bounded active work runs; terminal failure cleanup still releases capacity. No HLS finalizing cancel behavior change or speculative cache/privacy policy changes.
