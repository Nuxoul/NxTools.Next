# CLI errors and output

Evidence: `repo:rust/apps/nxtools-cli/src/lib.rs#CliError`, `repo:rust/apps/nxtools-cli/src/lib.rs#EXIT_SUCCESS`, `repo:rust/apps/nxtools-cli/src/lib.rs#CliHostPolicy`.

Existing exits: success0, syntax2, host unavailable3, task not found4, invalid state5, request rejected6, operation failed7, protocol incompatible8. Preserve typed incompatibility downcast and human/JSON mode; don't collapse every cause to exit1. Batch actions report per-item results and replacement ID where redownload changes identity. Operation error != transport absence. Requested task/path output is legitimate, but raw anyhow/path/URL context is not diagnostic-safe. [Shared errors](../../shared/engineering.md).

Archive `--json` emits one schema1/request_id/command/success/result/error object on success AND
failure, including syntax. No raw rejected argv/secret echo. Default fail cancels original conflict
prompt; create-destination consent is limited to that exact ticket/reason, not lowspace/risk/budget.
Preserve native terminal/code/commits/skips/journal/cleanup; ACK/cancel receipt is not terminal.
Expected prompt refusal uses6, native/cleanup failure7, incompatible8; same original exits otherwise.
Noninteractive encrypted TEST/extract returns password_required, no password argv/env/dialog/retry.
