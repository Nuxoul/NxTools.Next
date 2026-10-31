"""Read-only verification of denied final-stage admission; never runs product/tests."""
from pathlib import Path
import hashlib
import json

TASK = Path(__file__).resolve().parents[1]
ROOT = TASK.parents[2]
EVIDENCE = TASK / "evidence"


def load(name):
    return json.loads((EVIDENCE / name).read_text(encoding="utf-8-sig"))


def check(path, digest):
    assert path.is_file(), f"Missing: {path}"
    assert hashlib.sha256(path.read_bytes()).hexdigest() == digest, f"Byte movement: {path}"


manifest = load("delivery-manifest.json")
for entry in manifest["files"]:
    check(TASK / entry["path"], entry["sha256"])
source = load("source-baseline.json")
assert source["movement_since_latest_queue16_delivery"] == []
for entry in source["files"]:
    check(ROOT / entry["path"], entry["sha256"])
history = load("historical-protection.json")["files"]
for entry in history:
    check(ROOT / entry["path"], entry["sha256"])
logs = load("failure-log-hashes.json")
for entry in logs:
    check(ROOT / entry["path"], entry["sha256"])
    assert entry["evidence_level"] == "HISTORICAL_NOT_RERUN"
historical = next(entry for entry in logs if entry["path"].endswith("desktop-full-once.log"))
assert historical["encoding"] == "utf-16"
assert any("FAILED. 1237 passed; 1 failed; 0 ignored" in line for line in historical["signals"])
result = load("admission-result.json")
assert result["status"] == "NOT_ADMITTED_INCOMPLETE_PREREQUISITE_REMEDIATION"
assert result["final_stage_rust_invocations"] == 0
assert result["final_stage_full_desktop_attempts"] == 0
for key in ["full_desktop_passed", "full_desktop_failed", "full_desktop_ignored", "full_desktop_exit"]:
    assert result[key] is None
assert all(entry["exit"] == 0 for entry in load("command-results.json"))
print(f"PASS admission evidence integrity: {len(manifest['files'])} artifacts; "
      f"{len(source['files'])} source files, {len(history)} historical files, {len(logs)} original log records unchanged.")
print("ADMISSION DENIED: Queue08 partial; Queue09/10 no implementation; original #7 not repaired.")
print("Final-stage Rust invocations0 / complete Desktop attempts0; allowance unconsumed, counts/exit NOT RUN.")
print("Historical1237PASS/1FAIL/0ignored remains FAIL; documentation checks do not certify product verification.")
