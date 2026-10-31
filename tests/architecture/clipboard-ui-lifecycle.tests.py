#!/usr/bin/env python3
"""Source-gate mutation regressions; copies only inspected inputs into session scratch."""
from pathlib import Path
import os
import re
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
GATE = Path("tests/architecture/clipboard-ui-lifecycle.ps1")


class ClipboardLifecycleGateTests(unittest.TestCase):
    def setUp(self):
        scratch = os.environ.get("PI_SCRATCH_DIR")
        self.assertTrue(scratch and Path(scratch).is_dir(), "session scratch required")
        self.temporary = tempfile.TemporaryDirectory(prefix="clipboard-gate-", dir=scratch)
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        source = (ROOT / GATE).read_text("utf-8-sig")
        paths = {GATE}
        paths.update(Path(path.replace("\\", "/")) for path in re.findall(
            r"Join-Path \$root '([^']+)'", source))
        for path in paths:
            target = self.root / path
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / path, target)

    def run_gate(self):
        return subprocess.run(
            ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(self.root / GATE)],
            capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=30)

    def disconnect(self, relative, live, replacement):
        path = self.root / relative
        source = path.read_text("utf-8")
        self.assertEqual(source.count(live), 1, "unique production seam")
        path.write_text(source.replace(live, replacement) + "\n// " + live + "\n", encoding="utf-8")

    def test_current_production_wiring_passes(self):
        result = self.run_gate()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_rich_mount_cannot_be_replaced_by_a_comment(self):
        self.disconnect("rust/apps/nxtools-desktop/src/ui/markdown_edit.rs",
                        "BlockEditor::new(&self.edit_rich)", "BlockEditor::new(&self.unrelated_editor)")
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, "detached rich editor passed token-only gate")

    def test_save_identity_guard_cannot_be_replaced_by_a_comment(self):
        self.disconnect("rust/apps/nxtools-desktop/src/ui/mod.rs",
                        "panel.editing_item_id != Some(item_id)", "false")
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, "missing item identity passed token-only gate")

    def test_row_focus_identity_cannot_be_replaced_by_index_or_comment(self):
        self.disconnect("rust/apps/nxtools-desktop/src/ui/history.rs",
                        '.use_keyed_state(("history-row-focus", self.entry.id as u64)',
                        '.use_keyed_state(("history-row-focus", self.index as u64)')
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, "index retained focus passed gate")

    def test_removed_focus_scope_guard_cannot_be_replaced_by_a_comment(self):
        self.disconnect("rust/apps/nxtools-desktop/src/ui/mod.rs",
                        'window.focus_lost_restore_target(cx).as_ref() == Some(&panel.focus_scope)',
                        'true')
        result = self.run_gate()
        self.assertNotEqual(result.returncode, 0, "cross-panel fallback passed gate")


if __name__ == "__main__":
    unittest.main()
