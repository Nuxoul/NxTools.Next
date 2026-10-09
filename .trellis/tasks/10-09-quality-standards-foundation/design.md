# Standards organization and evidence design

## Discovery before writing
Read root/package manifests, existing AGENTS.md, code/test owners, reference indexes, quality scripts and current specs. Record owner -> source -> tests -> contract -> check mapping in child research notes. Use no fabricated package/layer. Native C# and extension routing can use focused spec topics without changing Trellis registry/runtime.

## Authority and migration
Build one index for engineering/UI navigation, links to package/layer topics and references. Preserve historical plans; add current-status/authority links rather than deleting evidence. Shared repeated rules have one home and short local links. Normative design derives from the later approved staged design, with tested native/content constraints and narrow exceptions documented; observed constants are labeled current. Product mismatches become Foundation backlog, not edits in this child.

## Contracts and checks
Concrete interfaces, IDs/generation, config ownership, return states, native error owners and memory environment accompany the seven-section cross-layer format. Guidance includes observable test assertions, not only principles. Spec/static checker is small, deterministic and bounded; detects stale links/placeholders/contradictions where feasible, tests bad fixture behavior, does not infer dynamic native correctness from regex. Existing architecture gates continue guarding actual production boundaries.

## Scope and risks
A new rule unsupported by current implementation needs explicit target/gap, not a fake claim that repo already follows it. Avoid duplicating copy/paste blocks across twelve packages. No whole-file product rewrite, runtime/vendor patch or automatic CI/release changes. First-party specs complete only after source verification; runtime/native visual release status remains separately pending.