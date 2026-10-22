"""Explicit selected-text transforms; preserve originals, never copy/paste."""
import json
import re
from urllib.parse import unquote
from protocol import Failure, item_id, require, serve, strict_json, text_size


def bounded_json_depth(text):
    # Scan before json.loads to cap nesting independently of Python's recursion
    # limit. Brackets inside quoted strings (including escaped quotes) are data.
    depth, quoted, escaped = 0, False, False
    for char in text:
        if quoted:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                quoted = False
        elif char == '"':
            quoted = True
        elif char in "[{":
            depth += 1
            require(depth <= 32)
        elif char in "]}":
            depth -= 1
            require(depth >= 0)
    require(depth == 0 and not quoted)


def transform(text, handler):
    if handler == "unique-lines":
        # CRLF/CR -> LF only. No trim, sorting or Unicode normalization; exact
        # duplicate lines keep their first occurrence (including a blank line).
        lines = text.replace("\r\n", "\n").replace("\r", "\n").split("\n")
        return "\n".join(dict.fromkeys(lines))
    if handler == "pretty-json":
        bounded_json_depth(text)
        value = strict_json(text)
        return json.dumps(value, ensure_ascii=False, allow_nan=False, indent=2)
    require(handler == "uri-decode")
    require(not re.search(r"%(?![0-9a-fA-F]{2})", text))
    try:
        decoded = unquote(text, encoding="utf-8", errors="strict")
    except UnicodeError:
        raise Failure("invalid") from None
    # Do not introduce C0/C1 controls or newline injection. '+' is literal;
    # percent decoding happens once, never recursively and never opens a URL.
    require(not any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in decoded))
    return decoded


def execute(frame, session):
    require(not frame.get("settings", {}), "settings")
    selection = frame["context"].get("selection")
    require(isinstance(selection, dict) and item_id(selection.get("item_id")))
    if selection.get("kind") in ("image", "files"):
        return "not_handled", {"text": "仅处理明确选中的文本条目。"}
    require(selection.get("kind") == "text")
    original = text_size(selection.get("text"))
    if not original:
        return "not_handled", {"text": "文本为空，未创建条目。"}
    changed = text_size(transform(original, frame["handler_id"]))
    if not changed.strip() or changed == original:
        return "not_handled", {"text": "结果为空或文本未变化，未创建条目。"}
    value = session.call("derive", "clipboard.create_derived", {"source_item_id": selection["item_id"], "text": changed})
    require(isinstance(value, dict) and item_id(value.get("item_id")))
    require(value["item_id"] != selection["item_id"])
    return "success", {"text": "已创建派生文本，原条目保留；未复制或粘贴。"}


if __name__ == "__main__":
    raise SystemExit(serve("example.clipboard-text-tools", "clipboard", {"unique-lines", "pretty-json", "uri-decode"}, execute))
