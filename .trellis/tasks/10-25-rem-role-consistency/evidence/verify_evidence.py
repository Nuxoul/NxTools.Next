"""Read-only audit06 evidence verification; run from any working directory."""
from pathlib import Path
import hashlib
import json

TASK = Path(__file__).resolve().parent.parent
ROOT = TASK.parents[2]
EVIDENCE = TASK / "evidence"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def load(name):
    return json.loads((EVIDENCE / name).read_text(encoding="utf-8-sig"))


def main():
    manifest = load("delivery-manifest.json")
    for row in manifest["evidence"]:
        path = TASK / row["path"]
        assert sha(path.read_bytes()) == row["sha256"], row["path"]
    for row in manifest["new_documents"]:
        assert sha((ROOT / row["path"]).read_bytes()) == row["sha256"], row["path"]
    for row in manifest["captured_sources"]:
        before = (TASK / row["baseline"]).read_bytes()
        after = (TASK / row["current"]).read_bytes()
        assert sha(before) == row["before_sha256"], row["path"]
        assert sha(after) == row["after_sha256"], row["path"]
        assert (ROOT / row["path"]).read_bytes() == after, row["path"]
    for row in load("own-deltas.json"):
        before = (TASK / row["baseline"]).read_text(encoding="utf8").splitlines(keepends=True)
        after = (TASK / row["current"]).read_text(encoding="utf8").splitlines(keepends=True)
        forward = before.copy()
        for operation in reversed(row["operations"]):
            start, end = operation["before_start"], operation["before_end"]
            assert forward[start:end] == operation["before_lines"]
            forward[start:end] = operation["after_lines"]
        reverse = after.copy()
        for operation in reversed(row["operations"]):
            start, end = operation["after_start"], operation["after_end"]
            assert reverse[start:end] == operation["after_lines"]
            reverse[start:end] = operation["before_lines"]
        assert sha("".join(forward).encode()) == row["after_sha256"], row["path"]
        assert sha("".join(reverse).encode()) == row["before_sha256"], row["path"]
    for group in ["protected", "frozen_history"]:
        for row in manifest[group]:
            assert sha((ROOT / row["path"]).read_bytes()) == row["sha256"], row["path"]
    for row in load("owner-invariants.json"):
        if "whitespace_normalized_equal" in row:
            assert row["whitespace_normalized_equal"], row
        if "old_prefix_equal" in row:
            assert row["old_prefix_equal"], row
    print(
        "PASS audit06 delivery:", len(load("own-deltas.json")), "reversible dirty deltas,",
        len(manifest["evidence"]), "evidence files,", len(manifest["protected"]),
        "protected files,", len(manifest["frozen_history"]), "frozen history files"
    )
    print("Full Desktop remains FAIL:1237 passed/1 failed; no native/DPI/visual acceptance.")


if __name__ == "__main__":
    main()
