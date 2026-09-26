# P1 snapshot detail layering

## Goal

Measure 1000/10000 task snapshot/detail payload and refresh costs, then implement only a proven summary/detail split while preserving list sorting, event-gap reconstruction, and selected-task detail behavior.

## Requirements

- Establish a reproducible 1,000-task and 10,000-task fixture for the current snapshot/detail contract.
- Measure serialized snapshot and detail payload bytes, snapshot construction/SQLite query time, and desktop refresh projection time before changing the public shape.
- Narrow the current consumer-facing snapshot only if the measurements show material payload or refresh cost: retain task summary data, queues, aggregate metrics, generation, and sequence; move probe metadata, validators, artifact paths, segments, and timeline access to `TaskDetail`.
- Keep protocol compatibility through optional/default fields or an explicit compatibility path; do not require a second database or transfer engine.
- Preserve list filtering/sorting behavior, event-gap snapshot reconstruction, CLI list/show behavior, and selected-task detail loading.
- Add contract/benchmark tests that make the measured baseline and post-change behavior reproducible.

## Acceptance Criteria

- [x] 1,000 and 10,000 task measurements record serialized bytes, snapshot construction time, SQLite query time, and desktop projection/refresh time before the change.
- [x] The implementation is only narrowed when the measured current shape has a material cost and the post-change fixture demonstrates a meaningful reduction.
- [x] Compact snapshot consumers can rebuild the full visible task list and preserve existing sorting/filtering results.
- [x] Event generation/sequence gap refresh rebuilds the list from compact snapshot data without a cached `task_details` dependency.
- [x] `TaskDetail` remains selected-task-only and returns probe/path/validator/segments/timeline data.
- [x] Legacy JSON/protocol decoding remains compatible, or the compatibility behavior is explicitly tested and documented.
- [x] Relevant Rust, IPC, CLI, desktop tests, formatting, check, and scoped diff gates pass; the nine pre-existing full-run fixture drifts are named in the reference plan.

## Notes

- Keep `prd.md` focused on requirements, constraints, and acceptance criteria.
- Lightweight tasks can remain PRD-only.
- For complex tasks, add `design.md` for technical design and `implement.md` for execution planning before `task.py start`.
