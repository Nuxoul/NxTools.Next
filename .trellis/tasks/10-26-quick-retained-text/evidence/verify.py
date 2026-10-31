"""Read-only audit13 evidence/live-delta verifier; no product or native acceptance."""
from pathlib import Path
import base64
import hashlib
import json

TASK = Path(__file__).resolve().parents[1]
ROOT = TASK.parents[2]
EVIDENCE = TASK / "evidence"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def load(name):
    return json.loads((EVIDENCE / name).read_text(encoding="utf-8-sig"))


def check(path, digest):
    assert path.is_file(), f"Missing file: {path}"
    assert sha(path.read_bytes()) == digest, f"Byte movement: {path}"


manifest = load("delivery-manifest.json")
for entry in manifest["files"]:
    check(TASK / entry["path"], entry["sha256"])
base = {item["path"]: item for item in load("baseline-manifest.json")["files"]}
for entry in base.values():
    check(TASK / entry["snapshot"], entry["sha256"])
deltas = load("own-deltas.json")
for entry in deltas:
    current = TASK / entry["after_snapshot"]
    check(current, entry["after_sha256"])
    check(ROOT / entry["path"], entry["after_sha256"])
    if entry.get("new_file"):
        assert entry["path"] not in base
        continue
    before = (TASK / base[entry["path"]]["snapshot"]).read_bytes()
    after = current.read_bytes()
    assert sha(before) == entry["before_sha256"]
    forward = before.splitlines(keepends=True)
    inverse = after.splitlines(keepends=True)
    for operation in reversed(entry["ops"]):
        old = base64.b64decode(operation["old"])
        new = base64.b64decode(operation["new"])
        assert b"".join(forward[operation["old_start"]:operation["old_end"]]) == old
        assert b"".join(inverse[operation["new_start"]:operation["new_end"]]) == new
        forward[operation["old_start"]:operation["old_end"]] = new.splitlines(keepends=True)
        inverse[operation["new_start"]:operation["new_end"]] = old.splitlines(keepends=True)
    assert b"".join(forward) == after, entry["path"]
    assert b"".join(inverse) == before, entry["path"]
protected = load("protected-manifest.json")
for entry in protected:
    check(ROOT / entry["path"], entry["sha256"])
history = load("historical-manifest.json")["files"]
for entry in history:
    check(ROOT / entry["path"], entry["sha256"])
checks = load("owner-checks.json")
assert all(entry["result"] == "PASS" for entry in checks)
summary = load("verification-summary.json")
assert summary["final_tests"] == 137
assert summary["status"] == "IMPLEMENTED_BOUNDED_VERIFICATION_WITH_BASELINE_QUALITY_BLOCKERS"
print(f"PASS audit13 integrity: {len(manifest['files'])} artifacts, {len(deltas)} deltas, "
      f"{len(protected)} protected files, {len(history)} historical files, {len(checks)} owner checks")
print("137 selected tests passed; test-strict/check/scoped-fmt/diff passed. "
      "Production strict, workspace fmt and QuickLaunch source gate retain baseline failures.")
print("NOT full Desktop/native/DPI/IME/reader/visual/user/release acceptance; no global13 closure.")
