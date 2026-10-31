#!/usr/bin/env python3
"""Opt-in raw-frame Release worker regression; never builds or grants native output.

Pass the actual Cargo artifact record produced by the caller's normal Release build.
All requests are synthetic and retained under a fresh child of --evidence-root.
The successful source_acquire reply is transport fact injection, NOT a C# lease/Job.
"""

import argparse
import copy
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import time
import uuid
import zipfile


MAX_FRAME = 1024 * 1024
MAX_CAPTURE = 64 * 1024
MAX_FRAMES = 16
PAYLOAD = b"release-private-field-control\n"


def check(condition, detail):
    """Keep regression assertions active even when Python is invoked with -O."""
    if not condition:
        raise AssertionError(detail)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()


def save_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def pack(value):
    body = json.dumps(value, separators=(",", ":")).encode("utf-8")
    check(0 < len(body) <= MAX_FRAME, "request frame bound")
    return struct.pack("<I", len(body)) + body


def decode_available(raw):
    """Return complete frames and their byte extent; partial data waits for EOF."""
    check(len(raw) <= MAX_CAPTURE, "capture bound exceeded")
    frames = []
    offset = 0
    while len(raw) - offset >= 4:
        size = struct.unpack_from("<I", raw, offset)[0]
        check(0 < size <= MAX_FRAME, "invalid frame length")
        if len(raw) - offset - 4 < size:
            break
        value = json.loads(raw[offset + 4:offset + 4 + size])
        check(isinstance(value, dict), "frame must be an object")
        frames.append(value)
        check(len(frames) <= MAX_FRAMES, "frame count bound exceeded")
        offset += size + 4
    return frames, offset


def release_identity(worker, record):
    artifact = record["artifact"]
    profile = artifact["profile"]
    check(artifact["reason"] == "compiler-artifact", "actual Cargo artifact required")
    check(artifact["target"]["name"] == "nxtools-archive-worker", "wrong Cargo target")
    check("bin" in artifact["target"]["kind"], "worker binary required")
    check(profile["debug_assertions"] is False, "not a Release assertion profile")
    check(profile["opt_level"] == "3" and profile["test"] is False, "Release profile required")
    check("isolated-codecs" in artifact["features"], "matching isolated-codecs feature required")
    check(Path(artifact["executable"]).resolve(strict=True) == worker, "artifact path mismatch")
    actual = digest(worker)
    check(actual == record["sha256"], "artifact hash mismatch")
    return {"path": str(worker), "sha256": actual, "bytes": worker.stat().st_size,
            "profile": profile, "features": artifact["features"]}


def requests(source):
    base = {"schema": 2, "operation_id": str(uuid.uuid4()), "op": "start",
            "operation": "test", "path": str(source), "codepage": 0}
    cases = [("control_test", base, True)]
    for operation in ("test", "extract"):
        for suffix, value in (("object", {"bytes": 1, "ratio": 1}), ("null", None)):
            request = copy.deepcopy(base)
            request["operation"] = operation
            request["zip_risk_fixture"] = value
            cases.append((f"{operation}_fixture_{suffix}", request, False))
    unknown = copy.deepcopy(base)
    unknown["unknown_release_probe"] = True
    cases.append(("unknown_field", unknown, False))
    return cases


def verdict(result, control):
    check(result["bounded_exit"] and not result["forced"], "no natural bounded exit")
    check(result["driver_error"] is None, result["driver_error"])
    check(result["stderr_bytes"] == 0, "private stderr must be empty")
    check(all(frame.get("operation_id") == result["operation_id"]
              and frame.get("schema") == 1 for frame in result["frames"]), "frame identity")
    if control:
        check(result["exit_code"] == 0, "control process exit")
        check(result["frame_sequence"] == ["accepted", "native_call", "complete"],
              "control must complete, not merely be accepted")
        check(result["native_actions"] == ["source_acquire"], "no output capability allowed")
        terminal = result["frames"][-1]
        check(terminal.get("files_verified") == 2, "literal control file count")
        check(terminal.get("bytes_verified") == 30, "literal control byte count")
    else:
        check(result["exit_code"] == 1, "expected original parser exit1")
        check(result["stdout_bytes"] == 0 and result["frames"] == [],
              "disabled field reached accepted/source/execution instead of strict parsing")
        check(result["native_actions"] == [], "source/output request on rejection")


