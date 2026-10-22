"""One-shot NxTools JSONL v1 action; Python 3.10+, standard library only."""

import json
import sys
from uuid import UUID

PACKAGE_ID = "example.clipboard-cleaner"
CALL_ID = "create-derived"
MAX_TEXT_BYTES = 64 * 1024
MAX_FRAME_BYTES = 256 * 1024
DEFAULT_OPTIONS = {
    "trim_line_ends": True,
    "collapse_blank_lines": True,
    "trim_outer": True,
}
MESSAGES = {
    "invalid_input": "输入无效，未继续整理。",
    "invalid_settings": "设置必须为布尔值，未继续整理。",
    "text_too_large": "文本超过 64 KiB，未创建条目。",
    "frame_too_large": "协议帧超过上限，未继续操作。",
    "protocol_error": "协议错误，操作未确认完成。",
    "host_failed": "宿主未能创建派生条目。",
    "eof": "宿主连接已结束，操作未确认完成。",
    "cancelled": "操作已取消，未确认创建结果。",
    "internal_error": "扩展执行失败，操作未确认完成。",
}


class ActionError(Exception):
    """Only fixed, payload-free categories may cross an error boundary."""

    def __init__(self, category):
        super().__init__(category)
        self.category = category


def parse_options(options):
    if not isinstance(options, dict) or options.keys() - DEFAULT_OPTIONS.keys():
        raise ActionError("invalid_settings")
    result = DEFAULT_OPTIONS.copy()
    for key, value in options.items():
        if type(value) is not bool:
            raise ActionError("invalid_settings")
        result[key] = value
    return result


def clean_text(text, options):
    """Pure transform; no IO, mutation of arguments, or Unicode normalization.

    Only CRLF/CR are line separators here. A blank line is empty or entirely
    Unicode whitespace. Collapsing retains the first blank line of each run.
    Line-end trimming removes ASCII spaces/tabs only; outer trimming uses strip.
    """
    if not isinstance(text, str):
        raise ActionError("invalid_input")
    resolved = parse_options(options)
    lines = text.replace("\r\n", "\n").replace("\r", "\n").split("\n")
    if resolved["trim_line_ends"]:
        lines = [line.rstrip(" \t") for line in lines]
    if resolved["collapse_blank_lines"]:
        kept = []
        previous_blank = False
        for line in lines:
            blank = not line.strip()
            if not (blank and previous_blank):
                kept.append(line)
            previous_blank = blank
        lines = kept
    result = "\n".join(lines)
    return result.strip() if resolved["trim_outer"] else result


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ActionError("protocol_error")
        result[key] = value
    return result


def reject_constant(_value):
    raise ActionError("protocol_error")


def read_frame(stream):
    # Binary IO avoids Windows locale encoding and newline translation.
    raw = stream.readline(MAX_FRAME_BYTES + 1)
    if not raw:
        raise ActionError("eof")
    if len(raw) > MAX_FRAME_BYTES:
        raise ActionError("frame_too_large")
    if not raw.endswith(b"\n"):
        raise ActionError("protocol_error")
    try:
        frame = json.loads(
            raw.decode("utf-8"),
            object_pairs_hook=unique_object,
            parse_constant=reject_constant,
        )
    except (UnicodeError, ValueError, RecursionError):
        raise ActionError("protocol_error") from None
    if not isinstance(frame, dict):
        raise ActionError("protocol_error")
    return frame


def valid_run_id(value):
    if not isinstance(value, str):
        return False
    try:
        return str(UUID(value)) == value.lower() and UUID(value).int != 0
    except ValueError:
        return False


def validate_envelope(frame, run_id):
    if (
        type(frame.get("protocol_version")) is not int
        or frame["protocol_version"] != 1
        or frame.get("run_id") != run_id
    ):
        raise ActionError("protocol_error")
    if frame.get("op") == "cancel":
        raise ActionError("cancelled")


