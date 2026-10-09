# Download structure

Evidence: `repo:rust/crates/nxtools-download/src/lib.rs#TransferPlan`, `repo:rust/crates/nxtools-download/src/engine.rs#DownloadEngine`, `repo:rust/crates/nxtools-download/src/message.rs`, `repo:rust/crates/nxtools-download/src/engine/segmented_runner.rs`.

lib defines domain request/task/segment/plan; message owns command/event/fault projection. engine coordinates admission/queue/settings and worker/store state. engine submodules own segmented/HLS/DASH runners; http owns network validation/streaming; artifact/capacity own disk publication; storage owns SQLite; media/ffmpeg_install own bounded child/explicit third-party install; organization/request_policy/rate_limit own reusable rules. GPUI/windows/chrome stay outside library. Don't split by file size without real seam/test benefit or leak raw requests into logs. [Architecture](../../shared/architecture.md).
