"""Public GitHub prefix rewrite only. This module performs no HTTP or credential calls."""
import ipaddress
import re
from urllib.parse import urlsplit
from protocol import Failure, require, serve, text_size

DEFAULT_PREFIX = "https://ghproxy.net/"  # Example only; not an uptime/speed endorsement.
SEGMENT = re.compile(r"[A-Za-z0-9_~.-]+\Z")
LABEL = re.compile(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\Z")
UPSTREAMS = {"github.com", "raw.githubusercontent.com"}


def parsed_url(value):
    require(isinstance(value, str) and len(value) <= 16384)
    # Deliberately reject ALL percent escapes, Unicode path spellings, and empty
    # delimiters too: never decode/normalize an ambiguous path before wrapping.
    require(not any(ord(c) <= 32 or ord(c) >= 127 for c in value))
    require(not any(c in value for c in "\\%?#"))
    try:
        parsed = urlsplit(value)
        require(parsed.scheme == "https" and value.startswith("https://"))
        require(bool(parsed.hostname) and parsed.username is None and parsed.password is None)
        require(parsed.port in (None, 443))
        require(parsed.netloc.lower() in (parsed.hostname, parsed.hostname + ":443"))
    except ValueError:
        raise Failure("invalid") from None
    return parsed


def path_segments(path, trailing=False):
    require(path.startswith("/"))
    body = path[1:]
    if trailing:
        require(body == "" or body.endswith("/"))
        body = body[:-1] if body else ""
    segments = body.split("/") if body else []
    require(all(SEGMENT.fullmatch(s) and s not in (".", "..") for s in segments))
    return segments


def public_source(url):
    try:
        parsed = parsed_url(url)
        require(parsed.hostname in UPSTREAMS)
        parts = path_segments(parsed.path)
        require(len(parts) >= 4)
        if parsed.hostname == "github.com":
            require((len(parts) == 6 and parts[2:4] == ["releases", "download"])
                    or (len(parts) >= 4 and parts[2] == "archive"))
        return True
    except Failure:
        return False


def proxy_prefix(prefix):
    try:
        parsed = parsed_url(prefix)
        hostname = parsed.hostname
        require(hostname not in UPSTREAMS and len(hostname) <= 253)
        # No DNS/network lookup. Reject local names, IP literals and numeric
        # browser IPv4 spellings; a domain's DNS/operator is still user-trusted.
        try:
            ipaddress.ip_address(hostname)
        except ValueError:
            pass
        else:
            raise Failure("settings")
        labels = hostname.split(".")
        require(len(labels) >= 2 and all(LABEL.fullmatch(s) for s in labels))
        require(labels[-1].isalpha() and len(labels[-1]) >= 2)
        require(labels[-1].lower() not in {"localhost", "local", "internal", "lan", "home"})
        path_segments(parsed.path or "/", trailing=True)
        return prefix if parsed.path else prefix + "/"
    except Failure:
        raise Failure("settings") from None


def execute(frame, _session):
    options = frame.get("settings", {})
    require(not options.keys() - {"third_party_opt_in", "proxy_prefix"}, "settings")
    opt_in = options.get("third_party_opt_in", False)
    require(type(opt_in) is bool, "settings")
    if not opt_in:
        return "not_handled", {"text": "未启用第三方镜像：请先阅读风险并明确同意，继续使用原地址。"}
    prefix = proxy_prefix(options.get("proxy_prefix", DEFAULT_PREFIX))
    request = frame["context"].get("request")
    require(isinstance(request, dict))
    # Presence of even empty cookie/referrer is rejected, never forwarded.
    if request.get("headers", []) != [] or request.get("cookie") is not None or request.get("referrer") is not None:
        return "not_handled", {"text": "仅支持无请求头、Cookie、来源信息的公开链接；未转发凭据。"}
    source = request.get("source_url")  # Current effective chain input, not original_url.
    if not public_source(source):
        return "not_handled", {"text": "不是支持的无凭据公开 GitHub 下载路径；未转换。"}
    rewritten = text_size(prefix + source)
    return "handled", {"url": rewritten, "headers": [], "cookie": None, "referrer": None, "valid_for_ms": 60000}


if __name__ == "__main__":
    raise SystemExit(serve("example.github-public-proxy", "downloads", {"resolve"}, execute))
