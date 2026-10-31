"""Step4 tiny ZIP Add conflict protocol fixture; independent zipfile oracle, no native publish."""
import base64
import hashlib
import json
import struct
import subprocess
import sys
import threading
import uuid
import zipfile
from pathlib import Path

worker, root, decision = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
future = len(sys.argv) > 4 and sys.argv[4] == "future-name"
source, candidate = root / "owned.zip", root / "candidate.zip"
with zipfile.ZipFile(source, "w", compression=zipfile.ZIP_DEFLATED) as archive:
    archive.writestr("folder/", b"")
    archive.writestr("folder/same.txt", b"old-content")
    archive.writestr("untouched.txt", b"keep")
original = source.read_bytes()
op, token, source_id = (str(uuid.uuid4()) for _ in range(3))
future_id = str(uuid.uuid4())
request = dict(schema=2, operation_id=op, op="start", operation="modify_zip", path=str(source), job_execution=True, password_interaction=True, conflict_policy="ask", job_modify=dict(browser=dict(snapshot_id=str(uuid.uuid4()), format="Zip", comment="", source=dict(native="v1:fixture", digest=list(hashlib.sha256(original).digest()), size=len(original), modified_unix_ns=source.stat().st_mtime_ns)), action="add", ordinals=[], value="folder/"))
child = subprocess.Popen([str(worker), "--archive-worker"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
timer = threading.Timer(30, lambda: child.kill() if child.poll() is None else None)
timer.start()
output, wire = bytearray(), []
prompts = commits = opens = closes = 0
handed = False

def send(value):
    body = json.dumps(value, separators=(",", ":")).encode()
    child.stdin.write(struct.pack("<I", len(body)) + body)
    child.stdin.flush()

def exact(length):
    result = b""
    while len(result) < length:
        part = child.stdout.read(length - len(result))
        if not part:
            raise AssertionError("worker EOF before terminal")
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
            prompts += 1
            prompt = frame["conflict_prompt"]
            assert prompt["display"] == {"incoming_member": "folder/same.txt", "target": "folder/same.txt"}
            assert prompt["existing_bytes"] == 11 and prompt["incoming_bytes"] == 3
            ticket = dict(prompt["ticket"])
            stale = dict(ticket, prompt_id=str(uuid.uuid4()))
            send(dict(version=1, operation_id=op, op="conflict_reply", conflict_reply=dict(ticket=stale, decision="overwrite", apply_to_all=True)))
            send(dict(version=1, operation_id=op, op="conflict_reply", conflict_reply=dict(ticket=ticket, decision=None if decision == "cancel" else decision, apply_to_all=True)))
            # Duplicate current ticket must not create another decision/publication.
            send(dict(version=1, operation_id=op, op="conflict_reply", conflict_reply=dict(ticket=ticket, decision="overwrite", apply_to_all=True)))
            continue
        if frame["op"] != "native_call":
            continue
        call = frame["native"]
        action = call["action"]
        reply = dict(version=1, operation_id=op, op="native_result", call_id=call["call_id"], success=True)
        if action == "source_stamp":
            reply["read_data"] = "v1:fixture"
        elif action == "source_roots":
            roots = [dict(id=source_id, name="same.txt", directory=False, size=3, dos_modified=None)]
            if future:
                roots.append(dict(id=future_id, name="same (2).txt", directory=False, size=5, dos_modified=None))
            reply["read_data"] = json.dumps(roots)
        elif action == "source_open":
            opens += 1
        elif action == "source_close":
            closes += 1
        elif action == "source_read":
            content = b"later" if call["token"] == future_id else b"new"
            reply["read_data"] = base64.b64encode(content[call["offset"]:call["offset"] + call["length"]]).decode()
        elif action == "begin":
            reply["token"] = token
        elif action == "write":
            output.extend(bytes(call["data"]))
        elif action == "read":
            reply["read_data"] = base64.b64encode(output[call["offset"]:call["offset"] + call["length"]]).decode()
        elif action == "edit_reads_complete":
            assert opens == closes
            handed = True
        elif action == "commit":
            assert handed
            commits += 1
        elif action not in ("source_acquire", "source_check", "edit_generating", "edit_verifying", "source_precommit", "abort"):
            raise AssertionError("unexpected action " + action)
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
assert source.read_bytes() == original
assert prompts == 1
assert opens == closes
if decision == "cancel":
    assert terminal["op"] == "failure" and terminal["error_code"] == "Cancelled" and commits == 0 and opens == 0
elif decision == "skip":
    assert terminal["op"] == "complete" and commits == 0 and opens == 0
    assert any(frame.get("extraction_summary", {}).get("files_skipped") == 1 for frame in wire)
else:
    assert terminal["op"] == "complete" and commits == 1 and handed
    candidate.write_bytes(output)
    with zipfile.ZipFile(candidate) as archive:
        expected = {"folder/": b"", "folder/same.txt": b"new" if decision == "overwrite" else b"old-content", "untouched.txt": b"keep"}
        if decision == "auto_rename":
            expected["folder/same (3).txt" if future else "folder/same (2).txt"] = b"new"
        if future:
            expected["folder/same (2).txt"] = b"later"
        assert sorted(archive.namelist()) == sorted(expected)
        assert archive.testzip() is None
        for name, content in expected.items():
            assert archive.read(name) == content
(root / "result.json").write_text(json.dumps(dict(decision=decision, terminal=terminal, prompts=prompts, commits=commits, source_unchanged=True, independent="CPython zipfile PASS"), indent=2))
print("PASS", decision, "single candidate / source unchanged / original tickets")
