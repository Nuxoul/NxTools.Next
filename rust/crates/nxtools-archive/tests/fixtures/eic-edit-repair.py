"""EIC exact worker/native-reply regressions. GUID-owned inputs; no real publication."""
import base64
import hashlib
import io
import json
import struct
import subprocess
import sys
import tarfile
import threading
import uuid
import zipfile
from pathlib import Path

worker, root, scenario = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
cancel = scenario.startswith("reply-")
fmt = sys.argv[4] if cancel else "Zip"
error = sys.argv[5] if cancel else None
source = root / ("owned.7z" if fmt == "SevenZ" else "owned.tar" if fmt == "Tar" else "owned.zip")
expected = {}
if fmt == "Tar":
    with tarfile.open(source, "w", format=tarfile.USTAR_FORMAT) as archive:
        header = tarfile.TarInfo("old.txt")
        header.size = 7
        archive.addfile(header, io.BytesIO(b"payload"))
elif fmt == "Zip":
    expected = {"old.txt": b"payload"} if cancel else {"folder/same.txt": b"old-content", "untouched.txt": b"keep"}
    if scenario == "explicit-skip":
        expected["folder/"] = b""
    if scenario == "boundary-new":
        expected = {"folder-other/same.txt": b"old-content", "untouched.txt": b"keep"}
    if scenario == "kind-conflict":
        expected = {"folder": b"file", "untouched.txt": b"keep"}
    with zipfile.ZipFile(source, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, content in expected.items():
            archive.writestr(name, content)
else:
    assert source.is_file(), "Rust tiny 7z seed required"
original = source.read_bytes()
op, token, directory_id, same_id, fresh_id = (str(uuid.uuid4()) for _ in range(5))
intent = dict(browser=dict(snapshot_id=str(uuid.uuid4()), format=fmt, comment="", source=dict(native="v1:fixture", digest=list(hashlib.sha256(original).digest()), size=len(original), modified_unix_ns=source.stat().st_mtime_ns)), action="rename" if cancel else "add", ordinals=[0] if cancel else [])
if cancel:
    intent["value"] = "new.txt"
request = dict(schema=2, operation_id=op, op="start", operation={"Zip":"modify_zip", "Tar":"modify_tar", "SevenZ":"modify_7z"}[fmt], path=str(source), job_execution=True, job_modify=intent)
if not cancel:
    request.update(password_interaction=True, conflict_policy="ask")
child = subprocess.Popen([str(worker), "--archive-worker"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
timer = threading.Timer(30, lambda: child.kill() if child.poll() is None else None)
timer.start()
wire, output = [], bytearray()
counts = dict(commit=0, begin=0, source_open=0, source_close=0, prompts=0)
handed = False

def send(value):
    body = json.dumps(value, separators=(",", ":")).encode()
    child.stdin.write(struct.pack("<I", len(body)) + body)
    child.stdin.flush()

def exact(count):
    result = b""
    while len(result) < count:
        part = child.stdout.read(count-len(result))
        assert part, "EOF before terminal"
        result += part
    return result

try:
    send(request)
    while True:
        frame = json.loads(exact(struct.unpack("<I", exact(4))[0]))
        wire.append(frame)
        if frame["op"] in ("complete", "failure"):
            terminal = frame
            break
        if frame["op"] == "conflict_prompt":
            assert not cancel
            counts["prompts"] += 1
            prompt = frame["conflict_prompt"]
            assert prompt["display"] == dict(incoming_member="folder/same.txt", target="folder/same.txt")
            send(dict(version=1, operation_id=op, op="conflict_reply", conflict_reply=dict(ticket=prompt["ticket"], decision="skip", apply_to_all=True)))
            continue
        if frame["op"] != "native_call":
            continue
        call = frame["native"]
        action = call["action"]
        if action in counts:
            counts[action] += 1
        reply = dict(version=1, operation_id=op, op="native_result", call_id=call["call_id"], success=True)
        if action == "source_stamp":
            reply["read_data"] = "v1:fixture"
        elif action == "source_roots":
            reply["read_data"] = json.dumps([dict(id=directory_id, name="new-empty" if scenario == "new-empty" else "folder", directory=True, size=0, dos_modified=None)])
        elif action == "source_next":
            items = [] if scenario == "new-empty" else [dict(id=same_id, name="same.txt", directory=False, size=3, dos_modified=None)]
            if scenario == "real-file":
                items.append(dict(id=fresh_id, name="fresh.txt", directory=False, size=5, dos_modified=None))
            offset = call["offset"]
            reply["read_data"] = json.dumps(items[offset] if offset < len(items) else None)
        elif action == "source_read":
            content = b"fresh" if call["token"] == fresh_id else b"new"
            reply["read_data"] = base64.b64encode(content[call["offset"]:call["offset"]+call["length"]]).decode()
        elif action == "begin":
            reply["token"] = token
        elif action == "write":
            output.extend(bytes(call["data"]))
        elif action == "read":
            reply["read_data"] = base64.b64encode(output[call["offset"]:call["offset"]+call["length"]]).decode()
        elif action == "edit_reads_complete":
            assert counts["source_open"] == counts["source_close"]
            handed = True
        elif action == "commit":
            assert handed, "candidate validation and actual reader closure precede commit"
            if cancel:
                reply.update(success=False, error_code=error)
        elif action not in ("source_acquire", "source_check", "source_open", "source_close", "edit_planning", "edit_generating", "edit_reading", "edit_reencoding", "edit_verifying", "creation_verifying", "source_precommit", "abort"):
            raise AssertionError("unexpected native action " + action)
        send(reply)
    child.stdin.close()
    child.wait(timeout=10)
    assert child.returncode == 0 and not child.stderr.read()
finally:
    timer.cancel()
    if child.poll() is None:
        child.kill()
        child.wait(timeout=10)
    (root / "wire.json").write_text(json.dumps(wire, indent=2))
unchanged = source.read_bytes() == original
result = dict(scenario=scenario, format=fmt, error=error, terminal=terminal, counts=counts, handed=handed, source_unchanged=unchanged)
(root / "result.json").write_text(json.dumps(result, indent=2))
assert unchanged
if cancel:
    expected_error = {"cancelled":"Cancelled", "source_changed":"SourceChanged", "target_exists":"OutputCollision", "write_limit":"ResourceLimit"}.get(error, "IoFailure")
    assert counts["commit"] == 1 and handed
    assert terminal["op"] == "failure" and terminal["error_code"] == expected_error, result
elif scenario in ("implicit-skip", "explicit-skip"):
    assert terminal["op"] == "complete" and counts["prompts"] == 1
    assert counts["commit"] == counts["begin"] == counts["source_open"] == 0 and not output, result
    assert any(item.get("extraction_summary", {}).get("files_skipped") == 1 for item in wire)
elif scenario == "kind-conflict":
    assert terminal["op"] == "failure" and terminal["error_code"] == "OutputCollision" and counts["begin"] == counts["commit"] == 0, result
else:
    assert terminal["op"] == "complete" and counts["commit"] == 1, result
    if scenario == "new-empty":
        expected["new-empty/"] = b""
        assert counts["source_open"] == 0 and counts["prompts"] == 0
    elif scenario == "real-file":
        expected["folder/fresh.txt"] = b"fresh"
        assert counts["source_open"] == counts["source_close"] == 1 and counts["prompts"] == 1
    elif scenario == "boundary-new":
        expected.update({"folder/": b"", "folder/same.txt": b"new"})
        assert counts["source_open"] == 1 and counts["prompts"] == 0
    candidate = root / "candidate.zip"
    candidate.write_bytes(output)
    with zipfile.ZipFile(candidate) as archive:
        assert sorted(archive.namelist()) == sorted(expected), archive.namelist()
        assert archive.testzip() is None
        for name, content in expected.items():
            assert archive.read(name) == content
print("PASS", scenario, fmt, error, counts)
