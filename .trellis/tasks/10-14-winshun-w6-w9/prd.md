# W6–W9 cross-feature extensions

## Goal
Provide a reusable extension foundation for Downloads, Quick Launch, Clipboard and tools. Plugins can contribute actions/providers, handle requests, observe explicitly enabled events and call supported feature operations. This is not a launcher-only script runner. WinShun informs mechanics; the user's cross-feature requirements govern scope.

## Authority and confirmed decisions
User approved the complete W6–W9 implementation after confirming full ordinary-user trust, manual actions plus individually enabled default-off automatic events, download URL resolution, and default single-handler selection with explicit ordered chains. Source implementation and isolated joint acceptance are complete; no packaging/install authorization is implied. W1–W5 source evidence remains separate in ../10-14-winshun-w2-w5/outcomes.md.

## Task map / canonical order
| Deliverable | Child | Gate |
| --- | --- | --- |
| W6 declarative command packages + common management | ../10-14-winshun-w6-command-packages | Independently verifiable; reuse current commands and retain package identity |
| W7 one-shot scripts + W8 stable merge + W9 settings/disclosure + feature integration | ../10-14-winshun-w7-w9-trusted-plugins | Depends on W6; scripts cannot be enabled before all three and cross-feature lifecycle gates pass |

## Requirements
- R1 — Declarative packages: bounded versioned JSON imports website search, programs, folders and fixed command lists. Preview before import, default-disabled, explicit enabling, provenance and normalized-keyword/ID conflict feedback. Invalid/update failures preserve prior state and unrelated manual commands; no silent overwrite or import-time execution.
- R2 — Trusted scripts: Python, PowerShell (Windows PowerShell and pwsh), Node and EXE use installed/configured runtimes. Only trusted AND enabled content runs. Trust grants current ordinary-user authority in full, not individual permission enforcement; no host runas/UAC/SYSTEM path. Lifecycle/resource limits always apply.
- R3 — Disclosure: common UI shows source/version/runtime, supported modules/entry points, declared permissions with purposes, trust/enablement/event states and errors. Show “信任后，此插件可使用当前用户的全部权限。以下权限由插件作者声明，并非系统访问限制。” Missing declarations do not imply unavailable permissions. Do not claim hostile-code isolation or reversal of external side effects.
- R4 — Lifecycle: finite deadlines/input-output/results/memory/process/concurrency/queue budgets, pre-execution process-tree ownership, cancellation and host-exit cleanup. Supersede/disable/revoke/disconnect retire stale work; incomplete cleanup holds its bounded slot, never unlimited replacements. Trust/declaration/content changes invalidate existing approvals and running invocations.
- R5 — W8: namespaced stable result IDs, generations and quotas; partial/replacement updates preserve selection, and stale output cannot publish or execute. Preserve W5/IME/deferred callbacks/current feature operation owners. Quick Launch keyword and ordinary-provider registrations are supported entry points, not the universal trigger.
- R6 — W9: host-rendered text/bool/number/choice/file/folder/password fields with explicit Save/Cancel and validation. Passwords use Windows Credential Manager; no plaintext config, argv, environment or diagnostic secret output. Intended trusted-plugin access to configured secrets is explained.
- R7 — Cross-feature freedom: common versioned invocation/host-call/result/error/cancellation interface and feature-specific typed Adapters. Downloads/Finder/Clipboard/tool modules keep state, persistence and validation. New extension points can be registered without replacing management/runtime or exposing raw GPUI entities/HWND/native pointers. Supported first-release operations are in design.md; unavailable operations return explicit unsupported errors, not ignored success.
- R8 — Manual and automatic: manual selected-item/tool actions plus independently enabled automatic lifecycle events. Default-off automatic handlers; committed events only, no render/pointer/byte-tick stream. Origin/causality/dedup/re-entry bounds prevent duplicate tasks and clipboard feedback loops. Plugin failures do not undo successful underlying feature transactions.
- R9 — Download parsing: match an original HTTP(S) page URL, run plugin-specific parsing/network/transformation, return a processed download request before normal probe/acceptance. Example `https://XXXXX.github.com/XXXXXX/main` is illustrative, not a URL to fetch or hardcoded special case. UI/browser/CLI/internal/background paths share semantics; original URL, resolved URL and plugin provenance remain distinguishable.
- R10 — Resolver composition: default selects one handler, with explicit user override and deterministic conflict feedback; explicit ordered chains are supported. Stage output becomes next stage input; whole-chain deadline/depth/cycle controls, per-stage trace and failure stop. No unordered broadcasting or silent fallback to other scripts; an explicit bypass-original action remains available on failure.
- R11 — Download compatibility: preserve probe/confirmation generation, scheduling/category/name/rate/TLS/network policy, idempotency, browser receipts, media selection and resumable resource validation. Validated original-to-resolved identity must survive Probe→Submit without duplicate parsing; no credential carryover merely because a URL changed origin. Blob import remains outside URL parsing. Expired pre-acceptance resolution offers explicit retry; accepted transfers do not silently rerun site plugins per segment/resume.
- R12 — Verification/privacy: deterministic owned fixtures and localhost only; low-noise correlation/stage/count/elapsed/native-code diagnostics. No query/URL/path/clipboard/request headers/cookies/secret/child-output logging. Preserve unrelated dirty work, no commit/push or native user-state operations.

