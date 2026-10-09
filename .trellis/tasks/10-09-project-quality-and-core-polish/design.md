# Task ownership and quality design

## Authority
AGENTS.md and verified reference behavior govern architecture/migration/native/security boundaries. `.trellis/spec/` owns executable engineering/UI guidance, with indexes routing by actual package/layer. Task PRD owns intentional behavior changes; design owns decisions, implement owns order/evidence. Older completed UI remediation entries remain historical; the later explicitly approved staged design is the target when they conflict. Current code is evidence, not a silent override.

## Ownership map
| Deliverable | Owner / relationship |
| --- | --- |
| Overall requirements, sequencing, integration | This parent |
| Template/source inventory, coding/UI authority, checks | New child `10-09-quality-standards-foundation`; related `00-bootstrap-guidelines` stays intact |
| Existing review failures | Existing `10-08-sol-engine-remediation`; native/live boundaries stay in corresponding SOL tasks |
| Foundation and five UI stages | Existing `10-02-staged-desktop-ui` under its original parent; no reparenting/duplicate implementation |
| Link Detection behavior/presentation | Existing `10-02-link-detector-panel`, reached within Clipboard + Tools; original parent retained |
| Adjacent tool recommendation only | New child `10-09-adjacent-utilities-evaluation` |

## Standards shape
Discover actual conventions through modules/tests/build scripts and reference notes. Shared repeated contracts belong in one navigable common document; package topics link to them and add concrete local interfaces/examples, not copies. Document C# native/extension boundaries using actual code ownership; do not fabricate a package registry or alter Trellis runtime to force a layout. Label current/required/proposed and concrete tracked gaps. Preserve every recent seven-section executable contract; merge only with traceable replacement.

UI specifies semantics and interaction, not arbitrary new constants. Build a conflict matrix for controls/radii/spacing/type/colors; resolve approved-target precedence, content budgets and narrow named exceptions before changing component consumers. No global search/replace of pixels. Present native effects in the existing Rust state/layout -> typed IPC -> C# Windows API -> WM_SIZE/result -> GPUI rendering flow. Cross-entity effects wait until leases end.

## Scheduling and failure handling
One coordinated Cargo process with the AGENTS.md memory environment. Source/document inspection can proceed independently; no duplicate repair of an existing owner's files. Bound investigation of known test failures; if an in-scope cause cannot be proved, report rather than repeatedly running until green. New UI implementation waits for its stage's updated baseline/scope and user acceptance of the prior visual stage. The AI queue only records prompts; confirmation starts execution, and failures pause rather than advancing silently.

## Compatibility / privacy / rollback
Keep user defaults, configuration migrations, passive preview, native activation and cancellation semantics. Only synthetic fixtures in checks; no payload secrets. Keep pending live gates honest without turning every deferred release gate into a blocker for documentation. Rollback uses only own scoped changes; no reset, branch switch or wholesale overwrite of dirty files. Native/service/package actions need separate approval.