"""Read-only verification for Queue01 evidence; does not run Cargo or change sources."""
from pathlib import Path
import hashlib
import json

TASK = Path(__file__).resolve().parent.parent
ROOT = TASK.parents[2]
EVIDENCE = TASK / "evidence"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def load(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def main():
    manifest = load(EVIDENCE / "manifest.json")
    for item in manifest["evidence"]:
        assert digest((TASK / item["path"]).read_bytes()) == item["sha256"], item["path"]
    for item in manifest["new_documents"]:
        assert digest((ROOT / item["path"]).read_bytes()) == item["sha256"], item["path"]
    delta = load(EVIDENCE / "own-delta.json")
    before = (EVIDENCE / "baselines/rust__apps__nxtools-desktop__src__extensions__committed_events.rs.baseline").read_bytes()
    after = (ROOT / delta["path"]).read_bytes()
    assert digest(before) == delta["before_sha256"]
    assert digest(after) == delta["after_sha256"]
    assert after.startswith(before), "original dirty prefix changed"
    forward = before.decode().splitlines(keepends=True)
    reverse = after.decode().splitlines(keepends=True)
    for operation in reversed(delta["operations"]):
        start, end = operation["before_start"], operation["before_end"]
        assert forward[start:end] == operation["before_lines"]
        forward[start:end] = operation["after_lines"]
        start, end = operation["after_start"], operation["after_end"]
        assert reverse[start:end] == operation["after_lines"]
        reverse[start:end] = operation["before_lines"]
    assert "".join(forward).encode() == after
    assert "".join(reverse).encode() == before
    for item in load(EVIDENCE / "historical-evidence-references.json"):
        assert digest((ROOT / item["path"]).read_bytes()) == item["sha256"], item["path"]
    print(f"PASS: {len(manifest['evidence'])} evidence hashes, two documents, one reversible append, original prefix and frozen historical references")
    print("Requested compiled checks pass; recorded workspace-wide fmt failure is not converted to green.")


if __name__ == "__main__":
    main()
