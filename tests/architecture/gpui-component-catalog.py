#!/usr/bin/env python3
"""Read-only local documentation gate; no network/product/build/native operations."""
from __future__ import annotations

import json
import re
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[2]
EXPECTED_COUNT = 79
SHA256 = re.compile(r"[a-fA-F0-9]{64}\Z")


def check(root: Path = ROOT) -> tuple[list[str], int]:
    docs = root / "docs/reference-code"
    failures: list[str] = []
    paths = {
        "data": docs / "gpui-kit-component-catalog.json",
        "table": docs / "gpui-kit-component-table.md",
        "viewer": docs / "gpui-kit-components.html",
        "rules": root / ".trellis/spec/shared/ui-component-selection.md",
        "index": docs / "gpui-kit-component-index.md",
        "agents": root / "AGENTS.md",
        "spec_index": root / ".trellis/spec/index.md",
        "desktop_index": root / ".trellis/spec/nxtools-desktop/backend/index.md",
    }
    for key, path in paths.items():
        if not path.is_file():
            failures.append(f"missing {key}: {path.relative_to(root)}")
    if failures:
        return failures, 0
    try:
        texts = {key: path.read_text(encoding="utf-8-sig") for key, path in paths.items()}
        data = json.loads(texts["data"])
    except (ValueError, OSError, UnicodeError) as exc:
        return [f"unreadable catalog: {type(exc).__name__}"], 0
    entries = data.get("entries", [])
    if not isinstance(entries, list):
        return ["entries is not a list"], 0
    if len(entries) != EXPECTED_COUNT or data.get("component_count") != EXPECTED_COUNT:
        failures.append("historical Component inventory must contain79 entries; re-audit names/URLs before changing this gate")
    # These hashes/pages remain the original2026-10-15 audit, not a new release audit.
    if (data.get("website_version"), data.get("locked_version")) != ("0.7.3", "0.7.1"):
        failures.append("historical website/release evidence distinction missing")
    if data.get("page_fetch_success") != EXPECTED_COUNT or data.get("source_entry_count") != 78:
        failures.append("page/source-entry evidence counts mismatch")
    if not SHA256.fullmatch(data.get("catalog_sha256", "")):
        failures.append("missing catalog digest")
    names: set[str] = set()
    urls: set[str] = set()
    required = ("name", "name_zh", "category", "use_zh", "use", "caution", "url", "path", "locked_entry", "source")
    for entry in entries:
        if not isinstance(entry, dict):
            failures.append("non-object entry")
            continue
        name = entry.get("name", "?")
        if any(not isinstance(entry.get(key), str) or not entry[key].strip() for key in required):
            failures.append(f"incomplete selection row: {name}")
        if name in names:
            failures.append(f"duplicate name: {name}")
        names.add(name)
        url = entry.get("url", "")
        if url in urls:
            failures.append(f"duplicate URL: {name}")
        urls.add(url)
        parsed = urlsplit(url)
        if parsed.scheme != "https" or parsed.netloc != "gpui-kit.com" or not parsed.path.rstrip("/").startswith("/zh-CN/component/"):
            failures.append(f"nonofficial Component URL: {name}")
        if parsed.path.rstrip("/") != entry.get("path"):
            failures.append(f"URL/path mismatch: {name}")
        if entry.get("http_status") != 200 or not SHA256.fullmatch(entry.get("page_sha256", "")):
            failures.append(f"missing successful page evidence: {name}")
        if not entry.get("reviewed_sections"):
            failures.append(f"missing reviewed sections: {name}")
        expected_status = "unavailable" if name == "Diff" else "entry-verified"
        if entry.get("locked_status") != expected_status:
            failures.append(f"wrong locked release availability: {name}")
        for source in entry.get("source_files", []):
            if not source.get("exists") or not SHA256.fullmatch(source.get("sha256") or ""):
                failures.append(f"invalid source-file evidence: {name}")
        if name != "Diff" and not entry.get("source_files"):
            failures.append(f"missing locked source-file mapping: {name}")
        if f"| [{name}]({url}) |" not in texts["table"]:
            failures.append(f"missing Markdown row: {name}")
    if sum(line.startswith("| [") for line in texts["table"].splitlines()) != EXPECTED_COUNT:
        failures.append("Markdown row count mismatch")
    embedded = re.search(r'<script id="catalog-data" type="application/json">(.*?)</script>', texts["viewer"], re.S)
    try:
        if not embedded or json.loads(embedded[1]) != data:
            failures.append("offline viewer embedded catalog differs from JSON")
    except ValueError:
        failures.append("invalid embedded catalog JSON")
    if re.search(r"\bfetch\s*\(|<script[^>]+src=|<link[^>]+(?:stylesheet|preload)", texts["viewer"], re.I):
        failures.append("offline viewer must not depend on network assets/fetch")
    if paths["rules"].stat().st_size >= 32768:
        failures.append("short mandatory rules exceed default injection budget")
    for key in ("agents", "spec_index", "desktop_index", "index"):
        if "ui-component-selection.md" not in texts[key]:
            failures.append(f"{key} does not route AI to short selection rules")
    for token in ("prepare_for_toolbar", "ghost().compact()", "Toolbar.content", "Other Tools", "DataTable", "not yet implemented"):
        if token not in texts["rules"]:
            failures.append(f"missing executable selection caution: {token}")
    if 'gpui-kit = "=0.10.0"' not in (root / "Cargo.toml").read_text("utf-8-sig"):
        failures.append("current product must pin reviewed0.10.0")
    current_release = docs / "gpui-kit-0100.md"
    if not current_release.is_file() or not all(token in current_release.read_text("utf-8") for token in ("0.10.0", "0.3.12", "editor_host", "WM_MOUSEACTIVATE", "notification", "historical")):
        failures.append("missing current release/source/patch review")
    for key in ("agents", "spec_index", "desktop_index", "table", "viewer", "rules", "index"):
        if "gpui-kit-0100.md" not in texts[key]:
            failures.append(f"{key} does not route current0.10.0 before historical catalog")
    return failures, len(entries)


if __name__ == "__main__":
    errors, count = check()
    for error in errors:
        print(f"FAIL {error}")
    if errors:
        raise SystemExit(1)
    print(f"PASS GPUI catalog: {count} historical pages/rows;78 historical0.7.1 entries; current0.10.0 release authority and selection cautions")
    print("Scope: local documentation consistency only; not live website, Rust API compilation, product visuals/interaction or native acceptance.")
