"""Synthetic inputs and fake host only; never uses real clipboard or services."""

import asyncio
import copy
import itertools
import json
from pathlib import Path
import subprocess
import sys
import unittest

sys.dont_write_bytecode = True
import main

ROOT = Path(__file__).resolve().parent
RUN_ID = "11111111-1111-4111-8111-111111111111"
OTHER_ID = "22222222-2222-4222-8222-222222222222"


def wire(frame):
    return (json.dumps(frame, ensure_ascii=False) + "\n").encode("utf-8")


def invocation(text="第一行  \r\n\r\n\r\n第二行\t"):
    return {
        "protocol_version": 1, "op": "invoke", "run_id": RUN_ID,
        "correlation_id": OTHER_ID, "generation": 1,
        "package_id": "example.clipboard-cleaner", "handler_id": "clean",
        "content_hash": "a" * 64, "feature": "clipboard",
        "context": {"selection": {"item_id": 123, "kind": "text", "text": text}},
        "settings": {},
        "causality": {"root_id": OTHER_ID, "origin_plugin": "example.clipboard-cleaner",
                      "handlers": ["example.clipboard-cleaner/clean"]},
    }


def result(**changes):
    frame = {
        "protocol_version": 1, "op": "host_result", "run_id": RUN_ID,
        "call_id": "create-derived", "success": True, "value": {"item_id": 456},
    }
    frame.update(changes)
    return frame


class CleanTextTests(unittest.TestCase):
    def test_unicode_preserved(self):
        text = "中文😀 café e\u0301\u200d\u2028末尾"
        self.assertEqual(main.clean_text(text + "  ", {}), text)

    def test_newlines_always_normalized(self):
        disabled = dict.fromkeys(main.DEFAULT_OPTIONS, False)
        self.assertEqual(main.clean_text("甲\r\n乙\r丙\n", disabled), "甲\n乙\n丙\n")

    def test_all_eight_setting_combinations(self):
        for trim, collapse, outer in itertools.product((False, True), repeat=3):
            with self.subTest(trim=trim, collapse=collapse, outer=outer):
                options = dict(zip(main.DEFAULT_OPTIONS, (trim, collapse, outer)))
                before = options.copy()
                body = "甲\n" if trim else "甲 \t\n"
                body += "\n" if collapse else "\n\n"
                body += "乙" if trim else "乙 \t"
                expected = "\n" + body + "\n"
                if outer:
                    expected = expected.strip()
                self.assertEqual(main.clean_text("\r\n甲 \t\r\n\r\n\r\n乙 \t\r\n", options), expected)
                self.assertEqual(options, before)

    def test_blank_runs_and_whitespace(self):
        self.assertEqual(main.clean_text("甲\n \t\n\u3000\n\n乙", {}), "甲\n\n乙")
        self.assertEqual(main.clean_text(" \t\r\n\u3000\r\n", {}), "")
        self.assertEqual(main.clean_text("", {}), "")
        self.assertEqual(main.clean_text("甲\n \t\n\t\n乙", {
            "trim_line_ends": False, "trim_outer": False,
        }), "甲\n \t\n乙")

    def test_invalid_setting_types(self):
        for invalid in ("false", 0, 1, None, [], {}):
            with self.subTest(invalid=invalid), self.assertRaises(main.ActionError):
                main.clean_text("文本", {"trim_outer": invalid})

    def test_manifest_matches_package(self):
        manifest = json.loads((ROOT / "extension.json").read_text(encoding="utf-8"))
        self.assertEqual(set(manifest), {
            "schema_version", "id", "name", "version", "description", "permissions",
            "runtime", "files", "settings", "handlers",
        })
        self.assertEqual(manifest["id"], main.PACKAGE_ID)
        self.assertEqual(manifest["version"], "1.0.0")
        self.assertEqual(manifest["files"], ["main.py", "test_main.py", "README.md"])
        self.assertTrue(all((ROOT / name).is_file() for name in manifest["files"]))
        self.assertEqual({s["key"]: s["default"] for s in manifest["settings"]}, main.DEFAULT_OPTIONS)
        self.assertEqual(manifest["handlers"], [{
            "id": "clean", "title": "整理文本并保留原文", "feature": "clipboard",
            "entry": "action", "action": {"type": "script", "entry_file": "main.py"},
        }])


