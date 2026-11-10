"""TAR-AC01 independent small-fixture byte oracle over the real worker protocol.

CPython creates/checks valid headers; raw expectations come from the original fixture,
not the editor's plan. No extraction or final-path publication; each exact owned child
has a 30-second watchdog. Used only by tar_edit_raw_header_contract.rs.
"""
import base64
import hashlib
import json
import os
from pathlib import Path
import struct
import subprocess
import sys
import tarfile
import threading
import uuid
from bulk_peer import BulkPeer


def pax_record(key, value, padded=False):
    body = (key + "=" + value + "\n").encode()
    width = 4 if padded else 1
    while True:
        length = len(body) + width + 1
        new_width = max(4 if padded else 1, len(str(length)))
        if new_width == width:
            return str(length).zfill(width).encode() + b" " + body
        width = new_width


def fields(data):
    result = {}
    at = 0
    while at < len(data):
        space = data.index(b" ", at)
        end = at + int(data[at:space])
        key, value = data[space + 1:end - 1].split(b"=", 1)
        result[key.decode()] = (value, data[at:end])
        at = end
    assert at == len(data)
    return result


def header(name, size, kind, variant):
    info = tarfile.TarInfo(name)
    info.type, info.size = kind, size
    info.uid, info.gid, info.mode, info.mtime = 123, 456, 0o640, 1700000000
    info.uname, info.gname = "owner", "group"
    raw = bytearray(info.tobuf(format=tarfile.GNU_FORMAT if variant == "binary" else tarfile.USTAR_FORMAT))
    # Valid distinct octal/base256 representations; checksum style independent of values.
    for offset, width, value in [(100, 8, 0o640), (108, 8, 123), (116, 8, 456), (124, 12, size)]:
        raw[offset:offset + width] = (b"\x80" + value.to_bytes(width - 1, "big")) if variant == "binary" else format(value, "o").rjust(width - 1, " ").encode() + b"\0"
    raw[148:156] = b" " * 8
    checksum = sum(raw)
    raw[148:156] = format(checksum, "06o").encode() + b"\0 " if variant == "posix" else format(checksum, "07o").encode() + b"\0"
    return bytes(raw)


def block(raw, data):
    return raw + data + bytes((-len(data)) % 512)


def seed(path, mode, variant):
    is_gnu = mode in ("gnu", "replace_gnu")
    old_name = "old/" + "segment/" * 16 + "file" if is_gnu else "old.txt"
    values = [("mtime", "1700000000.123456789"), ("atime", "1700000001.987654321")]
    if not is_gnu and mode != "replace_unrelated":
        values = [("path", old_name), ("size", "0000007")] + values
    data = b"".join(pax_record(key, value, padded=key != "path") for key, value in values)
    extensions = block(header("PaxHeader", len(data), tarfile.XHDTYPE, variant), data)
    if is_gnu:
        name_data = old_name.encode() + b"\0"
        extensions += block(header("././@LongLink", len(name_data), tarfile.GNUTYPE_LONGNAME, variant), name_data)
    main_name = "placeholder" if is_gnu or mode != "replace_unrelated" else old_name
    target = extensions + block(header(main_name, 7, tarfile.REGTYPE, variant), b"payload")
    untouched_data = pax_record("mtime", "1700000002.000000001", padded=True)
    untouched = block(header("OtherPax", len(untouched_data), tarfile.XHDTYPE, variant), untouched_data)
    untouched += block(header("untouched.txt", 4, tarfile.REGTYPE, variant), b"keep")
    path.write_bytes(target + untouched + bytes(1024))
    return old_name


def catalog(data):
    at, extensions, result = 0, [], {}
    while any(data[at:at + 512]):
        raw = data[at:at + 512]
        size = tarfile.nti(raw[124:136])
        kind = raw[156:157]
        if kind not in (tarfile.XHDTYPE, tarfile.GNUTYPE_LONGNAME):
            effective = next((int(fields(payload)["size"][0]) for _, payload in extensions if _[156:157] == tarfile.XHDTYPE and "size" in fields(payload)), size)
        else:
            effective = size
        payload = data[at + 512:at + 512 + effective]
        end = at + 512 + effective + (-effective) % 512
        assert len(raw) == 512 and len(payload) == effective
        if kind in (tarfile.XHDTYPE, tarfile.GNUTYPE_LONGNAME):
            extensions.append((raw, payload))
        else:
            name = tarfile.nts(raw[:100], "utf-8", "strict")
            for ext, body in extensions:
                if ext[156:157] == tarfile.GNUTYPE_LONGNAME:
                    name = body[:-1].decode()
            for ext, body in extensions:
                if ext[156:157] == tarfile.XHDTYPE and "path" in fields(body):
                    name = fields(body)["path"][0].decode()
            result[name] = dict(header=raw, extensions=extensions, content=payload)
            extensions = []
        at = end
    assert not extensions and data[at:] == bytes(len(data) - at) and len(data) - at >= 1024
    return result


