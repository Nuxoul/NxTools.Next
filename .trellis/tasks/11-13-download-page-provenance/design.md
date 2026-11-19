# Design

## References and boundary
Verified ABDM, legacy and Mister entries plus the real Next capture -> BrowserHost -> SQLite chain are recorded in `docs/reference-code/mister-download-manager.md`, Completion grouping and source-page facts. Mister is behavioral reference only. Referer is a request header, not recoverable source-page identity.

## Data contract
Add optional DownloadSourcePage { url, title } to DownloadRequest with serde default and skip-none serialization. Add bounded sourcePageUrl/sourcePageTitle to the existing native payload and map them once in build_browser_download_request, covering single, batch and controlled prepare. Use only existing captured page facts; contextUrl stays cookie context. Empty/missing page stays None. Existing request_json persistence and TaskDetail carry metadata without a schema migration.

## Compatibility and lifecycle
Absent metadata must serialize identically for existing receipt fingerprints. Exclude provenance from resolution_resource_fingerprint; it does not identify the remote bytes. Do not alter resource URL, final redirect URL, frozen Referer/Cookie, user-edited names, paused URL replacement or generation-scoped controlled handoff. Preserve the existing BrowserHost IPC/cancellation/result owners. No metadata requests, cache persistence expansion, payload logging or new command.

## Diagnostics
Retain existing correlation across extension, BrowserHost, IPC and engine acceptance/terminal events. No URL/title values enter diagnostics. Reject oversize payload fields at the decoder using current bounded-input rules.

## Acceptance
Obtain a red real framed BrowserHost -> pipe -> engine -> SQLite reopen test. Verify independent address roles, redirect preservation, edited names, absent legacy metadata and unchanged receipt/resource identity rules. Retain native wire/controlled handoff roundtrips and extension contract tests; installed-browser acceptance remains separate.
