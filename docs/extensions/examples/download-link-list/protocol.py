"""Package-local bounded JSONL v1 transport; no logging or external dependencies."""
import json
import math
import sys
from uuid import UUID

MAX_FRAME = 256 * 1024
MAX_TEXT = 64 * 1024
INVOKE_KEYS = {"protocol_version", "op", "run_id", "correlation_id", "generation", "package_id", "handler_id", "content_hash", "feature", "context", "settings", "causality"}
MESSAGES = {
    "invalid": "输入或协议无效，操作未确认完成。",
    "large": "内容超出大小上限，操作未确认完成。",
    "cancel": "操作已取消；已发出的效果可能已完成，不会重试。",
    "host": "宿主操作失败或连接已结束；不会重试。",
    "settings": "设置无效，请检查设置后重试。",
}


class Failure(Exception):
    pass


def require(condition, code="invalid"):
    if not condition:
        raise Failure(code)


def uuid_ok(value):
    try:
        return isinstance(value, str) and str(UUID(value)) == value.lower() and UUID(value).int != 0
    except (ValueError, AttributeError):
        return False


def item_id(value):
    return type(value) is int and 0 < value < (1 << 63)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result)
        result[key] = value
    return result


def reject_constant(_value):
    raise Failure("invalid")


def finite_float(value):
    result = float(value)
    require(math.isfinite(result))
    return result


def strict_json(text):
    try:
        return json.loads(text, object_pairs_hook=unique_object, parse_constant=reject_constant, parse_float=finite_float)
    except (ValueError, UnicodeError, RecursionError):
        raise Failure("invalid") from None


def text_size(text):
    require(isinstance(text, str))
    try:
        require(len(text.encode("utf-8")) <= MAX_TEXT, "large")
    except UnicodeError:
        raise Failure("invalid") from None
    return text


class Session:
    def __init__(self, reader, writer):
        self.reader, self.writer, self.run = reader, writer, None
        self.calls = 0

    def read(self):
        raw = self.reader.readline(MAX_FRAME + 1)
        require(bool(raw), "host")
        require(len(raw) <= MAX_FRAME, "large")
        require(raw.endswith(b"\n"))
        try:
            value = strict_json(raw.decode("utf-8"))
        except UnicodeError:
            raise Failure("invalid") from None
        require(isinstance(value, dict))
        return value

    def envelope(self, frame):
        require(type(frame.get("protocol_version")) is int and frame["protocol_version"] == 1)
        require(frame.get("run_id") == self.run)
        if frame.get("op") == "cancel":
            require(not frame.keys() - {"protocol_version", "op", "run_id"})
            raise Failure("cancel")

    def send(self, op, **fields):
        frame = {"protocol_version": 1, "op": op, "run_id": self.run, **fields}
        raw = (json.dumps(frame, ensure_ascii=False, allow_nan=False, separators=(",", ":")) + "\n").encode("utf-8")
        require(len(raw) <= MAX_FRAME, "large")
        self.writer.write(raw)
        self.writer.flush()

    def call(self, call_id, method, params):
        self.calls += 1
        require(self.calls <= 33)
        self.send("host_call", call_id=call_id, method=method, params=params)
        reply = self.read()
        self.envelope(reply)
        require(not reply.keys() - {"protocol_version", "op", "run_id", "call_id", "success", "value"})
        require(reply.get("op") == "host_result" and reply.get("call_id") == call_id)
        require(type(reply.get("success")) is bool and "value" in reply)
        require(reply["success"], "host")
        return reply["value"]


def serve(package, feature, handlers, execute):
    session = Session(sys.stdin.buffer, sys.stdout.buffer)
    try:
        frame = session.read()
        require(uuid_ok(frame.get("run_id")))
        session.run = frame["run_id"]  # Never generate or normalize the host ID.
        session.envelope(frame)
        require(not frame.keys() - INVOKE_KEYS)
        require(frame.get("op") == "invoke" and frame.get("package_id") == package)
        require(frame.get("feature") == feature and frame.get("handler_id") in handlers)
        require(isinstance(frame.get("context"), dict))
        require(isinstance(frame.get("settings", {}), dict), "settings")
        status, value = execute(frame, session)
    except Failure as failure:
        if session.run is None:
            return 1  # No safe run identity: exit silently; stdout stays protocol-only.
        status, value = "error", {"text": MESSAGES.get(str(failure), MESSAGES["invalid"])}
    except Exception:
        if session.run is None:
            return 1
        status, value = "error", {"text": MESSAGES["invalid"]}
    try:
        session.send("complete", status=status, value=value)
    except Exception:
        return 1  # Broken pipe/output failure is never retried or logged.
    return 0
