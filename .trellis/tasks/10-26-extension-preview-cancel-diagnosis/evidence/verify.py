"""Read-only Queue03 evidence verification; no Cargo or source changes."""
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
    before = (EVIDENCE / "baselines/rust__apps__nxtools-desktop__src__main_window__extensions_page.rs.baseline").read_bytes()
    after = (ROOT / delta["path"]).read_bytes()
    assert digest(before) == delta["baseline_sha256"]
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
    old, current = before.decode(), after.decode()
    name = "fn w6_manager_pointer_preview_confirm_and_toggle_commit_real_config"
    old_start = old.index("    #[gpui_kit::test]\n    " + name)
    new_start = current.index("    #[gpui_kit::test(iterations = 8)]\n    " + name)
    helper = "    fn wait_for_idle("
    assert old[:old_start] == current[:new_start]
    assert old[old.index(helper, old_start):] == current[current.index(helper, new_start):]
    assert "DEBUG-EXTENSION-CANCEL" not in current
    for group in ["protected-checks.json", "historical-references.json"]:
        for row in load(EVIDENCE / group):
            assert digest((ROOT / row["path"]).read_bytes()) == row["sha256"], row["path"]
    print(f"PASS: {len(manifest['evidence'])} evidence hashes, reversible test-only delta, production prefix and original helper, seven protected files and frozen history")
    print("Historic full-run timing attribution remains unresolved; external strict Clippy failure is retained.")


if __name__ == "__main__":
    main()
