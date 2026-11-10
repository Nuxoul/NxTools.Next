"""Actual private-pipe peer with independent inputs; native publication is simulated."""
import io
import json
import queue
import struct
import subprocess
import sys
import threading
import time
import uuid
import zipfile
from pathlib import Path

worker, root, scenario = Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3]
operation, token, source_id = uuid.uuid4(), uuid.uuid4(), uuid.uuid4()
payload = bytes((index * 37 + 19) % 256 for index in range(131091))
source = root / "independent.zip"
with zipfile.ZipFile(source, "w", compression=zipfile.ZIP_STORED) as archive:
    archive.writestr("payload.bin", payload)
original = source.read_bytes()
is_create = scenario.startswith(("create", "raw_", "ack_", "cancel_source", "eof_source", "cancel_partial", "eof_partial", "json_read")) or scenario in ("pause", "cancel_write", "eof_write", "duplicate")
request = dict(schema=2, operation_id=str(operation), op="start", operation="extract", path=str(source),
               bulk_transfer=1, extract_options=dict(copy_motw=False))
if is_create:
    request = dict(schema=2, operation_id=str(operation), op="start", operation="create_zip", path="id-source",
                   bulk_transfer=1, create_name="created.zip", compression_level="store",
                   zip_options=dict(method="store", encryption="none", strip_single_root=False, exclude_patterns=[]))
if scenario == "missing_negotiation":
    del request["bulk_transfer"]
elif scenario == "unsupported_negotiation":
    request["bulk_transfer"] = 2
elif scenario == "malformed_negotiation":
    request["bulk_transfer"] = "1"
