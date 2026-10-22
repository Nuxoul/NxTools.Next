# Bounded resource source-boundary verification

Date:2026-10-19. Source verification only,not runtime/Phase A acceptance.

Re-read legacy tracked archive grep and Archiver core Entry/Progress/ProgReader/list/test and
bench/gen_data.py:no old general archive lifecycle found;reference synchronous Vec listing and
1MiB progress callback do not establish bounded100k/native cleanup. Existing reference note
remains [archive.md](../../../../docs/reference-code/archive.md). No Bandizip disassembly.

Next actual chain:ArchiveReadSession.Dispatch -> ArchiveWorkerRunner.Start -> ExecuteAsync
suspended process/Job -> worker::run -> engine.open/full-source fingerprint -> ZIP list/extract.
Existing20ms runner poll/200ms grace/final Job+process+pumps drain precede Source/helper disposal
and slotrelease. Source-acquisition callback owns native lease; engine metadata/model budgets
and runner process reservation/Jobcap are independent of observed RSS/PrivateBytes.

Worker failure Cancelled after runner timeout reaches OnFrameAsync before OnRunnerEvent(exited).
The original timeout must outrank this cleanup consequence,not become user Cancelled.
Red-capable real session/runner fixture TimeoutIsNotUserCancellation reproduces that ordering;
current unchanged wire uses failed/io_failure because no timeout enum exists. Cleanupfalse wins.
No native show/hide/focus/layout exists in this headless slice.

Finite ceilings:engine1Mentries/256MiBmetadata/512MiBwork;privateworker validates1M/512MiB/1GiB;
C#work>=64MiB and work+metadata+64MiB<=1GiB;stdout cumulative8MiB/no escalation. Product4096/
4MiBmetadata/4MiBretention is a narrower wire contract,not silently increased to engine defaults.
Legacy schema1frames omit snapshotID;direct worker cannot claim nativeJob/sourcelease safety.

Actual resource results/limits/cleanup/defect history:
[resource evidence](../resource-acceptance-evidence.md). This inventory is not acceptance.
