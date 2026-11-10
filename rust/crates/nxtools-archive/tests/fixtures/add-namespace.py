"""OPT-02 actual worker namespace matrix; native replies are controlled, not publication evidence."""
import base64
import hashlib
import io
import json
import os
import struct
import subprocess
import sys
import tarfile
import threading
import uuid
import zipfile
from pathlib import Path
from bulk_peer import BulkPeer

worker, root = Path(sys.argv[1]), Path(sys.argv[2])
fmt, scenario, decision = sys.argv[3:6]
old = {"folder/same.txt": b"old", "untouched.txt": b"keep"}
inputs = [("same.txt", b"first"), ("same.txt", b"second")]
expected = dict(old)
apply_all = scenario != "each-ticket"
responses = 1
opens_expected = 0
failure = None

if scenario in ("duplicates", "each-ticket"):
    old.pop("folder/same.txt")
    expected = dict(old)
    if decision == "overwrite":
        expected["folder/same.txt"] = b"second"
        opens_expected = 1
    elif decision == "skip":
        expected["folder/same.txt"] = b"first"
        opens_expected = 1
    elif decision == "auto_rename":
        expected.update({"folder/same.txt": b"first", "folder/same (2).txt": b"second"})
        opens_expected = 2
elif scenario in ("original", "apply-all", "per-ticket"):
    apply_all = scenario != "per-ticket"
    responses = 1 if apply_all or decision == "cancel" else 2
    if decision == "overwrite":
        expected["folder/same.txt"] = b"second"
        opens_expected = 1
    elif decision == "auto_rename":
        expected.update({"folder/same (2).txt": b"first", "folder/same (3).txt": b"second"})
        opens_expected = 2
elif scenario in ("future", "future-reverse", "case-future"):
    inputs = [("same.txt", b"first"), ("same (2).txt", b"second")]
    if scenario == "case-future":
        inputs[1] = ("SAME (2).TXT", b"second")
    if scenario == "future-reverse":
        inputs.reverse()
    expected.update({"folder/same (3).txt": b"first", "folder/" + ("SAME (2).TXT" if scenario == "case-future" else "same (2).txt"): b"second"})
    opens_expected = 2
elif scenario in ("directories", "directory-skip", "merge-only", "explicit-merge"):
    old = {"folder/Tree/Sub/same.txt": b"old", "untouched.txt": b"keep"}
    if scenario == "explicit-merge":
        old["folder/Tree/"] = None
        old["folder/Tree/Sub/"] = None
    expected = dict(old)
    inputs = [("tree", [("sub", [("same.txt", b"first")])]), ("TREE", [("SUB", [("same.txt", b"second")])])]
    if scenario in ("merge-only", "explicit-merge"):
        inputs = [("tree", []), ("TREE", [])]
        responses = 0
    elif decision == "overwrite":
        expected["folder/Tree/Sub/same.txt"] = b"second"
        opens_expected = 1
    elif decision == "auto_rename":
        expected.update({"folder/Tree/Sub/same (2).txt": b"first", "folder/Tree/Sub/same (3).txt": b"second"})
        opens_expected = 2
elif scenario == "new-empty":
    inputs = [("empty", []), ("EMPTY", [])]
    expected["folder/empty/"] = None
    responses = 0
elif scenario in ("kind-original", "kind-incoming", "kind-implicit"):
    if scenario == "kind-original":
        inputs = [("same.txt", [])]
    elif scenario == "kind-incoming":
        inputs = [("fresh", b"file"), ("fresh", [])]
    else:
        inputs = [("Tree", b"file")]
        old = {"folder/Tree/leaf": b"old"}
    expected = dict(old)
    failure = "OutputCollision"
    responses = 0
elif scenario in ("source-change", "stale", "readback", "commit-unknown"):
    inputs = [("fresh.txt", b"first")]
    expected["folder/fresh.txt"] = b"first"
    responses = 0
    opens_expected = 0 if scenario in ("source-change", "stale") else 1
    failure = {"source-change": "SourceChanged", "stale": "SourceChanged", "readback": "CorruptArchive", "commit-unknown": "IoFailure"}[scenario]
elif scenario == "display-overflow":
    inputs = [("many", [(f"item-{index:03}.txt", b"new") for index in range(300)])]
    expected["folder/many/"] = None
    expected.update({f"folder/many/item-{index:03}.txt": b"new" for index in range(300)})
    responses = 0
    opens_expected = 300
