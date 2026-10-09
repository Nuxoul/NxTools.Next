# Clipboard/launcher defects

Repair KeepOpen actions hidden on queue acceptance; Windows quoted argument paths; purge floor max(7, retention_days); orphan import staging/assets and partial-failure semantics; storage migration config failure must not diverge worker/restart state. Remove raw query/path/resource payload logging. Preserve runtime data, favorite/pinned behavior, existing copy/paste result semantics and explicit command tokens.

Acceptance: production dispatch keeps copy actions visible; real argument preparation preserves single backslashes; retention matrix; failed import cleans owned new assets without deleting reused resources; failed migration config commit rolls back or retains durable recoverable ownership. No real clipboard/history migration/user data operations. Normal file-error paths remain retryable.
