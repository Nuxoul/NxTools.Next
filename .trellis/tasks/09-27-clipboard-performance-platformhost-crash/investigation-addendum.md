# Clipboard and PlatformHost Runtime Reliability Investigation Addendum

## Scope Addendum

This existing task is reused under parent `09-30-desktop-runtime-reliability-investigation`. Preserve the already completed cursor pagination, bounded history window, PlatformHost boundary hardening, and PixPin/browser image canonicalization. This addendum covers the newly reported duplicate `NxTools.PlatformHost.exe` processes, long-session CPU/private-memory growth, and intermittent missed clipboard captures.

## New Requirements

- Inventory every observed PlatformHost by PID, full executable path, SHA-256/approved runtime hash, parent PID, command line, session ID, start time, and relationship to the desktop or installed service. Distinguish duplicate desktop owners, packaged/protected/development copies, supervisor restarts, manual launches, orphans, and unrelated lookalikes.
- Sample per-process CPU delta, private bytes/commit, working set, thread/handle counts, process start/exit, and supervisor restart count during fixed idle, normal-use, and controlled clipboard-copy intervals. Do not equate working set with a memory leak.
- Trace the clipboard transaction from Windows sequence number/`WM_CLIPBOARDUPDATE` through native capture/retry/limits, image rewrite, PlatformHost publication tail, bounded event channel, pipe write/read, Rust worker/storage commit/deduplication, and GPUI refresh.
- Record one payload-free outcome per controlled copy attempt with sequence/correlation, content-kind booleans, native format count, byte/pixel counts, decision reason, retry/drop/IPC/storage/UI outcomes, elapsed time, and native error codes. Never record clipboard text, paths, window titles, credentials, or raw payloads.
- Cover Unicode text, file copy/`CF_HDROP`, PixPin screenshot, and browser right-click `Copy image`; vary clipboard rate/load only after a baseline is captured.
- Identify the first missing transition for each observed miss and build/run a red-capable test/replay at that boundary. Do not change retry/filter/queue policy until the evidence-backed cause and proposed scope are approved by the user.

## Acceptance Addendum

- [ ] Process inventory accounts for every PlatformHost observed in the test session using path/hash/parent/session evidence; no process was killed by name alone.
- [ ] CPU/private-bytes/working-set samples show whether resource growth reproduces and which exact process/workload owns it.
- [ ] Each controlled copy attempt has a payload-free end-to-end ledger and any miss is localized to a named boundary or documented evidence gap.
- [ ] The existing interactive performance soak is run or its exact blocker is documented; old benchmark conclusions are not substituted for live process evidence.
- [ ] PixPin/browser canonicalization and safe native formats/`CF_HDROP` compatibility remain covered.
- [ ] Findings and proposed fixes are reviewed with the user before any further product behavior changes.
