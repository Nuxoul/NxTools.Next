# Download disk backpressure

## Goal
A disk slower than the network must not cause healthy HTTP connections to be retired as slow network paths.

## Requirements
Preserve the existing planner, bounded receive memory, real written-byte progress and sync-before-checkpoint publication. Distinguish disk waits from network waits. Retain real slow-network retirement, task/global/runtime limiter exclusions and pause/cancel writer draining. Do not adopt Mister's large per-task buffer or indefinite checkpoint deferral.

## Acceptance
After `11-13-download-web-page`, a red-capable real writer/HTTP test controls disk latency, verifies no disk-induced slow retirement and exact output, then verifies real slow connections remain replaceable. Paused/reopened task preserves durable prefixes; no unbounded worker, buffer or diagnostic stream is introduced.