def run_case(worker, root, name, request, control, timeout):
    directory = root / name
    directory.mkdir()
    wire = pack(request)
    save_json(directory / "request.json", request)
    (directory / "request.bin").write_bytes(wire)
    started = time.monotonic()
    frames = []
    handled = 0
    driver_error = None
    forced = False
    process = None
    replies = []
    with (directory / "stdout.bin").open("wb") as stdout, \
            (directory / "stderr.bin").open("wb") as stderr:
        try:
            process = subprocess.Popen([str(worker), "--archive-worker"], stdin=subprocess.PIPE,
                                       stdout=stdout, stderr=stderr, cwd=directory)
            process.stdin.write(wire)
            process.stdin.flush()
            deadline = started + timeout
            while process.poll() is None:
                raw = (directory / "stdout.bin").read_bytes()
                frames, _ = decode_available(raw)
                check((directory / "stderr.bin").stat().st_size <= MAX_CAPTURE,
                      "stderr capture bound exceeded")
                if not control and raw:
                    raise AssertionError("negative request emitted protocol data")
                for frame in frames[handled:]:
                    check(frame.get("operation_id") == request["operation_id"], "operation ID")
                    check(frame.get("schema") == 1, "original response schema")
                    if frame.get("op") == "native_call":
                        action = frame["native"]["action"]
                        check(control and action == "source_acquire" and not replies,
                              "unexpected native call; no output authority granted")
                        reply = {"version": 1, "operation_id": request["operation_id"],
                                 "op": "native_result", "call_id": frame["native"]["call_id"],
                                 "success": True}
                        process.stdin.write(pack(reply))
                        process.stdin.flush()
                        replies.append(reply)
                    elif frame.get("op") == "complete":
                        process.stdin.close()
                    else:
                        check(frame.get("op") == "accepted", "unexpected control frame")
                    handled += 1
                if time.monotonic() >= deadline:
                    raise TimeoutError("owned worker deadline exceeded")
                time.sleep(0.01)
            if time.monotonic() >= deadline:
                raise TimeoutError("owned worker exited after deadline")
            process.wait(timeout=1)
        except (AssertionError, OSError, ValueError, KeyError, TimeoutError) as failure:
            driver_error = type(failure).__name__ + ": " + str(failure)
        finally:
            if process is not None:
                if process.stdin is not None and not process.stdin.closed:
                    process.stdin.close()
                try:
                    process.wait(timeout=1)
                except subprocess.TimeoutExpired:
                    forced = True
                    process.kill()  # Only this exact Popen child; never a process-name search.
                    process.wait(timeout=5)
    raw = (directory / "stdout.bin").read_bytes()
    try:
        frames, consumed = decode_available(raw)
        check(consumed == len(raw), "partial frame at EOF")
    except (AssertionError, ValueError) as failure:
        driver_error = driver_error or type(failure).__name__ + ": " + str(failure)
    save_json(directory / "source-replies.json", replies)
    result = {"name": name, "expected": "complete" if control else "parser_exit1_zero_frames",
              "pid": process.pid if process is not None else None,
              "operation_id": request["operation_id"],
              "exit_code": process.returncode if process is not None else None,
              "elapsed_ms": round((time.monotonic() - started) * 1000, 3),
              "bounded_exit": process is not None and process.returncode is not None,
              "forced": forced, "driver_error": driver_error, "frames": frames,
              "frame_sequence": [frame.get("op") for frame in frames],
              "native_actions": [frame["native"]["action"] for frame in frames if "native" in frame],
              "stdout_bytes": len(raw), "stderr_bytes": (directory / "stderr.bin").stat().st_size}
    save_json(directory / "result.json", result)
    verdict(result, control)
    result["verdict"] = "PASS"
    save_json(directory / "result.json", result)
    print(f"PASS {name} exit={result['exit_code']} frames={len(frames)}", flush=True)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worker", required=True, type=Path)
    parser.add_argument("--build-record", required=True, type=Path)
    parser.add_argument("--evidence-root", required=True, type=Path)
    parser.add_argument("--timeout", type=float, default=10)
    options = parser.parse_args()
    if not 1 <= options.timeout <= 30:
        parser.error("timeout must be within 1..30 seconds")
    worker = options.worker.resolve(strict=True)
    record = json.loads(options.build_record.read_text(encoding="utf-8-sig"))
    identity = release_identity(worker, record)
    root = options.evidence_root.resolve(strict=True) / ("runtime-" + str(uuid.uuid4()))
    root.mkdir()
    print("Evidence: " + str(root), flush=True)
    save_json(root / "worker-before.json", identity)
    archive = root / "tiny.zip"
    with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_STORED) as output:
        output.writestr("alpha.txt", PAYLOAD)
        output.writestr("empty.txt", b"")
    source_hash = digest(archive)
    save_json(root / "fixture.json", {"producer": "CPython zipfile ZIP_STORED", "sha256": source_hash,
                                     "entries": {"alpha.txt": len(PAYLOAD), "empty.txt": 0}})
    results = []
    try:
        for name, request, control in requests(archive):
            results.append(run_case(worker, root, name, request, control, options.timeout))
            save_json(root / "results.json", results)
    finally:
        after = release_identity(worker, record)
        save_json(root / "worker-after.json", after)
        check(identity == after and source_hash == digest(archive), "worker/source changed")
    check(len(results) == 6 and all(row["verdict"] == "PASS" for row in results), "exact six cases")
    print("SUMMARY selected=6 run=6 pass=6 fail=0 skip=0 not_run=0", flush=True)


if __name__ == "__main__":
    main()