class ProtocolTests(unittest.TestCase):
    def decode(self, raw, run_id=RUN_ID):
        if raw:
            self.assertTrue(raw.endswith(b"\n"))
        frames = []
        for line in raw.splitlines():
            frame = json.loads(line.decode("utf-8"), object_pairs_hook=main.unique_object)
            self.assertIsInstance(frame, dict)
            self.assertEqual(frame["protocol_version"], 1)
            self.assertEqual(frame["run_id"], run_id)
            self.assertIn(frame["op"], ("host_call", "complete"))
            expected = {"protocol_version", "op", "run_id"}
            expected |= {"call_id", "method", "params"} if frame["op"] == "host_call" else {"status", "value"}
            self.assertEqual(set(frame), expected)
            frames.append(frame)
        self.assertLessEqual(sum(f["op"] == "host_call" for f in frames), 1)
        self.assertLessEqual(sum(f["op"] == "complete" for f in frames), 1)
        return frames

    def launch(self, frames=None, raw=None, run_id=RUN_ID):
        process = subprocess.run(
            [sys.executable, "-B", str(ROOT / "main.py")],
            input=raw if raw is not None else b"".join(wire(f) for f in frames),
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5,
        )
        output = self.decode(process.stdout, run_id)
        if output:
            self.assertEqual(process.returncode, 0)
            self.assertEqual(process.stderr, b"")
            self.assertEqual(output[-1]["op"], "complete")
            self.assertEqual(sum(f["op"] == "complete" for f in output), 1)
        return process, output

    def assert_no_write(self, invoke, status):
        _, output = self.launch([invoke])
        self.assertEqual([f["op"] for f in output], ["complete"])
        self.assertEqual(output[0]["status"], status)
        return output[0]

    def test_unchanged_has_no_host_call(self):
        self.assert_no_write(invocation("中文😀\n第二行"), "not_handled")

    def test_nontext_has_no_host_call(self):
        for kind in ("files", "image"):
            invoke = invocation()
            invoke["context"]["selection"] = {"item_id": 123, "kind": kind}
            self.assert_no_write(invoke, "not_handled")

    def test_empty_has_no_host_call_even_with_outer_disabled(self):
        for text in ("", " \t\r\n\u3000\n"):
            for enabled in (False, True):
                invoke = invocation(text)
                invoke["settings"] = dict.fromkeys(main.DEFAULT_OPTIONS, enabled)
                frame = self.assert_no_write(invoke, "not_handled")
                self.assertIn("为空", frame["value"]["text"])

    def test_limit_is_utf8_bytes_and_no_payload_leak(self):
        marker = "SYNTHETIC-ONLY-MARKER"
        invoke = invocation(marker + "界" * (main.MAX_TEXT_BYTES // 3 + 1))
        frame = self.assert_no_write(invoke, "error")
        self.assertIn("64 KiB", frame["value"]["text"])
        self.assertNotIn(marker, json.dumps(frame, ensure_ascii=False))
        self.assertNotIn("界", json.dumps(frame, ensure_ascii=False))

    def test_exact_limit_accepted(self):
        text = "a" * (main.MAX_TEXT_BYTES - 1) + " "
        _, frames = self.launch([invocation(text), result()])
        self.assertEqual(frames[0]["params"]["text"], text[:-1])
        self.assertEqual(frames[-1]["status"], "success")

    def test_settings_missing_use_defaults_and_types_are_strict(self):
        invoke = invocation()
        del invoke["settings"]
        _, frames = self.launch([invoke, result()])
        self.assertEqual(frames[0]["params"]["text"], "第一行\n\n第二行")
        for invalid in ("false", 0, None, [], {"unknown": True}):
            invoke = invocation()
            invoke["settings"] = {"trim_outer": invalid} if invalid != {"unknown": True} else invalid
            self.assert_no_write(invoke, "error")

    def test_invalid_invoke_identity_and_context(self):
        for key, value in (("protocol_version", 2), ("protocol_version", True),
                           ("op", "other"), ("package_id", "other"),
                           ("handler_id", "other"), ("feature", "other"),
                           ("context", {}), ("context", {"text": "wrong place"})):
            with self.subTest(key=key, value=value):
                invoke = invocation()
                invoke[key] = value
                self.assert_no_write(invoke, "error")
        for selection in ({"kind": "text", "text": "x"},
                          {"item_id": True, "kind": "text", "text": "x"},
                          {"item_id": 123, "kind": "text", "text": None}):
            invoke = invocation()
            invoke["context"]["selection"] = selection
            self.assert_no_write(invoke, "error")

    def test_host_failure_does_not_expose_payload_or_retry(self):
        _, frames = self.launch([invocation(), result(success=False, value={"error": "SYNTHETIC-PRIVATE"})])
        self.assertEqual([f["op"] for f in frames], ["host_call", "complete"])
        self.assertEqual(frames[-1]["status"], "error")
        self.assertNotIn("SYNTHETIC-PRIVATE", json.dumps(frames))

    def test_invalid_host_results_fail_once(self):
        for changes in ({"run_id": OTHER_ID}, {"call_id": "wrong"},
                        {"protocol_version": 2}, {"protocol_version": True},
                        {"op": "invoke"}, {"success": "true"},
                        {"success": 1}, {"value": {}}, {"value": {"item_id": True}}):
            with self.subTest(changes=changes):
                _, frames = self.launch([invocation(), result(**changes), result()])
                self.assertEqual(len(frames), 2)
                self.assertEqual(frames[-1]["status"], "error")

    def test_eof_and_cancel_stop_without_retry(self):
        _, frames = self.launch([invocation()])
        self.assertEqual(frames[-1]["status"], "error")
        cancel = {"protocol_version": 1, "op": "cancel", "run_id": RUN_ID}
        self.assert_no_write(cancel, "error")
        _, frames = self.launch([invocation(), cancel, result()])
        self.assertEqual(len(frames), 2)
        self.assertEqual(frames[-1]["status"], "error")
        self.assertIn("取消", frames[-1]["value"]["text"])

    def test_malformed_input_without_run_id_has_fixed_stderr(self):
        for raw in (b"", b"not-json\n", b"[]\n", b"\xff\n", b"{\"run_id\":\"bad\"}\n",
                    b"{\"run_id\":\"bad\",\"run_id\":\"bad\"}\n",
                    b"{\"x\":NaN}\n", wire(invocation()).rstrip(b"\n"),
                    b"x" * (main.MAX_FRAME_BYTES + 1) + b"\n"):
            with self.subTest(length=len(raw)):
                process, frames = self.launch(raw=raw)
                self.assertEqual(frames, [])
                self.assertNotEqual(process.returncode, 0)
                self.assertIn(process.stderr, tuple(
                    category + ending
                    for category in (b"eof", b"protocol_error", b"frame_too_large")
                    for ending in (b"\n", b"\r\n")
                ))

    def test_bad_reply_json_or_eof_has_one_error_completion(self):
        for reply in (b"broken\n", b"\xff\n", b"{}", b"[]\n"):
            _, frames = self.launch(raw=wire(invocation()) + reply)
            self.assertEqual(len(frames), 2)
            self.assertEqual(frames[-1]["status"], "error")

    def test_run_id_preserved_exactly_and_only_one_invocation(self):
        invoke = invocation()
        invoke["run_id"] = "ABCDEFAB-1111-4111-8111-111111111111"
        _, frames = self.launch([invoke, result(run_id=invoke["run_id"]), invoke], run_id=invoke["run_id"])
        self.assertEqual(len(frames), 2)
        self.assertEqual(frames[-1]["status"], "success")

    def test_actual_interactive_jsonl_roundtrip_and_flush(self):
        async def exchange():
            process = await asyncio.create_subprocess_exec(
                sys.executable, "-B", str(ROOT / "main.py"),
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            )
            try:
                invoke = invocation()
                before = copy.deepcopy(invoke)
                process.stdin.write(wire(invoke))
                await process.stdin.drain()
                first_raw = await asyncio.wait_for(process.stdout.readline(), timeout=5)
                first = self.decode(first_raw)[0]
                self.assertEqual(first, {
                    "protocol_version": 1, "op": "host_call", "run_id": RUN_ID,
                    "call_id": "create-derived", "method": "clipboard.create_derived",
                    "params": {"source_item_id": 123, "text": "第一行\n\n第二行"},
                })
                self.assertIn("第一行".encode("utf-8"), first_raw)
                self.assertEqual(invoke, before)
                # Reply only after observing the real request (and its flush).
                process.stdin.write(wire(result()))
                await process.stdin.drain()
                last_raw = await asyncio.wait_for(process.stdout.readline(), timeout=5)
                last = self.decode(last_raw)[0]
                self.assertEqual(last["status"], "success")
                self.assertEqual(last["value"]["text"], "已生成整理后的文本，原内容保留。")
                # No stdin EOF is needed to finish a normal one-shot invocation.
                await asyncio.wait_for(process.wait(), timeout=5)
                self.assertEqual(await process.stdout.read(), b"")
                self.assertEqual(await process.stderr.read(), b"")
                self.assertEqual(process.returncode, 0)
                self.assertEqual(len(self.decode(first_raw + last_raw)), 2)
            finally:
                if process.returncode is None:
                    process.kill()
                    await process.wait()
                process.stdin.close()
                await process.stdin.wait_closed()
        asyncio.run(exchange())


if __name__ == "__main__":
    unittest.main()
