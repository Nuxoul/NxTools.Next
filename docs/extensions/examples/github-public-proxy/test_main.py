"""Owned subprocess fixtures only. No network, Desktop or user data."""
import json
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parent
RUN = "11111111-1111-4111-8111-111111111111"
OTHER = "22222222-2222-4222-8222-222222222222"
URL = "https://github.com/owner/repo/releases/download/v1/file.zip"


def invoke(url=URL, settings=None, **request):
    return {"protocol_version": 1, "op": "invoke", "run_id": RUN,
            "package_id": "example.github-public-proxy", "handler_id": "resolve",
            "feature": "downloads", "settings": settings if settings is not None else {"third_party_opt_in": True},
            "context": {"original_url": url, "request": {"source_url": url, "headers": [], "cookie": None, "referrer": None, **request}, "stage": 0}}


def launch(frame=None, raw=None):
    data = raw if raw is not None else (json.dumps(frame, ensure_ascii=False) + "\n").encode()
    p = subprocess.run([sys.executable, "-B", str(ROOT / "main.py")], input=data,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5)
    assert p.stderr == b"", p.stderr
    frames = [json.loads(line) for line in p.stdout.splitlines()]
    assert all(f["run_id"] == RUN for f in frames)
    return p, frames


class ProxyTests(unittest.TestCase):
    def result(self, frame, status):
        p, frames = launch(frame)
        self.assertEqual(p.returncode, 0)
        self.assertEqual(len(frames), 1)
        self.assertEqual(frames[0]["op"], "complete")
        self.assertEqual(frames[0]["status"], status)
        return frames[0]["value"]

    def test_release_rewrite_clears_all_credentials(self):
        value = self.result(invoke(), "handled")
        self.assertEqual(value["url"], "https://ghproxy.net/" + URL)
        self.assertEqual(value["headers"], [])
        self.assertIsNone(value["cookie"])
        self.assertIsNone(value["referrer"])
        self.assertTrue(0 < value["valid_for_ms"] <= 60000)

    def test_archive_and_raw(self):
        for url in ("https://github.com/o/r/archive/refs/heads/main.zip", "https://raw.githubusercontent.com/o/r/main/a.txt"):
            with self.subTest(url=url):
                self.assertEqual(self.result(invoke(url), "handled")["url"], "https://ghproxy.net/" + url)

    def test_opt_in_is_explicit_bool(self):
        value = self.result(invoke(settings={}), "not_handled")
        self.assertIn("第三方", value["text"])
        for opt in (False, "true", 1, None):
            self.result(invoke(settings={"third_party_opt_in": opt}), "not_handled" if opt is False else "error")

    def test_public_only_rejections(self):
        for url in ("http://github.com/o/r/archive/a.zip", "https://github.com.evil.org/o/r/archive/a.zip",
                    "https://github.com/o/r/blob/main/a", "https://github.com/o/r", "https://github.com/o/r/releases/latest",
                    "https://user:pass@github.com/o/r/archive/a.zip", URL + "?token=SECRET", URL + "#x", URL + "?",
                    URL.replace("github.com", "github.com:444"), URL.replace("file.zip", "../file.zip"),
                    URL.replace("file.zip", "%2ffile.zip"), URL.replace("file.zip", "%252ffile.zip"),
                    URL.replace("/v1/", "//v1/"), URL + "\\bad", URL + "\n", "https://raw.githubusercontent.com/o/r/main",
                    "https://ghproxy.net/" + URL, "https://github.com./o/r/archive/a.zip"):
            with self.subTest(url=url):
                self.result(invoke(url), "not_handled")

    def test_credentials_are_rejected_not_echoed(self):
        for request in ({"headers": [{"name": "Authorization", "value": "SECRET"}]},
                        {"cookie": "SECRET"}, {"referrer": "https://secret.org/SECRET"},
                        {"headers": None}, {"cookie": ""}, {"referrer": ""}):
            value = self.result(invoke(**request), "not_handled")
            self.assertNotIn("SECRET", json.dumps(value))

    def test_custom_prefix(self):
        value = self.result(invoke(settings={"third_party_opt_in": True, "proxy_prefix": "https://mirror.example.org/public/"}), "handled")
        self.assertEqual(value["url"], "https://mirror.example.org/public/" + URL)

    def test_bad_prefix(self):
        for prefix in ("http://mirror.example.org/", "https://localhost/", "https://127.0.0.1/", "https://10.0.0.1/",
                       "https://[::1]/", "https://2130706433/", "https://x.localhost/", "https://a:b@mirror.example.org/",
                       "https://mirror.example.org/?x", "https://mirror.example.org/#x", "https://mirror.example.org:444/",
                       "https://mirror.example.org/a/../", "https://mirror.example.org/%2f/", "https://github.com/",
                       "https://raw.githubusercontent.com/", "https://mirror.example.org/https://github.com/", "https://mirror.example.org\\evil/"):
            with self.subTest(prefix=prefix):
                self.result(invoke(settings={"third_party_opt_in": True, "proxy_prefix": prefix}), "error")

    def test_matching_cancel_and_identity(self):
        self.result({"protocol_version": 1, "op": "cancel", "run_id": RUN}, "error")
        frame = invoke(); frame["handler_id"] = "wrong"
        self.result(frame, "error")

    def test_malformed_duplicate_oversize_no_payload(self):
        for raw in (b"broken\n", b"\xff\n", b"[]\n", b'{"x":1,"x":2}\n',
                    b'{"x":NaN}\n', b"x" * (256 * 1024 + 1) + b"\n"):
            p, frames = launch(raw=raw)
            self.assertNotEqual(p.returncode, 0)
            self.assertEqual(frames, [])

    def test_request_not_original_is_effective_input(self):
        frame = invoke(); frame["context"]["request"]["source_url"] = "https://ghproxy.net/" + URL
        self.result(frame, "not_handled")


if __name__ == "__main__":
    unittest.main()
