#!/usr/bin/env python3
"""Synthetic spec-checker tests; all fixture writes stay in session scratch."""
from __future__ import annotations

import os
import runpy
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

CHECKER = Path(__file__).with_name("spec-contracts.py").resolve()
API = runpy.run_path(str(CHECKER), run_name="spec_contract_library")


class SpecContractTests(unittest.TestCase):
    def setUp(self) -> None:
        scratch = os.environ.get("PI_SCRATCH_DIR")
        if not scratch or not Path(scratch).is_dir():
            self.fail("PI_SCRATCH_DIR must name the existing session scratch directory")
        self.temporary = tempfile.TemporaryDirectory(prefix="spec-contracts-", dir=scratch)
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.spec = self.root / ".trellis/spec"
        self.write("Cargo.toml", '[workspace]\nmembers = ["rust/example"]\n')
        self.write("rust/example/Cargo.toml", '[package]\nname = "nxtools-fixture"\n')
        self.write("rust/example/src/lib.rs", "// synthetic source evidence\n")
        self.evidence = "Evidence: `repo:rust/example/src/lib.rs#Synthetic`.\n"
        owner = ".trellis/spec/nxtools-fixture/backend/"
        topic_links = []
        for topic in API["TOPICS"]:
            self.write(owner + topic, "# Synthetic topic\n\n" + self.evidence)
            topic_links.append(f"[{topic}]({topic})")
        self.write(owner + "index.md", "# Owner\n\n## Pre-Development Checklist\n" +
                   "\n".join(topic_links) + "\n\n## Quality Check\nSynthetic checks.\n")
        root_links = ["[Owner](nxtools-fixture/backend/index.md)"]
        for topic in ("architecture", "engineering", "verification", "foundation-backlog"):
            self.write(f".trellis/spec/shared/{topic}.md", "# Synthetic shared\n\n" + self.evidence)
            root_links.append(f"[{topic}](shared/{topic}.md)")
        self.write(".trellis/spec/shared/ui-design.md", "# Synthetic UI\n\n" + self.evidence +
                   "Approved target / Current evidence / Exception: #18181b #0ea5e9 24/28/32 3/6/8\n")
        root_links.append("[UI](shared/ui-design.md)")
        for folder in ("platformhost", "browser-extension"):
            self.write(f".trellis/spec/{folder}/index.md", "# Synthetic native/web owner\n")
            root_links.append(f"[{folder}]({folder}/index.md)")
        self.write(".trellis/spec/index.md", "# Standards\n\n" + "\n".join(root_links) + "\n")

    def write(self, relative: str, content: str) -> None:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def errors(self) -> list[str]:
        return API["check"](self.root)[0]

    def test_valid_fixture_and_read_only_cli(self) -> None:
        before = {p.relative_to(self.root): p.read_bytes() for p in self.root.rglob("*") if p.is_file()}
        self.assertEqual(self.errors(), [])
        result = subprocess.run([sys.executable, str(CHECKER), "--root", str(self.root)],
                                capture_output=True, text=True, timeout=15, check=False)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("1 workspace owners", result.stdout)
        after = {p.relative_to(self.root): p.read_bytes() for p in self.root.rglob("*") if p.is_file()}
        self.assertEqual(before, after)

    def test_template_and_broken_link_fail_cli(self) -> None:
        self.write(".trellis/spec/shared/architecture.md", "# Bad\n\n" + self.evidence +
                   "(To be filled by the team)\n[Missing](absent.md)\n")
        result = subprocess.run([sys.executable, str(CHECKER), "--root", str(self.root)],
                                capture_output=True, text=True, timeout=15, check=False)
        self.assertEqual(result.returncode, 1)
        self.assertIn("template residue", result.stdout)
        self.assertIn("broken local link", result.stdout)

    def test_missing_evidence_file_and_missing_evidence(self) -> None:
        self.write(".trellis/spec/shared/architecture.md", "# Bad\n`repo:rust/absent.rs`\n")
        self.write(".trellis/spec/shared/verification.md", "# No evidence\nPlain assertions.\n")
        errors = self.errors()
        self.assertTrue(any("missing evidence file" in error for error in errors))
        self.assertTrue(any("missing source evidence" in error for error in errors))

    def test_each_scenario_needs_all_sections(self) -> None:
        complete = "## Scenario: Complete\n" + "\n".join(
            f"### {index}. {name}\nSynthetic assertion."
            for index, name in enumerate(API["SECTIONS"], 1))
        self.write(".trellis/spec/shared/engineering.md", "# Contract\n" + self.evidence + complete + "\n")
        self.assertEqual(self.errors(), [])
        self.write(".trellis/spec/shared/engineering.md", "# Contract\n" + self.evidence + complete +
                   "\n## Scenario: Incomplete\n### 1. Scope / Trigger\nSynthetic trigger.\n")
        self.assertEqual(sum("incomplete scenario" in error for error in self.errors()), 6)

    def test_missing_owner_navigation_and_ui_target(self) -> None:
        self.write(".trellis/spec/nxtools-fixture/backend/index.md", "# Broken owner\n")
        self.write(".trellis/spec/shared/ui-design.md", "# Wrong authority\n" + self.evidence)
        errors = self.errors()
        self.assertTrue(any("missing topic navigation" in error for error in errors))
        self.assertTrue(any("missing owner routing" in error for error in errors))
        self.assertTrue(any("UI authority matrix missing" in error for error in errors))

    def test_fenced_example_link_is_not_navigation(self) -> None:
        path = self.spec / "shared/architecture.md"
        path.write_text(path.read_text() + "\n```text\n[Example](nonexistent.md)\n```\n", encoding="utf-8")
        self.assertEqual(self.errors(), [])

    def test_oversized_document_and_escaping_evidence_are_rejected(self) -> None:
        self.write(".trellis/spec/shared/architecture.md", "# Huge\n" + "x" * API["MAX_DOCUMENT_BYTES"])
        self.write(".trellis/spec/shared/verification.md", "# Escape\n`repo:../outside.rs`\n")
        errors = self.errors()
        self.assertTrue(any("unsafe/oversized spec" in error for error in errors))
        self.assertTrue(any("missing evidence file" in error for error in errors))

    def test_empty_contract_sections_are_rejected(self) -> None:
        empty = "## Scenario: Empty\n" + "\n".join(
            f"### {index}. {name}" for index, name in enumerate(API["SECTIONS"], 1))
        self.write(".trellis/spec/shared/engineering.md", "# Contract\n" + self.evidence + empty + "\n")
        self.assertEqual(sum("empty scenario section" in error for error in self.errors()), 7)

    def test_unclosed_code_fence_is_rejected(self) -> None:
        path = self.spec / "shared/architecture.md"
        path.write_text(path.read_text() + "\n```text\nUnclosed example\n", encoding="utf-8")
        self.assertTrue(any("unclosed code fence" in error for error in self.errors()))

    def test_contract_subheadings_have_real_body(self) -> None:
        complete = "## Scenario: Subheadings\n" + "\n".join(
            f"### {index}. {name}\n#### Example\nSynthetic assertion."
            for index, name in enumerate(API["SECTIONS"], 1))
        self.write(".trellis/spec/shared/engineering.md", "# Contract\n" + self.evidence + complete + "\n")
        self.assertEqual(self.errors(), [])


if __name__ == "__main__":
    unittest.main()
