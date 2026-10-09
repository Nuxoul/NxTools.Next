#!/usr/bin/env python3
"""Read-only checks for first-party spec navigation and contract structure.

No external requests, product execution, symbol/behavior inference or file writes.
Use --root for a complete synthetic repository fixture. Python standard library only.
"""
from __future__ import annotations

import argparse
import re
import sys
import tomllib
from pathlib import Path
from urllib.parse import unquote, urlsplit

MAX_DOCUMENTS = 250
MAX_DOCUMENT_BYTES = 512 * 1024
TOPICS = (
    "directory-structure.md", "database-guidelines.md", "error-handling.md",
    "logging-guidelines.md", "quality-guidelines.md",
)
SECTIONS = (
    "Scope / Trigger", "Signatures", "Contracts", "Validation & Error Matrix",
    "Good/Base/Bad Cases", "Tests Required", "Wrong vs Correct",
)
TEMPLATE = re.compile(
    r"To be filled by the team|\bTo fill\b|\bTBD\b|Questions to answer:|"
    r"Document your project(?:'s)?|Replace with your actual structure|"
    r"Fill in each file|How to Fill These Guidelines", re.IGNORECASE,
)
LINK = re.compile(r"\[[^\]\n]*\]\(([^)\n]+)\)")
EVIDENCE = re.compile(r"`repo:([A-Za-z0-9_.][^`\s#]+)(?:#[^`]*)?`")
SCENARIO = re.compile(r"(?m)^## Scenario:[^\n]*$")


def inside(root: Path, path: Path) -> bool:
    return path.is_relative_to(root)


def prose(text: str) -> str:
    """Exclude fenced illustrative code from Markdown navigation parsing."""
    result = []
    fence = None
    for line in text.splitlines():
        marker = re.match(r"^\s*(`{3,}|~{3,})", line)
        if marker:
            kind = marker[1][0]
            if fence is None:
                fence = kind
            elif fence == kind:
                fence = None
            continue
        if fence is None:
            result.append(line)
    if fence is not None:
        raise ValueError("unclosed code fence")
    return "\n".join(result)


def check(root: Path) -> tuple[list[str], int, int]:
    root = root.resolve()
    spec = root / ".trellis/spec"
    errors: list[str] = []
    docs: dict[Path, str] = {}
    if not spec.is_dir():
        return ["missing .trellis/spec directory"], 0, 0
    for path in sorted(spec.rglob("*.md")):
        if len(docs) >= MAX_DOCUMENTS:
            errors.append(f"document budget exceeded ({MAX_DOCUMENTS})")
            break
        if not inside(root, path.resolve()) or path.stat().st_size > MAX_DOCUMENT_BYTES:
            errors.append(f"unsafe/oversized spec: {path.relative_to(root)}")
            continue
        try:
            docs[path] = path.read_text(encoding="utf-8-sig")
        except (OSError, UnicodeError) as failure:
            errors.append(f"unreadable spec {path.relative_to(root)}: {type(failure).__name__}")

    links: dict[Path, set[Path]] = {}
    for path, text in docs.items():
        label = str(path.relative_to(root))
        if not text.startswith("# "):
            errors.append(f"missing document title: {label}")
        if TEMPLATE.search(text):
            errors.append(f"template residue: {label}")
        try:
            content = prose(text)
        except ValueError:
            errors.append(f"unclosed code fence: {label}")
            content = ""
        links[path] = set()
        for match in LINK.finditer(content):
            value = match[1].strip().strip("<>")
            parsed = urlsplit(value)
            if parsed.scheme or parsed.netloc or not parsed.path:
                continue
            target = (path.parent / unquote(parsed.path)).resolve()
            if not inside(root, target) or not target.is_file():
                errors.append(f"broken local link: {label} -> {value}")
            else:
                links[path].add(target)
        evidence = list(EVIDENCE.finditer(text))
        if path.name != "index.md" and "guides" not in path.relative_to(spec).parts and not evidence:
            errors.append(f"missing source evidence: {label}")
        for match in evidence:
            target = (root / match[1]).resolve()
            if not inside(root, target) or not target.is_file():
                errors.append(f"missing evidence file: {label} -> {match[1]}")
        starts = list(SCENARIO.finditer(text))
        for index, start in enumerate(starts):
            end = starts[index + 1].start() if index + 1 < len(starts) else len(text)
            block = text[start.end():end]
            for number, title in enumerate(SECTIONS, 1):
                # Historical headings allow whitespace around the slash only.
                heading = re.escape(title).replace("/", r"\s*/\s*")
                section = re.search(rf"(?m)^### {number}\.[ \t]+{heading}[ \t]*$", block)
                if section is None:
                    errors.append(f"incomplete scenario: {label}: {start[0]} missing {number}. {title}")
                elif not re.split(r"(?m)^#{2,3} ", block[section.end():], maxsplit=1)[0].strip():
                    errors.append(f"empty scenario section: {label}: {start[0]}: {number}. {title}")

    package_count = 0
    try:
        workspace = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8-sig"))
        for member in workspace["workspace"]["members"]:
            manifest = (root / member / "Cargo.toml").resolve()
            if not inside(root, manifest):
                raise ValueError("workspace member escapes root")
            package = tomllib.loads(manifest.read_text(encoding="utf-8-sig"))["package"]["name"]
            package_count += 1
            folder = spec / package / "backend"
            entry = folder / "index.md"
            index_text = docs.get(entry, "")
            if not index_text:
                errors.append(f"missing owner index: {package}")
            for heading in ("Pre-Development Checklist", "Quality Check"):
                if f"## {heading}" not in index_text:
                    errors.append(f"missing owner routing: {package}: {heading}")
            for topic in TOPICS:
                if folder / topic not in docs or (folder / topic).resolve() not in links.get(entry, set()):
                    errors.append(f"missing topic navigation: {package}/{topic}")
            if entry.resolve() not in links.get(spec / "index.md", set()):
                errors.append(f"owner absent from standards entry: {package}")
    except (OSError, ValueError, KeyError, TypeError) as failure:
        errors.append(f"workspace discovery failed: {type(failure).__name__}")

    required = (
        "index.md", "shared/architecture.md", "shared/engineering.md", "shared/verification.md",
        "shared/ui-design.md", "shared/foundation-backlog.md", "platformhost/index.md",
        "browser-extension/index.md",
    )
    for relative in required:
        if spec / relative not in docs:
            errors.append(f"missing standards document: {relative}")
    for folder in ("platformhost", "browser-extension"):
        entry = spec / folder / "index.md"
        for topic in (spec / folder).glob("*.md"):
            if topic != entry and topic.resolve() not in links.get(entry, set()):
                errors.append(f"missing topic navigation: {folder}/{topic.name}")
        if entry.resolve() not in links.get(spec / "index.md", set()):
            errors.append(f"owner absent from standards entry: {folder}")
    ui = docs.get(spec / "shared/ui-design.md", "")
    for token in ("Approved target", "Current evidence", "Exception", "#18181b", "#0ea5e9", "24/28/32", "3/6/8"):
        if token.casefold() not in ui.casefold():
            errors.append(f"UI authority matrix missing: {token}")
    return sorted(set(errors)), len(docs), package_count


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    errors, document_count, package_count = check(args.root)
    for failure in errors:
        print(f"FAIL {failure}")
    if errors:
        print(f"FAIL specs: {len(errors)} violations in {document_count} documents")
        return 1
    print(f"PASS specs: {document_count} documents, {package_count} workspace owners; navigation/evidence/contracts")
    print("Scope: source files/links/structure only; not symbol meaning, UI or native/release acceptance.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
