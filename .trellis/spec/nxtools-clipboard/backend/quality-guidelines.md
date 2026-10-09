# Clipboard quality and ownership

Evidence: `repo:rust/crates/nxtools-clipboard/src/lib.rs#tests`, `repo:rust/crates/nxtools-clipboard/src/archive.rs`, `repo:rust/crates/nxtools-clipboard/src/storage.rs`.

Temp SQLite/archive/assets assert full cursor continuation, favorites/pinned retention, native-format references, late-partial imports and exact remaining ownership. Use real SQL-trigger failures/readable backups. Desktop worker/config-delay tests protect publication; no live history. [Verification](../../shared/verification.md), [engineering](../../shared/engineering.md); retain scenario below.

## Scenario: Archive ownership, purge and migration commit

### 1. Scope / Trigger
- Changes to import staging/cleanup, deleted-row purge or worker/storage configuration publication.

### 2. Signatures
- `ClipboardHistory::import_archive`, `referenced_asset_paths`; `ClipboardArchiveError::Partial` exposes committed import counts.
- Worker `migrate_storage(destination, config_path)`; desktop `save_clipboard_section` uses AppConfig::update_section.

### 3. Contracts
- Purge deleted rows after max(7, retention_days), preserving favorite/pinned soft-delete semantics.
- Preflight only importable ZIP rows before database writes. Track only newly owned extracted/temp resources; reused shared assets are never owned by this import.
- Late database failure preserves committed rows, returns partial counts and removes only newly owned resources proven unreferenced. Reference-query failure retains conservatively.
- Serialized worker prepares/opens replacement, commits destination config, then switches history/paths. Ordinary settings save never owns storage_directory; preserve latest disk root. Delayed UI publication reloads disk.

### 4. Validation & Error Matrix
- Preflight/missing asset failure -> no inserted rows, remove owned resources/temp, shared files unchanged.
- Late row failure -> explicit Partial, committed row assets retained, unreferenced new assets removed.
- Config commit failure -> source worker/config authority; stale UI S after durable D -> ordinary save keeps D.

### 5. Good/Base/Bad Cases
- Good: second row fails; first row and icon remain, second orphan disappears.
- Base: unknown/skipped row extracts no assets.
- Bad: delete reused files, claim all-or-nothing after committed rows, or publish worker destination before durable config.

### 6. Tests Required
- Real temporary ZIP/missing-entry/shared-asset and SQL-trigger two-icon regression; assert exact remaining files equal committed references.
- 1/7/14/30-day retention matrix and favorite/pinned exclusions.
- Real synthetic worker/config failure and delayed-completion ordinary-save race; assert disk/live root destination, no real user clipboard access.

### 7. Wrong vs Correct
```text
Wrong: import error -> remove every extracted/reused path.
Correct: owned_new_paths minus committed_database_references -> cleanup; preserve shared files.
```

Cross-layer migration authority and evidence: desktop quality guidelines and `docs/audits/2026-10-08-sol-review-remediation.md`.
