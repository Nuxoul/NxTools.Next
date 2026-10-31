"""Read-only source-gate delivery verifier; no native/product operation."""
from pathlib import Path
import base64
import hashlib
import json
import re

TASK = Path(__file__).resolve().parents[1]
ROOT = TASK.parents[2]
EVIDENCE = TASK / "evidence"


def load(name):
    return json.loads((EVIDENCE / name).read_text(encoding="utf-8-sig"))


def sha(data):
    return hashlib.sha256(data).hexdigest()


def check(path, digest):
    assert path.is_file(), f"Missing: {path}"
    assert sha(path.read_bytes()) == digest, f"Byte movement: {path}"


manifest = load("delivery-manifest.json")
for record in manifest["files"]:
    check(TASK / record["path"], record["sha256"])
baseline = {record["path"]: record for record in load("baseline-manifest.json")["files"]}
for record in baseline.values():
    check(TASK / record["snapshot"], record["sha256"])
deltas = load("own-deltas.json")
for record in deltas:
    snapshot = TASK / record["after_snapshot"]
    check(snapshot, record["after_sha256"])
    check(ROOT / record["path"], record["after_sha256"])
    if record.get("new_file"):
        assert record["path"] not in baseline
        continue
    before = (TASK / baseline[record["path"]]["snapshot"]).read_bytes()
    after = snapshot.read_bytes()
    assert sha(before) == record["before_sha256"]
    forward = before.splitlines(keepends=True)
    inverse = after.splitlines(keepends=True)
    for operation in reversed(record["ops"]):
        old = base64.b64decode(operation["old"])
        new = base64.b64decode(operation["new"])
        assert b"".join(forward[operation["old_start"]:operation["old_end"]]) == old
        assert b"".join(inverse[operation["new_start"]:operation["new_end"]]) == new
        forward[operation["old_start"]:operation["old_end"]] = new.splitlines(keepends=True)
        inverse[operation["new_start"]:operation["new_end"]] = old.splitlines(keepends=True)
    assert b"".join(forward) == after
    assert b"".join(inverse) == before
protected = load("protected-manifest.json")
for record in protected:
    check(ROOT / record["path"], record["sha256"])
history = load("historical-manifest.json")["files"]
for record in history:
    check(ROOT / record["path"], record["sha256"])
owned = deltas[0]
old_gate = (TASK / baseline[owned["path"]]["snapshot"]).read_text(encoding="utf-8")
new_gate = (TASK / owned["after_snapshot"]).read_text(encoding="utf-8")
marker = "if ($protocolSource -notmatch 'ElevatedDirectoryHelperRequested'"


def predicates(text):
    region = marker + text.split(marker, 1)[1].split("    throw @'", 1)[0]
    return re.findall(r"\$(\w+)\s+(-notmatch|-match)\s+'([^']*)'", region)


previous = predicates(old_gate)
previous.remove(("buildScriptSource", "-notmatch", r"write-release-binary-manifest\.ps1"))
assert previous == predicates(new_gate) and len(previous) == 73
summary = load("verification-summary.json")
assert summary["source_only_assertions"] == 100
assert summary["chain_mutations_rejected"] == 26
assert summary["original_compound_predicates"] == 74 and summary["original_violations"] == 1
assert [record["name"] for record in summary["related_checks"] if record["exit"] != 0] == ["quick-full-source-final"]
assert all(record["result"] == "PASS" for record in load("owner-checks.json"))
print(f"PASS source-gate integrity: {len(manifest['files'])} artifacts, {len(deltas)} deltas, "
      f"{len(protected)} protected files, {len(history)} historical files; all73original security predicates preserved.")
print("Target SourceOnly100PASS; whole QuickLaunch source gate still FAIL at preexisting QL-11 line458.")
print("No product/security-implementation mutation or native/release acceptance; no full Desktop/commit.")
