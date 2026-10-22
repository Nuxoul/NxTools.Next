"""Fake JSONL host; selected synthetic tasks only."""
import json
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parent
RUN = "11111111-1111-4111-8111-111111111111"
IDS = ["22222222-2222-4222-8222-222222222222", "33333333-3333-4333-8333-333333333333"]
URLS = ["https://example.org/中文?a=1&b=2", "https://example.org/a(b)[x]<z>"]


def invoke(handler="copy-lines", ids=None):
    return {"protocol_version": 1, "op": "invoke", "run_id": RUN,
            "package_id": "example.download-link-list", "handler_id": handler,
            "feature": "downloads", "settings": {}, "context": {"task_ids": IDS if ids is None else ids}}


def reply(call, value=None, **changes):
    return {"protocol_version": 1, "op": "host_result", "run_id": RUN,
            "call_id": call, "success": True, "value": value, **changes}


def task(index, url=None):
    return {"task_id": IDS[index], "status": "completed", "request": {"source_url": URLS[index] if url is None else url,
            "headers": [{"name": "Authorization", "value": "SECRET"}], "cookie": "SECRET", "referrer": "SECRET"},
            "resolution": {"effective_request": {"source_url": "https://mirror.example.org/not-original"}}}


def launch(frames=None, raw=None):
    data = raw if raw is not None else b"".join((json.dumps(f, ensure_ascii=False) + "\n").encode() for f in frames)
    p = subprocess.run([sys.executable, "-B", str(ROOT / "main.py")], input=data,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5)
    assert p.stderr == b"", p.stderr
    output = [json.loads(line) for line in p.stdout.splitlines()]
    assert all(f["run_id"] == RUN for f in output)
    return p, output


class DownloadTests(unittest.TestCase):
    def test_lines_read_sequential_then_copy_once(self):
        _, frames = launch([invoke(), reply("get-1", task(0)), reply("get-2", task(1)), reply("copy", {})])
        self.assertEqual([f.get("method") for f in frames], ["downloads.get", "downloads.get", "common.copy", None])
        self.assertEqual([f["params"] for f in frames[:2]], [{"task_id": i} for i in IDS])
        self.assertEqual(frames[2]["params"], {"text": "\n".join(URLS)})
        self.assertEqual(frames[-1]["status"], "success")
        self.assertNotIn("SECRET", json.dumps(frames))
        self.assertNotIn(URLS[0], json.dumps(frames[-1], ensure_ascii=False))

    def test_markdown_destination_escaped_fixed_label(self):
        _, frames = launch([invoke("copy-markdown", [IDS[1]]), reply("get-1", task(1)), reply("copy", {})])
        self.assertEqual(frames[1]["params"], {"text": "- [下载 1](<https://example.org/a%28b%29%5Bx%5D%3Cz%3E>)"})
        self.assertEqual(frames[-1]["status"], "success")

    def test_bad_selection_no_calls(self):
        for ids in ([], ["bad"], [IDS[0]] * 2, [IDS[0]] * 33, [True], ["{" + IDS[0] + "}"], None):
            frame = invoke(ids=[] if ids is None else ids)
            _, frames = launch([frame])
            self.assertEqual([f["op"] for f in frames], ["complete"])
            self.assertEqual(frames[-1]["status"], "error")

    def test_host_error_cancel_stale_id_no_copy(self):
        for bad in (reply("get-1", task(0), success=False), reply("wrong", task(0)),
                    reply("get-1", task(0), run_id=IDS[0]), reply("get-1", {**task(0), "task_id": IDS[1]}),
                    {"protocol_version": 1, "op": "cancel", "run_id": RUN}):
            _, frames = launch([invoke(), bad])
            self.assertEqual([f.get("method") for f in frames], ["downloads.get", None])
            self.assertEqual(frames[-1]["status"], "error")
            self.assertNotIn("SECRET", json.dumps(frames))

    def test_later_failure_never_partial_copy(self):
        _, frames = launch([invoke(), reply("get-1", task(0)), reply("get-2", {}, success=False)])
        self.assertEqual([f.get("method") for f in frames], ["downloads.get", "downloads.get", None])
        self.assertEqual(frames[-1]["status"], "error")

    def test_unsafe_url_and_size_no_copy(self):
        for url in ("https://user:SECRET@example.org/a", "https://example.org/a\nb", "file:///a", "https://example.org/" + "a" * 65536):
            _, frames = launch([invoke(ids=[IDS[0]]), reply("get-1", task(0, url))])
            self.assertEqual([f.get("method") for f in frames], ["downloads.get", None])
            self.assertEqual(frames[-1]["status"], "error")

    def test_exact_limit_and_output_expansion_bound(self):
        url = "https://example.org/" + "a" * (65536 - len("https://example.org/"))
        _, frames = launch([invoke(ids=[IDS[0]]), reply("get-1", task(0, url)), reply("copy", {})])
        self.assertEqual(len(frames[1]["params"]["text"].encode()), 65536)
        _, frames = launch([invoke("copy-markdown", [IDS[0]]), reply("get-1", task(0, url))])
        self.assertEqual([f.get("method") for f in frames], ["downloads.get", None])

    def test_copy_failure_cancel_and_stale_no_retry(self):
        for last in (reply("copy", {}, success=False), reply("wrong", {}), {"protocol_version": 1, "op": "cancel", "run_id": RUN}):
            _, frames = launch([invoke(ids=[IDS[0]]), reply("get-1", task(0)), last])
            self.assertEqual(sum(f.get("method") == "common.copy" for f in frames), 1)
            self.assertEqual(frames[-1]["status"], "error")

    def test_duplicate_reply_key_rejected(self):
        raw = (json.dumps(invoke()) + "\n").encode() + b'{"protocol_version":1,"op":"host_result","run_id":"' + RUN.encode() + b'","call_id":"get-1","success":true,"success":false}\n'
        _, frames = launch(raw=raw)
        self.assertEqual(frames[-1]["status"], "error")
        self.assertEqual(len(frames), 2)


if __name__ == "__main__":
    unittest.main()
