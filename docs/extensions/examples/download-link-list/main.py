"""Read selected original task URLs, then issue exactly one explicit copy request."""
from urllib.parse import quote, urlsplit
from protocol import Failure, require, serve, text_size, uuid_ok


def safe_url(value):
    text_size(value)
    require(not any(ord(c) < 33 or 127 <= ord(c) <= 159 or c.isspace() for c in value))
    require("\\" not in value)
    try:
        parsed = urlsplit(value)
        require(parsed.scheme in ("http", "https") and bool(parsed.hostname))
        require(parsed.username is None and parsed.password is None)
        require(parsed.port is None or 0 < parsed.port <= 65535)
    except ValueError:
        raise Failure("invalid") from None
    return value


def execute(frame, session):
    require(not frame.get("settings", {}), "settings")
    ids = frame["context"].get("task_ids")
    require(isinstance(ids, list) and 1 <= len(ids) <= 32)
    require(all(uuid_ok(i) for i in ids))
    require(len(set(i.lower() for i in ids)) == len(ids))
    urls = []
    for index, task_id in enumerate(ids, 1):
        value = session.call(f"get-{index}", "downloads.get", {"task_id": task_id})
        require(isinstance(value, dict) and value.get("task_id") == task_id)
        request = value.get("request")
        require(isinstance(request, dict))
        # task_snapshot.request is original; resolution may contain a rewritten
        # credentialed request and is intentionally never consulted or emitted.
        urls.append(safe_url(request.get("source_url")))
        text_size("\n".join(urls))
    if frame["handler_id"] == "copy-markdown":
        # Fixed labels avoid title/filename injection. Percent-encode Markdown/
        # HTML punctuation in destinations; preserve existing URL %-escapes.
        escaped = [quote(url, safe=":/?#@!$&'*+,;=%~._-") for url in urls]
        lines = [f"- [下载 {i}](<{url}>)" for i, url in enumerate(escaped, 1)]
        text = "\n".join(lines)
    else:
        text = "\n".join(urls)
    text_size(text)
    session.call("copy", "common.copy", {"text": text})  # No paths, cause, retry or task mutations.
    return "success", {"text": "已复制所选任务的原始链接。分享前请检查链接是否包含敏感参数。"}


if __name__ == "__main__":
    raise SystemExit(serve("example.download-link-list", "downloads", {"copy-lines", "copy-markdown"}, execute))
