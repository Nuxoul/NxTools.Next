"""Isolated one-shot subprocess and fake derived-item host."""
import json
from pathlib import Path
import subprocess
import sys
import unittest

ROOT = Path(__file__).resolve().parent
RUN = "11111111-1111-4111-8111-111111111111"
OTHER = "22222222-2222-4222-8222-222222222222"


def invoke(text="乙\n甲\n乙", handler="unique-lines"):
    return {"protocol_version": 1, "op": "invoke", "run_id": RUN,
            "package_id": "example.clipboard-text-tools", "handler_id": handler,
            "feature": "clipboard", "settings": {},
            "context": {"selection": {"item_id": 123, "kind": "text", "text": text}}}


def reply(**changes):
    return {"protocol_version": 1, "op": "host_result", "run_id": RUN,
            "call_id": "derive", "success": True, "value": {"item_id": 456}, **changes}


def launch(frames=None, raw=None):
    data = raw if raw is not None else b"".join((json.dumps(f, ensure_ascii=False) + "\n").encode() for f in frames)
    p = subprocess.run([sys.executable, "-B", str(ROOT / "main.py")], input=data,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5)
    assert p.stderr == b"", p.stderr
    output = [json.loads(line) for line in p.stdout.splitlines()]
    assert all(f["run_id"] == RUN for f in output)
    return p, output


class ClipboardTests(unittest.TestCase):
    def changed(self, text, handler, expected):
        _, frames = launch([invoke(text, handler), reply()])
        self.assertEqual([f.get("method") for f in frames], ["clipboard.create_derived", None])
        self.assertEqual(frames[0]["params"], {"source_item_id": 123, "text": expected})
        self.assertEqual(frames[-1]["status"], "success")
        self.assertNotIn(expected, json.dumps(frames[-1], ensure_ascii=False))

    def no_write(self, frame, status):
        _, frames = launch([frame])
        self.assertEqual([f["op"] for f in frames], ["complete"])
        self.assertEqual(frames[0]["status"], status)

    def test_unique_preserves_first_order_and_unicode(self):
        self.changed("乙😀\r\n甲\r\n乙😀\r\n甲", "unique-lines", "乙😀\n甲")

    def test_pretty_json_unicode(self):
        self.changed('{"字":"😀","a":[1,true]}', "pretty-json", '{\n  "字": "😀",\n  "a": [\n    1,\n    true\n  ]\n}')

    def test_uri_decode_not_form_encoding(self):
        self.changed("a+b%20%E4%B8%AD%E6%96%87", "uri-decode", "a+b 中文")

    def test_nochange_empty_nontext(self):
        for text, handler in (("甲\n乙", "unique-lines"), ("", "unique-lines"), ("a+b", "uri-decode"),
                              ('{\n  "a": 1\n}', "pretty-json")):
            self.no_write(invoke(text, handler), "not_handled")
        frame = invoke(); frame["context"]["selection"] = {"item_id": 123, "kind": "image"}
        self.no_write(frame, "not_handled")

    def test_strict_json_duplicate_nonfinite_invalid_deep(self):
        for text in ('{"x":1,"x":2}', '{"x":NaN}', '{"x":Infinity}', '{"x":1e999}',
                     '{"x":', "[" * 40 + "0" + "]" * 40, '{"x":"\\ud800"}'):
            with self.subTest(text=text):
                self.no_write(invoke(text, "pretty-json"), "error")

    def test_uri_decode_conservative(self):
        for text in ("%", "%GG", "%FF", "%00", "%0A", "%0D", "%7F"):
            self.no_write(invoke(text, "uri-decode"), "error")

    def test_bounds_input_and_expanded_output(self):
        self.no_write(invoke("界" * 22000), "error")
        self.no_write(invoke("[" + ",".join("0" for _ in range(20000)) + "]", "pretty-json"), "error")
        self.changed("a" * 65532 + "\na\na", "unique-lines", "a" * 65532 + "\na")

    def test_invalid_selection_and_identity(self):
        for item in (True, 0, -1, 1 << 63, "123"):
            frame = invoke(); frame["context"]["selection"]["item_id"] = item
            self.no_write(frame, "error")
        frame = invoke(); frame["handler_id"] = "committed"
        self.no_write(frame, "error")

    def test_host_fail_stale_cancel_no_retry(self):
        for last in (reply(success=False, value={"error": "SECRET"}), reply(call_id="old"), reply(run_id=OTHER),
                     reply(value={"item_id": True}), reply(value={"item_id": 123}),
                     {"protocol_version": 1, "op": "cancel", "run_id": RUN}):
            _, frames = launch([invoke(), last, reply()])
            self.assertEqual(len(frames), 2)
            self.assertEqual(frames[-1]["status"], "error")
            self.assertNotIn("SECRET", json.dumps(frames))

    def test_malformed_frames_and_cancel_before_invoke(self):
        self.no_write({"protocol_version": 1, "op": "cancel", "run_id": RUN}, "error")
        for raw in (b"", b"bad\n", b'{"x":1,"x":2}\n', b'{"x":NaN}\n', b"\xff\n",
                    b"x" * (256 * 1024 + 1) + b"\n"):
            p, frames = launch(raw=raw)
            self.assertNotEqual(p.returncode, 0)
            self.assertEqual(frames, [])


if __name__ == "__main__":
    unittest.main()
