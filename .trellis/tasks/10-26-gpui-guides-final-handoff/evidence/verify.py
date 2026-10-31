"""Read-only final-report integrity verifier. Never runs product, native or tests."""
from pathlib import Path
import hashlib
import json
import re

TASK = Path(__file__).resolve().parents[1]
ROOT = TASK.parents[2]
EV = TASK / "evidence"


def load(name):
    return json.loads((EV / name).read_text(encoding="utf-8-sig"))


def check(entry):
    path = ROOT / entry["path"]
    assert path.is_file(), f"Missing: {path}"
    assert hashlib.sha256(path.read_bytes()).hexdigest() == entry["sha256"], f"Byte movement: {path}"


manifest = load("delivery-manifest.json")
for entry in manifest["files"]:
    check(entry)
base = load("workspace-baseline.json")["files"]
for entry in base:
    check(entry)
index = load("source-index.json")
indexed = 0
for task in index["tasks"]:
    entries = task["documents"] + task["actual_changed_files_current_hashes"]
    if task["delta_index"]:
        entries += [task["delta_index"]]
    for entry in entries:
        check(entry)
        indexed += 1
for entry in index["historical_command_logs"]:
    check(entry)
    indexed += 1
assert index["historical_parent_accepted_findings"] == [1, 2, 3, 4, 5, 9, 10]
assert index["open_findings"] == [6, 7] and index["final_integrated_attempts"] == 0
final = load("checks-final.json")
assert final["status"] == "PASS_REPORT_ONLY" and all(row["passed"] for row in final["commands"])
for row in final["commands"]:
    check({"path": row["log"], "sha256": row["sha256"]})
assert load("checks.json")["status"] == "FAIL", "Initial harness-expectation failure must stay frozen"
semantics = load("git-no-index-semantics.json")
assert semantics[0]["exit"] == 1 and not semantics[0]["output"]
assert semantics[1]["exit"] == 3 and "trailing whitespace" in semantics[1]["output"]
structure = load("document-structure-final.json")
for report in structure["reports_current"]:
    check(report)
for link in structure["links"]:
    value = link["target"].strip("<>")
    match = re.search(r":(\d+)(?::(\d+))?$", value)
    path = Path(value[:match.start()] if match else value)
    assert path.is_file(), link
    if match:
        assert int(match.group(1)) <= len(path.read_text(encoding="utf-8-sig").splitlines()), link
last = load("handoff-final-checks.json")
assert last["spec_contracts_exit"] == 0 and last["document_scope_exit"] == 0
print(f"PASS final report integrity: {len(manifest['files'])} delivery files, {len(base)} original Git-visible files unchanged, "
      f"{indexed} indexed records and {len(structure['links'])} local report links verified.")
print("Seven historical bounded parent acceptances; four later bounded implementations; #6 partial / #7 unrepaired.")
print("Final convergence NOT ADMITTED / full Desktop attempts0; physical/native/DPI/reader/visual/user/release NOT RUN.")
print("Documentation handoff only; old failure logs/manifests preserved, no product mutation or automatic follow-up.")
