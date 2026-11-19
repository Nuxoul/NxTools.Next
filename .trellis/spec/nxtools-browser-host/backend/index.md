# Browser host backend

Evidence: `repo:rust/apps/nxtools-browser-host/src/lib.rs#BrowserRequest`, `repo:rust/apps/nxtools-browser-host/src/blob_upload.rs`.

## Pre-Development Checklist
Read [download/browser reference](../../../../docs/reference-code/download.md), [engineering](../../shared/engineering.md), [structure](directory-structure.md), [persistence](database-guidelines.md), [errors](error-handling.md), [logging](logging-guidelines.md), [quality](quality-guidelines.md), [extension](../../browser-extension/index.md). Native Messaging and engine wire versions/bounds are distinct.
Automatic browser ownership follows [controlled handoff](../../browser-extension/controlled-handoff.md); window admission is Pending, durable Held cannot transfer, and Commit requires the original task identity.

## Quality Check
`cargo test -p nxtools-browser-host --lib`; real temp socket Blob and identity/malformed/batch validation; extension contracts and browser-native-messaging gate. Strict checks per [verification](../../shared/verification.md). No real browser/download/cookie/registry operations. [Entry](../../index.md).
