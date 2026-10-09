# Evaluate adjacent small utilities (no implementation)

## Goal
Recommend a small number of coherent, privacy-preserving utilities that improve existing Downloads/Quick Find/Clipboard/Link Detection workflows rather than building an unrelated toolbox.

## Requirements
- Inspect existing first-party code/routes and tests before calling a feature new. Pure-text paste already exists; calculator/custom commands/URLs/batch download coverage must be verified.
- Evaluate text cleanup, explicit tracking-parameter cleanup, local QR generation and batch link extraction/deduplication. Candidate list is research input, not implementation consent.
- For each candidate state user job, current/partial/missing coverage with evidence, inputs/outputs, host surface, reuse, privacy/security risks, boundedness, dependency/maintenance cost and testable acceptance proposal.
- Recommend at most two highest-value candidates; explain rejected/deferred options. Signature URLs never silently cleaned; original retained and transformations explicitly initiated. Local text/URL contents never uploaded/logged.

## Acceptance criteria
One source-backed recommendation report with overlap matrix, priorities, risk/dependency analysis and narrow proposed follow-up specs. No tool implementation, configuration/default change, dependency addition or real clipboard/credential inspection. Parent scope and operational restrictions apply. New implementation always needs separate user approval.

## Out of scope
OCR/screenshot/cloud services, bulk file mutation, browser/network introspection, QR recognition engine, feature-complete general-purpose toolbox and product source edits.

## Decisions
Evaluation only; no candidate selected by popularity or agent preference. Run after core quality sequencing is reviewed, but read-only evaluation may proceed without release/native acceptance. Lightweight document-only task: PRD + curated research/check context are sufficient; no product technical design is claimed.