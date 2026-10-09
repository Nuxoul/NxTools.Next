# Project quality standards and core UX refinement

## Goal
Establish source-backed, maintainable engineering/UI standards and a consistent, refined Windows desktop experience for Downloads, Quick Find, Clipboard and Link Detection. Quality means reliable workflows, clarity, measurable checks and coherent interaction, not adopting rules because another repository has many stars.

## Consent and state
The user approved creating the planning/task schedule on 2026-10-09 after the proposed scope. This task remains planning; approval of the final artifact summary is required before implementation. Preserve all existing dirty paths. No commit/archive, package/install, product launch, real DOpus navigation or Indexer service mutation is authorized.

## Confirmed context
- SOL remediation evidence is in `docs/audits/2026-10-08-sol-review-remediation.md`. Historical engine102/103 failures were causally closed by the 2026-10-09 fixture-only correction and one integrated103/103 run (route item1); live pointer/DPI and LocalSystem release gates remain pending.
- Twelve Rust workspace packages, C# PlatformHost and browser-extension require owner-specific guidance. Route item2 populated applicable specs and removed templates on2026-10-09;15 existing scenario bodies across9 docs remain intact. Standards coverage is documented separately from product/native compliance.
- Existing UI authority is split: `docs/ui-remediation-plan.md:75-117`, the later approved `.trellis/tasks/10-02-staged-desktop-ui/design.md:3-25`, and current `ui/metrics.rs:5-50` differ on spacing/control/radius tiers. Distinguish current implementation, approved target and exceptions; do not silently treat current code as product authority.
- Existing phased UI plan is `.trellis/tasks/10-02-staged-desktop-ui/implement.md:5-9`. Names/order are canonical and must not be renumbered.
- Existing link task owns default copy-only, opt-in auto-open, generation-safe configured hide, single HWND, bounds/topmost and actual transparent-corner acceptance. Current tools page has one Link Detection card; pure-text clipboard actions already exist.

## Requirements
- R1: consolidate one discoverable authority hierarchy for architecture, engineering quality and UI guidance. Populate source-backed package/layer specs; remove non-applicable templates; preserve verified behavioral contracts. Mark observed conventions separately from required targets and tracked gaps.
- R2: cover typed boundaries, ownership, async cancellation/generation, GPUI leases, configuration/storage transactions, errors, payload-safe diagnostics, test seams, dependency hygiene and review/check commands. No arbitrary file-size mandates or abstraction for its own sake.
- R3: define semantic UI roles, density/type/layout, component interaction states, keyboard/focus, bounded motion/reduced-motion behavior, DPI/window geometry and accessibility expectations. Resolve conflicting authority before token migrations; later approved user design is the target, behavior/native geometry constraints remain mandatory.
- R4: refine existing core surfaces in canonical stages without changing domain semantics or default actions. Each surface has a behavior/state inventory, concrete defect evidence, before/after visual review and interaction regressions. Do not presume source-complete slices need rewriting.
- R5: Link Detection improves recognition/feedback/presentation reliability within its existing task. No implicit browser open, hover-extension of timeout or second UI runtime.
- R6: evaluate adjacent small tools for demonstrated value, existing overlap, privacy, complexity and upkeep. Candidates: text cleanup, explicit URL tracking cleanup, local QR generation and batch link extraction. Choose at most two recommendations; implement none in this scope.
- R7: task scheduling records dependencies and honest failure/deferred states; queue prompts are self-contained and must not bypass user approval, visual stage acceptance or restricted native operations.

## Acceptance criteria
- AC1/R1–R3: every first-party package/layer has accurate navigation, evidence-backed applicable guidance and no unaddressed template placeholders; historical reference/task evidence retained. UI authority conflict matrix resolves target/observed/exception status without mass edits.
- AC2/R2: critical checks have reproducible commands and evidence classifications; automated rules cover applicable boundary/token/privacy contracts and report actual legacy gaps instead of hiding them. Code compilation is not native lifecycle acceptance.
- AC3/R4: accepted UI stages follow `Foundation`, `Downloads`, `Quick Find`, `Clipboard + Tools`, `Global settings + supporting surfaces` in order, with complete affected-state matrix and unchanged product workflow regressions.
- AC4/R5: link-panel acceptance satisfies its existing PRD; unavailable real alpha/corner/topmost/input checks remain explicitly pending.
- AC5/R6: evidence-backed candidate report distinguishes existing/partial/new functionality, risks and estimated scope; no added tool implementation.
- AC6/R7: every queue item points to owning artifacts, stops on relevant failure and does not claim downstream stages started. Existing review failures and restricted release checks remain visible.

## Out of scope
Mass source rewrites, switching rendering runtime, new product defaults, domain/protocol redesign, arbitrary performance targets without baseline, changes to protected deployment/security policy, automatic real clipboard inspection, packaging/install/service operations, commit/push/archive and new utility implementation.

## Decisions / deferred items
Retain existing compact, restrained desktop identity and later approved palette; styling work requires real screenshots/interaction review at the relevant stage. Standards docs may proceed with an explicit unresolved native-release gate, but no new release acceptance is claimed. Module-specific detailed changes are planned in their existing owning tasks when their stage arrives; this parent does not pre-authorize speculative improvements.