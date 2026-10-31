"""Read-only evidence verification for Queue02; no Cargo or source writes."""
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
    for row in manifest["evidence"]:
        assert digest((TASK / row["path"]).read_bytes()) == row["sha256"], row["path"]
    for row in manifest["documents"]:
        assert digest((ROOT / row["path"]).read_bytes()) == row["sha256"], row["path"]
    delta = load(EVIDENCE / "own-delta.json")
    before = (EVIDENCE / "baselines/rust__apps__nxtools-desktop__src__main_window__archive_page_tests.rs.baseline").read_bytes()
    after = (ROOT / delta["path"]).read_bytes()
    assert digest(before) == delta["before_sha256"]
    assert digest(after) == delta["after_sha256"]
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
    original, current = before.decode(), after.decode()
    old_start = original.index("#[gpui_kit::test]\nfn archive_settings_official_search_never_saves")
    new_start = current.index("#[gpui_kit::test(iterations = 8)]\nfn archive_settings_official_search_never_saves")
    end = "\n#[gpui_kit::test]\nfn archive_settings_exclusion_editor_is_lossless_and_stream_levels_are_valid"
    assert original[:old_start] == current[:new_start]
    assert original[original.index(end, old_start):] == current[current.index(end, new_start):]
    assert "DEBUG-ARCHIVE-SEARCH" not in current
    for name in ["protected-checks.json", "historical-references.json"]:
        for row in load(EVIDENCE / name):
            assert digest((ROOT / row["path"]).read_bytes()) == row["sha256"], row["path"]
    print(f"PASS: {len(manifest['evidence'])} evidence hashes, reversible test-only delta, unchanged outside-test bytes, protected sources and historical references")
    print("No complete Desktop/native/visual acceptance is implied.")


if __name__ == "__main__":
    main()