else:
    raise AssertionError(scenario)

if decision == "cancel" and responses:
    failure = "Cancelled"
    opens_expected = 0

source = root / ("owned.7z" if fmt == "SevenZ" else "owned.zip" if fmt == "Zip" else "owned.tar")
if fmt == "Zip":
    with zipfile.ZipFile(source, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, data in old.items():
            archive.writestr(name, data or b"")
elif fmt == "SevenZ":
    oracle = os.environ["NXTOOLS_ARCHIVE_7ZIP_ORACLE"]
    seed = root / "seed"
    seed.mkdir()
    for name, data in old.items():
        path = seed / name
        if data is None:
            path.mkdir(parents=True, exist_ok=True)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    command = [oracle, "a", "-t7z", "-mx=1", "-ms=off", str(source), *old.keys()]
    result = subprocess.run(command, cwd=seed, capture_output=True, timeout=30)
    assert result.returncode == 0, result.stdout + result.stderr
else:
    with tarfile.open(source, {"Tar": "w", "TarGz": "w:gz", "TarXz": "w:xz"}[fmt], format=tarfile.USTAR_FORMAT) as archive:
        for name, data in old.items():
            header = tarfile.TarInfo(name)
            header.uid, header.gid, header.mtime = 123, 456, 1700000000
            header.uname, header.gname = "owner", "group"
            header.mode = 0o750 if data is None else 0o640
            header.type = tarfile.DIRTYPE if data is None else tarfile.REGTYPE
            header.size = len(data or b"")
            archive.addfile(header, io.BytesIO(data or b""))
original = source.read_bytes()

roots, children, payloads, source_files = [], {}, {}, {}
def register(name, data, path):
    source_id = str(uuid.uuid4())
    directory = isinstance(data, list)
    item = dict(id=source_id, name=name, directory=directory, size=0 if directory else len(data), dos_modified=None)
    if directory:
        path.mkdir(parents=True)
        children[source_id] = [register(n, d, path / n) for n, d in data]
    else:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        payloads[source_id] = data
        source_files[path] = data
    return item
for index, (name, data) in enumerate(inputs):
    roots.append(register(name, data, root / ("input-" + str(index)) / name))

op, token = str(uuid.uuid4()), str(uuid.uuid4())
binding = dict(native="v1:fixture", digest=list(hashlib.sha256(original).digest()), size=len(original), modified_unix_ns=source.stat().st_mtime_ns)
if scenario == "stale":
    binding["digest"][0] ^= 1
request = dict(schema=2, bulk_transfer=1, operation_id=op, op="start", operation={"Zip": "modify_zip", "SevenZ": "modify_7z"}.get(fmt, "modify_tar"), path=str(source), job_execution=True, password_interaction=True, conflict_policy="ask", job_modify=dict(browser=dict(snapshot_id=str(uuid.uuid4()), format=fmt, comment="", source=binding), action="add", ordinals=[], value="folder/"))
child = subprocess.Popen([str(worker), "--archive-worker"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
timer = threading.Timer(45, lambda: child.kill() if child.poll() is None else None)
timer.start()
counts = dict(prompts=0, source_open=0, source_close=0, begin=0, commit=0, abort=0)
wire, output, active = [], bytearray(), None
handed = False
peer = BulkPeer(request)

def send(value):
    child.stdin.write(peer.encode(value))
    child.stdin.flush()

def exact(count):
    data = b""
    while len(data) < count:
        part = child.stdout.read(count - len(data))
        assert part, "worker EOF"
        data += part
    return data

try:
    send(request)
    while True:
        frame = peer.read(exact)
        wire.append(frame)
        if frame["op"] in ("complete", "failure"):
            terminal = frame
            break
        if frame["op"] == "conflict_prompt":
            counts["prompts"] += 1
            assert counts["source_open"] == counts["begin"] == 0 and not output, "decision after payload"
            prompt = frame["conflict_prompt"]
            assert prompt["display"]["target"].startswith("folder/")
            assert prompt["incoming_bytes"] in (5, 6)
            # A stale or duplicate response cannot release a different ticket.
            ticket = prompt["ticket"]
            stale = dict(ticket, prompt_id=str(uuid.uuid4()))
            for reply_ticket, choice in [(stale, "overwrite"), (ticket, None if decision == "cancel" else decision), (ticket, "overwrite")]:
                send(dict(version=1, operation_id=op, op="conflict_reply", conflict_reply=dict(ticket=reply_ticket, decision=choice, apply_to_all=apply_all)))
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
            reply["read_data"] = json.dumps(roots)
        elif action == "source_next":
            items = children[call["token"]]
            reply["read_data"] = json.dumps(items[call["offset"]] if call["offset"] < len(items) else None)
        elif action == "source_check" and scenario == "source-change":
            reply.update(success=False, error_code="source_changed")
        elif action == "source_open":
            assert active is None
            assert call["token"] in payloads
            active = call["token"]
        elif action == "source_close":
            assert active == call["token"]
            active = None
        elif action == "source_read":
            assert active == call["token"]
            data = payloads[active]
            reply["read_data"] = base64.b64encode(data[call["offset"]:call["offset"] + call["length"]]).decode()
        elif action == "begin":
            reply["token"] = token
        elif action == "write":
            output.extend(call["data"])
        elif action == "read":
            data = bytearray(output[call["offset"]:call["offset"] + call["length"]])
            if scenario == "readback" and data:
                data[0] ^= 1
            reply["read_data"] = base64.b64encode(data).decode()
        elif action == "edit_reads_complete":
            assert active is None and counts["source_open"] == counts["source_close"]
            handed = True
        elif action == "commit":
            assert handed and counts["begin"] == counts["commit"] == 1
            if scenario == "commit-unknown":
                reply.update(success=False, error_code="native64")
        elif action not in ("source_acquire", "source_check", "edit_planning", "edit_reading", "edit_generating", "edit_reencoding", "edit_verifying", "creation_verifying", "source_precommit", "abort"):
            raise AssertionError(action)
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
result = dict(format=fmt, scenario=scenario, decision=decision, terminal=terminal, counts=counts)
(root / "result.json").write_text(json.dumps(result, indent=2))
assert source.read_bytes() == original, result
assert all(path.read_bytes() == data for path, data in source_files.items()), result
assert counts["prompts"] == responses, result
assert counts["source_open"] == counts["source_close"] == opens_expected, result
if failure:
    assert terminal["op"] == "failure" and terminal["error_code"] == failure, result
    assert counts["commit"] == (1 if scenario == "commit-unknown" else 0), result
    assert counts["begin"] == (1 if scenario in ("readback", "commit-unknown") else 0), result
    if scenario == "readback":
        assert counts["abort"] == 1, result
elif expected == old:
    assert terminal["op"] == "complete", result
    assert counts["begin"] == counts["commit"] == 0 and not output, result
else:
    assert terminal["op"] == "complete" and counts["begin"] == counts["commit"] == 1, result
    candidate = root / source.name.replace("owned", "candidate")
    candidate.write_bytes(output)
    if fmt == "Zip":
        with zipfile.ZipFile(candidate) as archive:
            assert archive.testzip() is None
            actual = {item.filename: None if item.is_dir() else archive.read(item) for item in archive.infolist()}
            assert len(actual) == len(archive.infolist())
    elif fmt == "SevenZ":
        listing = subprocess.run([oracle, "l", "-slt", "-ba", "-sccUTF-8", str(candidate)], capture_output=True, timeout=30)
        assert listing.returncode == 0, listing.stdout + listing.stderr
        actual = {}
        for block in listing.stdout.decode("utf-8-sig").replace("\r\n", "\n").strip().split("\n\n"):
            fields = dict(line.rstrip("\r").split(" = ", 1) for line in block.splitlines() if " = " in line)
            if "Path" not in fields:
                continue
            name = fields["Path"].replace("\\", "/")
            directory = fields.get("Folder") == "+" or "D" in fields.get("Attributes", "")
            name = name.rstrip("/") + ("/" if directory else "")
            assert name not in actual
            if directory:
                actual[name] = None
            else:
                extracted = subprocess.run([oracle, "x", "-so", "-spd", str(candidate), name], capture_output=True, timeout=30)
                assert extracted.returncode == 0
                actual[name] = extracted.stdout
    else:
        with tarfile.open(candidate) as archive:
            actual = {}
            for item in archive:
                name = item.name.rstrip("/") + ("/" if item.isdir() else "")
                assert name not in actual
                actual[name] = None if item.isdir() else archive.extractfile(item).read()
                if name in old:
                    assert (item.uid, item.gid, item.mtime, item.uname, item.gname) == (123, 456, 1700000000, "owner", "group")
    assert actual == expected, (result, actual, expected)
print("PASS", fmt, scenario, decision, counts)