## Acceptance criteria
| ID | Requirements | Observable evidence |
| --- | --- | --- |
| AC1 | R1 | Import preview runs nothing; good imports save/reopen/enable/disable correctly; invalid/duplicate/conflicting/update failures retain prior package/manual state |
| AC2 | R2–R4 | Untrusted/disabled/high-integrity runs spawn nothing; owned fixture proves assign-before-resume, deadline/overflow/cancel/revoke/host-exit cleanup and bounded stuck slots |
| AC3 | R5 | Actual Quick Launch input/batch/action tests preserve selected stable ID and reject old-generation/deferred execution, including ordinary provider plus indexed results |
| AC4 | R3,R6 | Actual manager/settings UI discloses full authority/declarations; Save/Cancel/validation/changed-content approval and synthetic credential compensation/redaction pass |
| AC5 | R7,R8 | Real typed feature actions and post-commit events work for downloads, clipboard and tools; default events run nothing, repeated/plugin-produced events do not recursively duplicate effects |
| AC6 | R9–R11 | Synthetic page resolver produces localhost artifact through common UI/browser/CLI/background command ingress; no-match unchanged; Probe→Submit and duplicate replay do not rerun unnecessarily; credentials/cancel/trust/receipt remain coherent |
| AC7 | R10 | Single selection and two-stage resolver chain pass; ambiguous registration, stage failure, cyclic rewrite, disabled stage and whole-chain timeout produce truthful terminal errors without creating a task |
| AC8 | R12 | Scoped strict Rust lint/check/fmt, owner/IPC/full Desktop/C# safe suites, affected architecture/spec gates and example-protocol tests pass; actual counts/evidence levels recorded |

## Out of scope
W10 resident plugins; arbitrary injected UI/DLLs/native pointers/direct internal DB/index access; marketplace, automatic plugin/runtime download or update; MCP/AI; OS file/network sandbox or individual permission switches; packaging/installation/service mutation/real third-party scripts/real Clipboard or external URL operations; commit/push. Existing manual elevated commands retain their current behavior; they do not confer plugin elevation.

## Risks and deferred acceptance
Trusted code has full normal-user access; declared permissions are not enforcement. Job/deadline ownership handles ordinary process trees, not arbitrary malicious escape via unrelated OS services. Original caller credentials and private item data are intentional plugin inputs only under declared interfaces and trust, never telemetry. Interpreter availability is a runtime state, not a planning blocker. Native Windows edge cases are proved with owned fixtures during implementation; installed physical/DPI/reader and real plugin behavior remain separately authorized acceptance.

## Delivery status
Approved scope implemented and safe joint source gates passed; see outcomes.md, usage.md and child runtime-checkpoint.md. Development-only script enable rejection removed after joint gates, with explicit trust/enable and execution-time identity checks retained. Installed physical/DPI/reader/live credentials and Release acceptance remain unperformed, not blocking questions or fabricated passes.