def validate_invoke(frame, run_id):
    validate_envelope(frame, run_id)
    for key, expected in (
        ("op", "invoke"),
        ("package_id", PACKAGE_ID),
        ("handler_id", "clean"),
        ("feature", "clipboard"),
    ):
        if frame.get(key) != expected:
            raise ActionError("protocol_error")
    # entry is a manifest field, not a required invoke field. Causality remains
    # host-owned; it is deliberately neither reconstructed nor sent in params.
    options = parse_options(frame.get("settings", {}))
    context = frame.get("context")
    selection = context.get("selection") if isinstance(context, dict) else None
    if not isinstance(selection, dict):
        raise ActionError("invalid_input")
    item_id = selection.get("item_id")
    if type(item_id) is not int or not 0 < item_id <= (1 << 63) - 1:
        raise ActionError("invalid_input")
    if selection.get("kind") not in ("text", "files", "image"):
        raise ActionError("invalid_input")
    if selection["kind"] == "text" and not isinstance(selection.get("text"), str):
        raise ActionError("invalid_input")
    return selection, options


def validate_text_size(text):
    try:
        size = len(text.encode("utf-8"))
    except UnicodeError:
        raise ActionError("invalid_input") from None
    if size > MAX_TEXT_BYTES:
        raise ActionError("text_too_large")


def send_frame(stream, run_id, op, **fields):
    frame = {"protocol_version": 1, "op": op, "run_id": run_id, **fields}
    raw = (json.dumps(frame, ensure_ascii=False, separators=(",", ":")) + "\n").encode("utf-8")
    if len(raw) > MAX_FRAME_BYTES:
        raise ActionError("frame_too_large")
    stream.write(raw)
    stream.flush()


def execute(invoke, run_id, reader, writer):
    selection, options = validate_invoke(invoke, run_id)
    if selection["kind"] != "text":
        return "not_handled", "仅支持文本历史条目。"
    original = selection["text"]
    validate_text_size(original)
    cleaned = clean_text(original, options)
    validate_text_size(cleaned)
    if not cleaned.strip():
        return "not_handled", "整理后为空，未创建条目。"
    if cleaned == original:
        return "not_handled", "文本无需整理。"

    # Exactly one write request, never retried, even if its result is unknown.
    send_frame(
        writer, run_id, "host_call", call_id=CALL_ID,
        method="clipboard.create_derived",
        params={"source_item_id": selection["item_id"], "text": cleaned},
    )
    reply = read_frame(reader)
    validate_envelope(reply, run_id)
    if (
        reply.get("op") != "host_result"
        or reply.get("call_id") != CALL_ID
        or type(reply.get("success")) is not bool
    ):
        raise ActionError("protocol_error")
    if not reply["success"]:
        raise ActionError("host_failed")
    value = reply.get("value")
    item_id = value.get("item_id") if isinstance(value, dict) else None
    if type(item_id) is not int or not 0 < item_id <= (1 << 63) - 1:
        raise ActionError("protocol_error")
    return "success", "已生成整理后的文本，原内容保留。"


def main():
    reader, writer = sys.stdin.buffer, sys.stdout.buffer
    run_id = None
    try:
        invoke = read_frame(reader)
        if not valid_run_id(invoke.get("run_id")):
            raise ActionError("protocol_error")
        run_id = invoke["run_id"]  # Preserve spelling/case exactly; generate none.
        status, message = execute(invoke, run_id, reader, writer)
    except ActionError as failure:
        if run_id is None:
            sys.stderr.write(failure.category + "\n")
            sys.stderr.flush()
            return 1
        status, message = "error", MESSAGES[failure.category]
    except Exception:
        if run_id is None:
            sys.stderr.write("internal_error\n")
            sys.stderr.flush()
            return 1
        status, message = "error", MESSAGES["internal_error"]
    try:
        # One terminal emission point. A broken output pipe is never retried.
        send_frame(writer, run_id, "complete", status=status, value={"text": message})
    except Exception:
        sys.stderr.write("output_error\n")
        sys.stderr.flush()
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
