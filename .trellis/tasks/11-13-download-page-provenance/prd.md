# Durable download page provenance

## Goal
Download history retains the page a file came from independently of the file's address and the request Referer.

## Requirements
Carry optional source-page URL/title from existing browser metadata through its typed decoder, request envelope, SQLite persistence and selected-task projection. Preserve download/final URL, frozen Referer/Cookie context, redirects and user-edited file names. Missing legacy facts stay unknown without background probing or bulk rewrite. Never log page URLs/titles.

## Acceptance
After `11-13-download-grouped-completion`, a browser-to-engine fixture and real SQLite reopen prove all three address roles survive independently, redirects preserve provenance, old records load and edited names stay untouched. Command/event/IPC roundtrips remain compatible under the project's version rules. Native installed-browser acceptance remains separate.
