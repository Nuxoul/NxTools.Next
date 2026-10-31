# Phase E workspace integration convergence — 2026-10-22

**Workspace integration blockers CLOSED at the source/compiled-test level. Not overall Phase E or
first-release PASS.** Only [current checkpoint](phase-a-checkpoint.md) owns phase status. Historical
[994/1 delivery](phase-e-delivery-outcomes.md) and [997/2 independent acceptance](phase-e-delivery-acceptance.md)
remain unchanged; this is a new observation, not a rewrite or a new archive implementation slice.

## Scope, latest ownership and dispositions

Opening HEAD97d9a9d/clean tree already contained later owner fixes. Baseline2876tracked files captured
before implementation (first attempt missed2Git-escaped Unicode paths; corrected baseline retained).
No active same-workspace edits observed; unrelated NxMonitor Cargo builds were waited out, not killed.

| Item | Current cause / narrow disposition | This round's evidence |
| --- | --- | --- |
| Quick Launch95vs91 | Historical fixture used pre-gap geometry; approved current content40DIP plus external4DIP slot is now explicitly asserted by second.top = row.bottom + Separated.gap. Original rails/containment remain; no test/product modification here. | Complete-name test1/1PASS; full suitePASS. |
| Graphite090909vs181a1f | Current production explicitly selects registered Graphite configs through Kit update/change; current test checks approved#181A1F/#F4F5F7 and all six roles with1e-5 RGB/HSL tolerance. No palette/asset/assertion modification here. Historical color failure not reproduced; do not infer a new color decision or exact prior cause from today's green. | Complete-name real-Kit test1/1PASS; full suitePASS. |
| Clipboard items_after_test_module | Latest owner's set_image_view_mode impl already precedes asset_tests. No business/declaration/test modification here. | Full-dependency strict ClippyPASS, no allow. |
| Editor dependency nonminimal_bool | Still reproduced at serialize.rs215. User separately approved one logically equivalent expression; all16Boolean cases equal. Second explicit approval covers only rustfmt in that push_str fragment and passive-link if. No dependency/version/feature/framework change. | Clippy red101→green0; provenance reverses serializer fragment to originalSHA256 and element whitespace to approved passive baseline. Three exact editor testsPASS. |
| Clipboard UI gate | Owner had already moved source Textarea/rich BlockEditor checks to actual markdown_edit renderer. Existing token-only checks still accepted disconnected mounts/item guard if original token survived in a comment. Added scoped real begin/cancel/save and mounted-body/passive-document checks, retaining every old check. | Mutation regressions initially1PASS/2FAIL(false acceptance), then3/3PASS; all mutations/copies in scratch. No product owner replacement, exclusion or whitelist. |
| Global formatting | All nonvendor files already formatted by their owners. Only two exact vendor fragments remained; no whole-vendor or other-task format sweep. This round's JSON metadata uses LF with original hash values/field order unchanged. | fmt initiallyFAIL(two fragments), final globalfmtPASS. |

Editor revision31608fa74bdbeca79d9901bf2daa48ea7ecf1ecc and all original57file hashes/licenses retained.
Allowlist adds serialize.rs, while reverse-hash checks constrain its ONE expression+format fragment;
element formatting reverses to approved passive baseline2a44a51e00c803b278b9b4c925a2c0dbe0625cf94334c7c16658e9c2836660a8.
The two framework patches stay unchanged. Current contracts/reference/provenance records contain
this explicit amendment; prior task outcomes are historical and unmodified.

## Executed verification (unique logs, append-only commands.jsonl)

Scratch: `C:\Users\Administrator\.pi-desktop\scratch\7c2ebec3-f1a7-4623-9ed1-760f9adb94f5\phase-e-integration-20261022-190316`.
Every Cargo checked shared cargo/rustc lane; jobs1/incremental0/RUSTFLAGS=-C codegen-units=256,
locked/offline/serial. Busy-lane refusals started no check and are not test failures. No --no-deps
substitute, repeated full suite, worker build, C# or codec/native matrix.

- Complete-name --exact tests, each observed1PASS/0FAIL: QuickLaunch polish; Graphite role projection;
  Clipboard queue07 save/focus; stale-save completion; source-save/duplicate/failure draft;
  editor list_continuation_indented; every_block_kind_round_trips; passive focus preservation.
- Clipboard static-gate positive/negative tests3/3PASS; current production UI gatePASS. Scope is
  source ownership, not UIA/native behavior. Actual compiled save/focus tests separatelyPASS.
- **Desktop full-dependency strict Clippy --tests -- -D warnings PASS** (final-full-dependency-clippy.log).
- **Global cargo fmt --all -- --check PASS** (final-global-format.log); workspace --tests compilePASS.
- Clipboard UI/core lifecycle, QuickLaunch lifecycle, UI-state, Desktop-shell gatesPASS. Editor
  provenance/16case equivalence, GPUI release/catalog, spec and scoped diff checksPASS.
- **Exactly ONE full Desktop:1002PASS/0FAIL/0ignored/0filtered,116.79s,exit0**
  (final-full-desktop-once.log).1314source/manifest/JSON hashes unchanged before/after this run.
  This includes existing archive UI tests naturally; no separate Archive28/Protocol65/IPC38/C#11+6
  or archive-worker build was run. Cargo may compile existing dependencies for Desktop; that is not
  a rerun of the independently accepted archive codec/native matrix.

## Preservation / stop

QuickLaunch panel/polish tests, theme/assets, Clipboard business/UI files, Cargo.toml/lock and all
archive/native sources are unchanged from this round's opening baseline. Historical reports remain
hash-identical. Final tracked-change/new-file hashes are saved in final-preservation.json; no commit/push.
The only added behavior checks are source-gate regressions; no archive capability/cache/lease/volume
review or new source cleanup task was introduced. No delegation/review loop.

No remaining workspace integration blocker observed in these checks. Stop at independent acceptance:
real GUI/Shell/physical input/DPI/mixed-monitor/UIA/user acceptance stays **NOT RUN**. EarlierA/B,
JOB07/SAFE08 and other incomplete obligations remain. No PhaseF/P204/P205, packaging/install/system
configuration/source recycle/formal product launch/self-app bypass/commit/push. Overall PhaseE and
first release are **not** declared PASS.
