"""Test-only OPT-06 mixed-frame adapter for existing native-peer assertions."""
import base64
import json
import struct
import uuid


HEADER = struct.Struct("<BBH16s16s16sQQII")
assert HEADER.size == 76


def identity(value):
    result = uuid.UUID(value)
    assert result.int, "nonempty original UUID"
    return result


class BulkPeer:
    def __init__(self, request):
        assert request["schema"] == 2 and request["bulk_transfer"] == 1
        self.operation = identity(request["operation_id"])
        self.accepted = False
        self.pipelined_output = False
        self.sequence = 0
        self.pending = None
        self.written = {}

    def admit(self, call):
        assert self.accepted, "bulk capability precedes native effects"
        assert self.pending is None, "one acknowledged native call"
        identity(call["call_id"])
        if call["action"] in ("write", "source_read", "read"):
            identity(call["token"])
            self.sequence += 1
            assert call["sequence"] == self.sequence, "operation-wide positive sequence"
            assert 1 <= call["length"] <= 65536
            assert 0 <= call["offset"] < 1 << 64
        self.pending = call

    def read(self, exact):
        prefix, = struct.unpack("<I", exact(4))
        binary = bool(prefix & 0x80000000)
        size = prefix & 0x7fffffff
        assert (76 <= size <= 65612) if binary else (1 <= size <= 1048576)
        body = exact(size)
        if binary:
            version, kind, reserved, operation, call, token, sequence, offset, requested, actual = HEADER.unpack_from(body)
            assert (version, kind, reserved) == (1, 1, 0), "worker output direction"
            assert uuid.UUID(bytes=operation) == self.operation
            call, token = str(uuid.UUID(bytes=call)), str(uuid.UUID(bytes=token))
            assert requested == actual and size == 76 + actual, "full exact output write"
            assert self.written.get(token) == offset, "owned partial and write offset"
            native = dict(action="write", call_id=call, token=token, sequence=sequence,
                          offset=offset, length=requested, data=list(body[76:]))
            self.admit(native)
            return dict(version=1, operation_id=str(self.operation), op="native_call", native=native)
        frame = json.loads(body)
        assert identity(frame["operation_id"]) == self.operation
        if frame["op"] == "accepted":
            assert not self.accepted, "single admission"
            capabilities = frame["capabilities"]
            assert isinstance(capabilities, list) and all(isinstance(item, str) for item in capabilities)
            assert capabilities.count("binary_bulk_v1") == 1
            self.accepted = True
            self.pipelined_output = "extraction_output_v1" in capabilities
        elif frame["op"] == "native_call":
            assert frame["native"]["action"] != "write", "modern output must be raw"
            self.admit(frame["native"])
        return frame

    def encode(self, value):
        value = dict(value)
        assert identity(value["operation_id"]) == self.operation
        if value["op"] == "native_result":
            call, self.pending = self.pending, None
            assert call is not None and value["call_id"] == call["call_id"]
            action, success = call["action"], value["success"]
            if action == "begin" and success:
                token = str(identity(value["token"]))
                assert token not in self.written
                self.written[token] = 0
            elif action in ("commit", "abort") and success:
                self.written.pop(call["token"], None)
            elif action in ("write", "source_read", "read"):
                value.update((field, call[field]) for field in ("token", "sequence", "offset", "length"))
                if action == "write":
                    if success:
                        self.written[call["token"]] = call["offset"] + call["length"]
                    if self.pipelined_output:
                        return b""
                elif success:
                    payload = base64.b64decode(value["read_data"], validate=True)
                    assert len(payload) <= call["length"], "short read or EOF"
                    header = HEADER.pack(1, 2 if action == "source_read" else 3, 0,
                                         self.operation.bytes, identity(call["call_id"]).bytes,
                                         identity(call["token"]).bytes, call["sequence"],
                                         call["offset"], call["length"], len(payload))
                    return struct.pack("<I", 0x80000000 | (76 + len(payload))) + header + payload
                value.pop("read_data", None)
        body = json.dumps(value, separators=(",", ":")).encode()
        assert 1 <= len(body) <= 1048576
        return struct.pack("<I", len(body)) + body
