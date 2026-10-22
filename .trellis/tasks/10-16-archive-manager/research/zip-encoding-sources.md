# ZIP filename candidates — verified sources and boundary

2026-10-20, B1 (ENG05 / SAFE01–02), not Phase A completion.

Reference-first inspection: legacy `D:\Code\NxTools` tracked ZipArchive/SevenZip/SharpCompress search still locates only BrowserExtensionPackageService package ZIP, not a general archive browser. Archiver `core/src/lib.rs::list` (494–514) uses ZipArchive/by_index_raw/name; SupaZip `supazip/supazip-core/src/formats/zip.rs` (149–177) uses by_index/name. Neither supplies Auto filename candidates. Bandizip public codepage/manual/auto resources are evidence of choices, not a detection algorithm; no proprietary inspection.

Existing Next boundary: `encoding::decode` first validates UTF-8 flag / CRC-bound Unicode Path extra, then explicit pages; ZIP central-directory enumeration retains raw bytes and ordinal, admits combined metadata before decoding, annotates decoded paths, and ExtractionPlan refuses unsafe/colliding outputs. Existing encoding_rs 0.8.35 strict replacement-free decoding is reused. No dependency change.

Smallest change: bounded per-name UTF-8/GBK/Shift-JIS heuristic, with short or competing candidates remaining UncertainCp437/manual. A unique script-supported candidate is Heuristic(codepage), not authoritative Unicode or a numerical confidence. Keep heuristic names marked UncertainEncoding because language correctness is not guaranteed. No whole-archive detection, global voting, language framework or locale assumption. Unicode authority remains ahead of explicit pages and Auto.

Reprojection rereads the original ZIP directory on the worker/calling background thread, checks the original source revision before/after, preserves session + raw ordinal, and reuses original path annotation / ExtractionPlan. Listing retains dangerous names; planning refuses before any output call. No UI/layout/focus/show-hide change, no new native lifecycle or cancellation channel. Existing operation correlation and accepted/start/terminal diagnostics remain owners; no name/byte/decision-per-entry logs added. Cancel and finite metadata admission remain at the existing directory loop.

Test seams are the user-requested public engine, actual headless product open over typed IPC, and original runner/worker/native extraction with independent Python raw ZIP fixtures. C4 manual pages use literal bytes, not a full cross-process language matrix.
