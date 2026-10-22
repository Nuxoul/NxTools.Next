# Execution

1. Read-only service counters/cache metadata and reference review; no service changes. Preserve original dirty Indexer and notification work.
2. Add production writer process-byte regression first; measured red8,285,532bytes for three one-row updates against2,761,844-byte overlay.
3. Keep version3base and legacy delta loader, add checksummed append transactions/proportional change encoding, shared successful-checkpoint state and bounded atomic checkpoint. Recovery accepts only committed verified frames; truncated tail resumes last complete cursor and is removed before retry. Preserve existing persistence failure/retry and search publication ordering.
4. Real temporary-file replay/torn/corrupt/legacy/checkpoint/reset tests plus synthetic USN/maintenance/admission/search; no unrestricted volumes. Strict Indexer/workspace/fmt and source gates. Extend release idle gate with I/O counters; do not run RestartService without separate authority.
5. Record exact bytes and final checks/limits, task/spec/reference updates; unchanged installer/service hash and no package/install/restart claim.
