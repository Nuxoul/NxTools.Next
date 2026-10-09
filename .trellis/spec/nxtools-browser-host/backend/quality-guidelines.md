# Browser host quality

Evidence: `repo:rust/apps/nxtools-browser-host/src/lib.rs#tests`, `repo:rust/apps/nxtools-browser-host/src/blob_upload.rs#tests`.

Test bounded fields/version/acceptance identity at real typed-command seam, malformed/oversized frames, batch IDs and temp socket ownership/cleanup. No real browser or registry operations. Library/extension/source checks follow [verification](../../shared/verification.md); registration/native presentation are separate owners. Preserve lease contract below.

## Scenario: Exclusive Blob Receiving lease

### 1. Scope / Trigger
- Changes to upload claim, socket body IO, idle expiry or cleanup/capacity release.

### 2. Signatures
- UploadState::Receiving; `reap_expired_uploads(registry, staging_root, now)` excludes it; MAX_RECEIVING_DURATION = two minutes.

### 3. Contracts
- One owner per Receiving chunk, at most 1 MiB; duplicate claim rejected. Idle reaper cannot delete owner state/staging during its write.
- Owner enforces total body deadline and per-read timeout min(remaining, 30 seconds), then owns finish/failure cleanup/capacity release.
- Pending/Uploading/Complete idle expiry semantics remain; no payload or capability token logging.

### 4. Validation & Error Matrix
- Duplicate Receiving -> reject without replacing owner; abandoned body/deadline -> bounded failure/cleanup.
- Write/rename failure -> preserve typed failure, release state; idle expired non-Receiving -> cleanup and capacity release.

### 5. Good/Base/Bad Cases
- Good: claim, idle-reap attempt, body completes with state still owned.
- Base: idle partial upload expires normally.
- Bad: TTL removes Receiving mid-write or active exemption has no owner-side deadline.

### 6. Tests Required
- Synthetic expired-Receiving/duplicate claim and real socket claim/reap/finish; assert exclusive ownership and output.
- Keep write/rename cleanup failure and idle-capacity tests; BrowserHost library and extension contracts.

### 7. Wrong vs Correct
```text
Wrong: expires_at <= now -> remove every state.
Correct: state != Receiving && expired -> reap; Receiving owner enforces bounded IO.
```