def independent(path, expected):
    with tarfile.open(path) as archive:
        items = archive.getmembers()
        assert sorted(h.name for h in items) == sorted(expected)
        for item in items:
            assert archive.extractfile(item).read() == expected[item.name]
            assert (item.uid, item.gid, item.mode, item.uname, item.gname) == (123, 456, 0o640, "owner", "group")
            assert item.pax_headers["mtime"] == ("1700000002.000000001" if item.name == "untouched.txt" else "1700000000.123456789")
    bsdtar = Path(os.environ.get("WINDIR", r"C:\Windows")) / "System32" / "tar.exe"
    assert bsdtar.is_file(), "independent bsdtar is required, never silently skipped"
    listing = subprocess.run([str(bsdtar), "-tf", str(path)], capture_output=True, timeout=10, check=True)
    assert sorted(listing.stdout.decode("utf-8").splitlines()) == sorted(expected)


def run_worker(worker, source, candidate, edit, incoming):
    original = source.read_bytes()
    operation, token, source_id = (str(uuid.uuid4()) for _ in range(3))
    edit["browser"] = dict(snapshot_id=str(uuid.uuid4()), format="Tar", comment="", source=dict(native="v1:fixture", digest=list(hashlib.sha256(original).digest()), size=len(original), modified_unix_ns=source.stat().st_mtime_ns))
    request = dict(schema=2, bulk_transfer=1, operation_id=operation, op="start", operation="modify_tar", path=str(source), job_execution=True, password_interaction=True, job_modify=edit)
    child = subprocess.Popen([str(worker), "--archive-worker"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    timed_out = threading.Event()
    def expire():
        if child.poll() is None:
            timed_out.set()
            child.kill()
    timer = threading.Timer(30, expire)
    timer.start()
    wire, output = [], bytearray()
    opened = closed = commits = 0
    handed = False
    peer = BulkPeer(request)
    def send(value):
        child.stdin.write(peer.encode(value))
        child.stdin.flush()
    def exact(length):
        result = b""
        while len(result) < length:
            chunk = child.stdout.read(length - len(result))
            if not chunk:
                raise AssertionError("worker EOF / timeout=" + str(timed_out.is_set()))
            result += chunk
        return result
    try:
        send(request)
        while True:
            frame = peer.read(exact)
            wire.append(frame)
            if frame["op"] in ("complete", "failure"):
                terminal = frame
                break
            if frame["op"] != "native_call":
                continue
            call = frame["native"]
            action = call["action"]
            reply = dict(version=1, operation_id=operation, op="native_result", call_id=call["call_id"], success=True)
            if action == "source_stamp":
                reply["read_data"] = "v1:fixture"
            elif action == "source_roots":
                reply["read_data"] = json.dumps([dict(id=source_id, name="replacement", directory=False, size=len(incoming), dos_modified=None)])
            elif action == "source_open":
                opened += 1
            elif action == "source_close":
                closed += 1
            elif action == "source_read":
                reply["read_data"] = base64.b64encode(incoming[call["offset"]:call["offset"] + call["length"]]).decode()
            elif action == "begin":
                reply["token"] = token
            elif action == "write":
                output.extend(bytes(call["data"]))
            elif action == "read":
                reply["read_data"] = base64.b64encode(output[call["offset"]:call["offset"] + call["length"]]).decode()
            elif action == "edit_reads_complete":
                assert opened == closed
                handed = True
            elif action == "commit":
                assert handed
                commits += 1
            elif action not in ("source_acquire", "edit_planning", "edit_reading", "edit_reencoding", "edit_verifying", "creation_verifying", "source_precommit", "source_check", "abort"):
                raise AssertionError("unexpected native action " + action)
            send(reply)
        child.stdin.close()
        child.wait(timeout=10)
        assert child.returncode == 0 and not child.stderr.read() and not timed_out.is_set()
    finally:
        timer.cancel()
        if child.poll() is None:
            child.kill()
            child.wait(timeout=10)
        source.with_suffix(".wire.json").write_text(json.dumps(wire, indent=2))
    candidate.write_bytes(output)
    assert terminal["op"] == "complete", terminal
    assert commits == 1 and handed and opened == closed
    assert source.read_bytes() == original
    return terminal


def exercise(worker, root, mode):
    failures, results = [], []
    for variant in ("posix", "binary"):
        lengths = ("same", "different") if mode.startswith("replace") else ("same", "longer")
        for length in lengths:
            folder = root / (variant + "-" + length)
            folder.mkdir()
            source, candidate = folder / "owned.tar", folder / "candidate.tar"
            old_name = seed(source, mode, variant)
            independent(source, {old_name: b"payload", "untouched.txt": b"keep"})
            incoming = b"replace" if length == "same" else b"replacement-content"
            new_name = ("new/" + "segment/" * 16 + "file") if mode == "gnu" else "new.txt"
            if length == "longer":
                new_name += ".longer"
            replacing = mode.startswith("replace")
            edit = dict(action="replace_content" if replacing else "rename", ordinals=[0], value=None if replacing else new_name)
            terminal = run_worker(worker, source, candidate, edit, incoming)
            target_name = old_name if replacing else new_name
            independent(candidate, {target_name: incoming if replacing else b"payload", "untouched.txt": b"keep"})
            old, new = catalog(source.read_bytes()), catalog(candidate.read_bytes())
            before, after = old[old_name], new[target_name]
            errors = []
            if old["untouched.txt"] != new["untouched.txt"]:
                errors.append("unmodified record raw bytes changed")
            main_before, main_after = before["header"], after["header"]
            if not replacing or length == "same":
                if main_before != main_after:
                    errors.append("untouched main header normalized")
            else:
                if main_before[:124] != main_after[:124] or main_before[136:148] != main_after[136:148] or main_before[156:] != main_after[156:]:
                    errors.append("unrelated main header fields changed")
                if tarfile.nti(main_after[124:136]) != len(incoming):
                    errors.append("required main size not updated")
                if main_before[148:156] == main_after[148:156]:
                    errors.append("required main checksum not updated")
            assert len(before["extensions"]) == len(after["extensions"])
            for (old_header, old_data), (new_header, new_data) in zip(before["extensions"], after["extensions"]):
                if len(old_data) == len(new_data):
                    if old_header != new_header:
                        errors.append("same-length extension header normalized")
                elif old_header[:124] != new_header[:124] or old_header[136:148] != new_header[136:148] or old_header[156:] != new_header[156:]:
                    errors.append("unrelated extension header fields changed")
                if old_header[156:157] == tarfile.XHDTYPE:
                    original_fields, candidate_fields = fields(old_data), fields(new_data)
                    assert original_fields.keys() == candidate_fields.keys()
                    changed_key = "size" if replacing and length != "same" else "path" if not replacing else None
                    for key, (_, original_record) in original_fields.items():
                        if key != changed_key and candidate_fields[key][1] != original_record:
                            errors.append("unaffected PAX record normalized: " + key)
                    if changed_key not in original_fields and old_data != new_data:
                        errors.append("unaffected PAX extension payload changed")
                elif replacing and old_data != new_data:
                    errors.append("unaffected GNU extension payload changed")
            record = dict(mode=mode, variant=variant, length=length, terminal=terminal["op"], independent="CPython+bsdtar PASS", source_unchanged=True, original_main_checksum=main_before[148:156].hex(), candidate_main_checksum=main_after[148:156].hex(), failures=errors)
            (folder / "result.json").write_text(json.dumps(record, indent=2))
            results.append(record)
            failures.extend(errors)
    (root / "results.json").write_text(json.dumps(results, indent=2))
    print(json.dumps(results, indent=2))
    assert not failures, "TAR-AC01 raw fidelity: " + "; ".join(sorted(set(failures)))


if __name__ == "__main__":
    exercise(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3])
