#!/usr/bin/env python3
"""Opt-in real archive CLI regression against an explicitly supplied isolated build.

Never builds, installs, launches Desktop, touches services or removes test evidence.
All fixtures/output live in a fresh UUID child of --evidence-root. No user archive inputs.
"""
import argparse
import json
from pathlib import Path
import subprocess
import uuid
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", required=True, type=Path)
    parser.add_argument("--evidence-root", required=True, type=Path)
    options = parser.parse_args()
    cli = options.cli.resolve(strict=True)
    if cli.name != "NxTools.Cli.exe":
        parser.error("use the exact isolated NxTools.Cli.exe sibling layout")
    root = options.evidence_root.resolve(strict=True) / ("archive-cli-" + str(uuid.uuid4()))
    root.mkdir()
    print("Evidence: " + str(root), flush=True)
    archive = root / "tiny.zip"
    with zipfile.ZipFile(archive, "w") as output:
        output.writestr("alpha.txt", b"alpha\n")
        output.writestr("empty/", b"")
        output.writestr("deep/implicit/beta.txt", b"beta\n")
    raw = bytearray(archive.read_bytes())
    raw[30 + len("alpha.txt")] = ord("X")
    corrupt = root / "crc.zip"
    corrupt.write_bytes(raw)
    unsafe = root / "unsafe.zip"
    with zipfile.ZipFile(unsafe, "w") as output:
        output.writestr("../escape.txt", b"no")
    unsupported = root / "unsupported.bin"
    unsupported.write_bytes(b"not an archive")
    results = []

    def run(name, *arguments, expected=0):
        process = subprocess.run(
            [str(cli), "archive", *map(str, arguments), "--json"],
            capture_output=True, text=True, encoding="utf-8", timeout=70,
        )
        (root / (name + ".stdout.json")).write_text(process.stdout, encoding="utf-8")
        (root / (name + ".stderr.txt")).write_text(process.stderr, encoding="utf-8")
        try:
            value = json.loads(process.stdout)
        except Exception as failure:
            raise AssertionError((name, process.returncode, process.stdout, process.stderr)) from failure
        results.append({"case": name, "exit": process.returncode, "success": value["success"],
                        "error": value["error"], "request_id": value["request_id"]})
        (root / "results.json").write_text(json.dumps(results, indent=2), encoding="utf-8")
        assert process.returncode == expected, (name, process.returncode, value)
        assert not process.stderr, (name, "unexpected stderr", process.stderr)
        assert value["schema_version"] == 1
        uuid.UUID(value["request_id"])
        if expected == 0:
            assert value["success"] and value["error"] is None
        else:
            assert not value["success"] and value["error"] is not None
        print("PASS", name, "exit=" + str(process.returncode), flush=True)
        return value

    value = run("list", "list", archive)
    assert [entry["name"] for entry in value["result"]["entries"]] == [
        "alpha.txt", "empty/", "deep/implicit/beta.txt"]
    assert [entry["size"] for entry in value["result"]["entries"]] == [6, 0, 5]
    assert value["result"]["terminal"]["cleanup_complete"]
    value = run("test", "test", archive)
    assert value["result"]["terminal"]["files_verified"] == 2
    assert value["result"]["terminal"]["bytes_verified"] == 11
    assert run("crc-test", "test", corrupt, expected=7)["error"]["code"] == "corrupt_archive"
    destination = root / "output"
    destination.mkdir()
    value = run("extract", "extract", archive, "--dest", destination)
    assert value["result"]["terminal"]["cleanup_complete"]
    assert (destination / "alpha.txt").read_bytes() == b"alpha\n"
    assert (destination / "deep/implicit/beta.txt").read_bytes() == b"beta\n"
    assert (destination / "empty").is_dir()
    assert sorted(path.relative_to(destination).as_posix() for path in destination.rglob("*")) == [
        "alpha.txt", "deep", "deep/implicit", "deep/implicit/beta.txt", "empty"]
    (destination / "alpha.txt").write_bytes(b"preserve existing")
    value = run("conflict-fail", "extract", archive, "--dest", destination, expected=6)
    assert value["error"]["code"] == "output_collision"
    assert value["result"]["terminal"]["cleanup_complete"]
    assert (destination / "alpha.txt").read_bytes() == b"preserve existing"
    value = run("conflict-skip", "extract", archive, "--dest", destination, "--conflict", "skip")
    assert value["result"]["terminal"]["files_skipped"] == 2
    assert (destination / "alpha.txt").read_bytes() == b"preserve existing"
    run("conflict-rename", "extract", archive, "--dest", destination, "--conflict", "rename")
    assert (destination / "alpha.txt").read_bytes() == b"preserve existing"
    assert (destination / "alpha (2).txt").read_bytes() == b"alpha\n"
    run("conflict-overwrite", "extract", archive, "--dest", destination, "--conflict", "overwrite")
    assert (destination / "alpha.txt").read_bytes() == b"alpha\n"
    missing = root / "missing-destination"
    assert run("destination-refused", "extract", archive, "--dest", missing, expected=6)["error"]["code"] == "destination_required"
    assert not missing.exists()
    run("destination-created", "extract", archive, "--dest", missing, "--create-destination")
    assert (missing / "alpha.txt").read_bytes() == b"alpha\n"
    unsafe_destination = root / "unsafe-output"
    unsafe_destination.mkdir()
    assert run("unsafe-refused", "extract", unsafe, "--dest", unsafe_destination, expected=7)["error"]["code"] == "unsafe_path"
    assert not (root / "escape.txt").exists() and not list(unsafe_destination.iterdir())
    value = run("source-missing", "list", root / "absent.zip", expected=7)
    assert value["error"]["code"] == "source_unavailable" and value["error"]["native_error_code"] in (2, 3)
    assert run("unsupported", "test", unsupported, expected=7)["error"]["code"] == "unsupported_format"
    assert run("syntax-single-json", "extract", archive, expected=2)["error"]["code"] == "invalid_request"
    sentinel = "not-a-real-password"
    run("password-argv-rejected", "test", archive, "--password", sentinel, expected=2)
    assert sentinel not in (root / "password-argv-rejected.stdout.json").read_text(encoding="utf-8")
    summary = {"commands_passed": len(results), "evidence_root": str(root), "results": results}
    (root / "passed.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    print("PASS", len(results), "actual CLI commands; exact byte/tree/conflict checks; no cleanup")


if __name__ == "__main__":
    main()