child = subprocess.Popen([str(worker), "--archive-worker"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
timer = threading.Timer(30, lambda: child.kill() if child.poll() is None else None)
timer.start()
records, output = [], bytearray()
frames = queue.Queue(maxsize=2)
sequence = commits = source_bytes = partial_bytes = 0
source_eof = partial_eof = 0
terminal = None
injected = False
previous_ack = None


def exact(length):
    value = bytearray()
    while len(value) < length:
        part = child.stdout.read(length - len(value))
        if not part:
            raise EOFError("worker did not provide negotiated binary frame")
        value.extend(part)
    return bytes(value)


def reader():
    try:
        while True:
            prefix = struct.unpack("<I", exact(4))[0]
            size = prefix & 0x7fffffff
            assert 76 <= size <= 65612 if prefix & 0x80000000 else 1 <= size <= 1048576
            body = exact(size)
            if not prefix & 0x80000000:
                frames.put(json.loads(body))
                continue
            version, kind, reserved = struct.unpack_from("<BBH", body)
            op_id, call, actual_token = (uuid.UUID(bytes=body[start:start+16]) for start in (4, 20, 36))
            seq, offset, requested, actual = struct.unpack_from("<QQII", body, 52)
            assert (version, kind, reserved, op_id, actual_token) == (1, 1, 0, operation, token)
            assert call.int and 1 <= actual == requested <= 65536 and size == 76 + actual
            frames.put(dict(op="binary", call_id=str(call), token=str(actual_token), sequence=seq,
                            offset=offset, length=requested, payload=body[76:]))
    except BaseException as failure:
        frames.put(failure)


def send(value):
    body = json.dumps(value, separators=(",", ":")).encode()
    send_bytes(struct.pack("<I", len(body)) + body)


def send_bytes(value):
    if scenario == "create_short":
        # Prefix/header/payload split across short pipe deliveries, with no extra frame.
        for start, end in ((0, 1), (1, 3), (3, 9), (9, 51), (51, 79), (79, len(value))):
            child.stdin.write(value[start:end])
            child.stdin.flush()
    else:
        child.stdin.write(value)
        child.stdin.flush()


def control(name):
    send(dict(version=1, operation_id=str(operation), op=name))


def next_sequence(value):
    global sequence
    assert value == sequence + 1, (value, sequence)
    sequence = value


def read_reply(call, data, kind):
    header = bytearray(struct.pack("<BBH", 1, kind, 0) + operation.bytes + uuid.UUID(call["call_id"]).bytes
                       + uuid.UUID(call["token"]).bytes
                       + struct.pack("<QQII", call["sequence"], call["offset"], call["length"], len(data)))
    if scenario.startswith("raw_") and not injected:
        field = scenario[4:]
        if field in ("operation", "call", "token"):
            start = dict(operation=4, call=20, token=36)[field]
            header[start:start+16] = uuid.uuid4().bytes
        elif field == "sequence":
            struct.pack_into("<Q", header, 52, call["sequence"] + 1)
        elif field == "offset":
            struct.pack_into("<Q", header, 60, call["offset"] + 1)
        elif field == "length":
            struct.pack_into("<I", header, 68, call["length"] - 1)
        elif field == "zero_sequence":
            struct.pack_into("<Q", header, 52, 0)
        elif field == "oversize":
            struct.pack_into("<I", header, 68, 65537)
        elif field == "actual":
            struct.pack_into("<I", header, 72, call["length"] + 1)
        elif field == "version":
            header[0] = 2
        elif field == "reserved":
            header[2] = 1
        elif field == "kind":
            header[1] = 3 if kind == 2 else 2
        elif field == "direction":
            header[1] = 1
        elif field == "nil_token":
            header[36:52] = bytes(16)
        elif field != "truncated":
            raise AssertionError(field)
        if field == "truncated":
            send_bytes(struct.pack("<I", 0x80000000 | (76 + len(data))) + header[:35])
            child.stdin.close()
            return
    send_bytes(struct.pack("<I", 0x80000000 | (76 + len(data))) + header + data)


def withhold():
    # Ignore progress only; no later data/native authority may overtake this ACK.
    deadline = time.monotonic() + 0.15
    while time.monotonic() < deadline:
        try:
            value = frames.get(timeout=max(0.001, deadline - time.monotonic()))
        except queue.Empty:
            return
        assert isinstance(value, dict) and value["op"] == "progress", value
        records.append(value)


try:
    threading.Thread(target=reader, daemon=True).start()
    send(request)
    while True:
        frame = frames.get(timeout=10)
        if isinstance(frame, EOFError) and scenario.endswith("negotiation"):
            break
        if isinstance(frame, BaseException):
            raise frame
        if frame["op"] == "binary":
            next_sequence(frame["sequence"])
            assert frame["offset"] == len(output)
            output.extend(frame.pop("payload"))
            records.append(dict(frame))
            if not is_create:
                continue  # Ordinary extraction drains at close, without output-record ACKs.
            reply = dict(version=1, operation_id=str(operation), op="native_result", call_id=frame["call_id"],
                         success=True, token=str(token), sequence=sequence, offset=frame["offset"], length=frame["length"])
            if not injected and scenario == "pause":
                control("pause")
                withhold()
                send(reply)
                withhold()
                control("resume")
                injected = True
                continue
            if not injected and scenario in ("cancel_write", "eof_write"):
                if scenario == "cancel_write":
                    control("cancel")
                else:
                    child.stdin.close()
                injected = True
                continue
            if not injected and scenario.startswith("ack_"):
                field = scenario[4:]
                if field in ("operation", "call", "token"):
                    reply[dict(operation="operation_id", call="call_id", token="token")[field]] = str(uuid.uuid4())
                else:
                    reply[field] += 1
                injected = True
            if scenario == "duplicate" and previous_ack is not None and not injected:
                send(previous_ack)
                injected = True
                continue
            send(reply)
            previous_ack = reply
            continue
        records.append(frame)
        assert frame["operation_id"] == str(operation)
        if frame["op"] == "accepted":
            assert frame.get("capabilities", []).count("binary_bulk_v1") == 1
            assert ("extraction_output_v1" in frame.get("capabilities", [])) == (not is_create)
            continue
        if frame["op"] in ("complete", "failure"):
            terminal = frame
            break
        if frame["op"] != "native_call":
            continue
        call = frame["native"]
        action = call["action"]
        assert action != "write", "modern payload must not use decimal JSON"
        reply = dict(version=1, operation_id=str(operation), op="native_result", call_id=call["call_id"], success=True)
        if action == "begin":
            assert not output and call["path"] == ("created.zip" if is_create else "payload.bin")
            reply["token"] = str(token)
        elif action == "source_roots":
            reply["read_data"] = json.dumps([dict(id=str(source_id), name="payload.bin", directory=False,
                                                   size=len(payload), dos_modified=None)])
        elif action in ("source_read", "read"):
            next_sequence(call["sequence"])
            kind = 2 if action == "source_read" else 3
            value = payload if kind == 2 else output
            assert call["token"] == str(source_id if kind == 2 else token)
            assert 1 <= call["length"] <= 65536
            offset = call["offset"]
            count = min(call["length"], 12347 if kind == 2 else 8191)
            data = bytes(value[offset:offset+count])
            if kind == 2:
                source_bytes += len(data)
                source_eof += not data
            else:
                partial_bytes += len(data)
                partial_eof += not data
            stop = ("source" if kind == 2 else "partial")
            if not injected and scenario in ("cancel_" + stop, "eof_" + stop):
                if scenario.startswith("cancel"):
                    control("cancel")
                else:
                    child.stdin.close()
                injected = True
                continue
            if scenario == "json_read" and not injected:
                reply.update(token=call["token"], sequence=call["sequence"], offset=offset, length=call["length"], read_data="")
                send(reply)
            else:
                read_reply(call, data, kind)
            if scenario.startswith("raw_") or scenario == "json_read":
                injected = True
            continue
        elif action == "commit":
            assert not is_create or source_eof == partial_eof == 1
            assert call["token"] == str(token)
            commits += 1
        elif action == "source_close":
            assert source_eof == 1 or injected
        elif action not in ("source_acquire", "source_validated", "codec_opened", "source_decoding", "source_precommit",
                            "file_metadata", "directory_metadata", "abort", "creation_planning", "creation_encoding",
                            "creation_verifying", "source_open", "source_check"):
            raise AssertionError("unexpected native action: " + action)
        send(reply)
    if not child.stdin.closed:
        child.stdin.close()
    child.wait(timeout=10)
    if scenario.endswith("negotiation"):
        assert child.returncode != 0 and not records and commits == 0
    else:
        assert child.returncode == 0 and not child.stderr.read()
        rejected = scenario.startswith(("raw_", "ack_", "cancel_", "eof_")) or scenario in ("duplicate", "json_read")
        assert terminal["op"] == ("failure" if rejected else "complete"), terminal
        assert commits == (0 if rejected else 1)
        if rejected:
            assert injected
            expected = "Cancelled" if scenario.startswith(("cancel_", "eof_", "ack_")) or scenario == "duplicate" else "IoFailure"
            assert terminal["error_code"] == expected, terminal
        elif is_create:
            assert source_bytes == len(payload) and partial_bytes == len(output)
            assert source_eof == partial_eof == 1
            with zipfile.ZipFile(io.BytesIO(output)) as archive:
                assert archive.namelist() == ["payload.bin"] and archive.read("payload.bin") == payload
        else:
            assert output == payload and sequence == 5
            assert [item["length"] for item in records if item["op"] == "binary"] == [32768] * 4 + [19]
        if scenario == "pause":
            assert injected
    assert source.read_bytes() == original
finally:
    timer.cancel()
    if child.poll() is None:
        child.kill()
        child.wait(timeout=10)
    (root / "frames.json").write_text(json.dumps(records, indent=2))
    (root / "result.json").write_text(json.dumps(dict(scenario=scenario, terminal=terminal, bulk_records=sequence,
        output_bytes=len(output), source_bytes=source_bytes, partial_bytes=partial_bytes, commits=commits,
        source_unchanged=source.read_bytes() == original), indent=2))
print("PASS", scenario, "bulk_records", sequence, "output_bytes", len(output), "commits", commits)
