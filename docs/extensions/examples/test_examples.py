"""Aggregate runner and deterministic archive tests. No plugin imports in this process."""
import asyncio
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
import zipfile

ROOT = Path(__file__).resolve().parent
NAMES = ("github-public-proxy", "download-link-list", "clipboard-text-tools")


class PackageTests(unittest.TestCase):
    def test_subprocess_suites(self):
        for name in NAMES:
            with self.subTest(package=name):
                completed = subprocess.run([sys.executable, "-B", str(ROOT / name / "test_main.py")],
                                           stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=45)
                self.assertEqual(completed.returncode, 0, completed.stderr.decode("utf-8", "replace"))

    def test_manifest_files_complete_and_runtime(self):
        for name in NAMES:
            root = ROOT / name
            manifest = json.loads((root / "extension.json").read_text(encoding="utf-8"))
            self.assertEqual(manifest["id"], "example." + name)
            self.assertEqual(manifest["runtime"]["program"], "python")
            self.assertEqual(set(manifest["files"]), {"main.py", "protocol.py", "test_main.py", "README.md"})
            self.assertEqual({p.name for p in root.iterdir()}, {"extension.json", *manifest["files"]})
            self.assertTrue(all(h["action"] == {"type": "script", "entry_file": "main.py"} for h in manifest["handlers"]))
            self.assertTrue(all(h["entry"] in ("action", "resolve") for h in manifest["handlers"]))

    def test_zip_deterministic_exact_root_members(self):
        spec = importlib.util.spec_from_file_location("example_builder", ROOT / "build-examples.py")
        builder = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(builder)
        for name in NAMES:
            first, second = builder.archive_bytes(ROOT / name), builder.archive_bytes(ROOT / name)
            self.assertEqual(first, second)
            manifest = json.loads((ROOT / name / "extension.json").read_text(encoding="utf-8"))
            with zipfile.ZipFile(io.BytesIO(first)) as archive:
                self.assertEqual(archive.namelist(), ["extension.json", *sorted(manifest["files"])])
                self.assertIsNone(archive.testzip())
                for entry in archive.infolist():
                    self.assertEqual(entry.date_time, (1980, 1, 1, 0, 0, 0))
                    self.assertEqual(archive.read(entry), (ROOT / name / entry.filename).read_bytes())

    def test_interactive_fake_host_flush_and_sequential_calls(self):
        async def exchange(name, handler, context, expected):
            run = "11111111-1111-4111-8111-111111111111"
            process = await asyncio.create_subprocess_exec(
                sys.executable, "-B", str(ROOT / name / "main.py"),
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            )
            async def send(frame):
                process.stdin.write((json.dumps({"protocol_version": 1, "run_id": run, **frame}) + "\n").encode())
                await process.stdin.drain()
            try:
                await send({"op": "invoke", "package_id": "example." + name, "handler_id": handler,
                            "feature": "downloads" if name == "download-link-list" else "clipboard",
                            "settings": {}, "context": context})
                for method, params, value in expected:
                    raw = await asyncio.wait_for(process.stdout.readline(), 5)
                    frame = json.loads(raw)
                    self.assertEqual(frame, {"protocol_version": 1, "run_id": run, "op": "host_call",
                                             "call_id": frame["call_id"], "method": method, "params": params})
                    # Reply only after observing each real flushed call. A second
                    # read cannot be admitted until the first matching reply.
                    await send({"op": "host_result", "call_id": frame["call_id"], "success": True, "value": value})
                terminal = json.loads(await asyncio.wait_for(process.stdout.readline(), 5))
                self.assertEqual(terminal["op"], "complete")
                self.assertEqual(terminal["status"], "success")
                self.assertEqual(terminal["run_id"], run)
                # A one-shot process must exit even while host stdin is open.
                await asyncio.wait_for(process.wait(), 5)
                self.assertEqual(process.returncode, 0)
                self.assertEqual(await process.stdout.read(), b"")
                self.assertEqual(await process.stderr.read(), b"")
            finally:
                if process.returncode is None:
                    process.kill()
                    await process.wait()
                process.stdin.close()
                await process.stdin.wait_closed()
        ids = ["22222222-2222-4222-8222-222222222222", "33333333-3333-4333-8333-333333333333"]
        urls = ["https://example.org/first", "https://example.org/second"]
        expected = [("downloads.get", {"task_id": i}, {"task_id": i, "request": {"source_url": u}}) for i, u in zip(ids, urls)]
        expected.append(("common.copy", {"text": "\n".join(urls)}, {}))
        asyncio.run(exchange("download-link-list", "copy-lines", {"task_ids": ids}, expected))
        asyncio.run(exchange("clipboard-text-tools", "unique-lines", {"selection": {"item_id": 123, "kind": "text", "text": "乙\n甲\n乙"}},
                             [("clipboard.create_derived", {"source_item_id": 123, "text": "乙\n甲"}, {"item_id": 456})]))

    def test_shared_transport_fail_closed_for_each_standalone_package(self):
        run = "11111111-1111-4111-8111-111111111111"
        for name in NAMES:
            for raw in (b"bad\n", b'{"x":1,"x":2}\n', b'{"x":NaN}\n', b"x" * (256 * 1024 + 1) + b"\n"):
                result = subprocess.run([sys.executable, "-B", str(ROOT / name / "main.py")], input=raw,
                                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, b"")
                self.assertEqual(result.stderr, b"")
            invoke = {"protocol_version": 1, "op": "invoke", "run_id": run, "package_id": "example." + name,
                      "feature": "downloads" if name != "clipboard-text-tools" else "clipboard",
                      "handler_id": "resolve" if name == "github-public-proxy" else "copy-lines" if name == "download-link-list" else "unique-lines",
                      "context": {}, "settings": {}, "unexpected": "SECRET"}
            result = subprocess.run([sys.executable, "-B", str(ROOT / name / "main.py")],
                                    input=(json.dumps(invoke) + "\n").encode(), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=5)
            frame = json.loads(result.stdout)
            self.assertEqual(frame["status"], "error")
            self.assertNotIn("SECRET", result.stdout.decode())
            self.assertEqual(result.stderr, b"")


if __name__ == "__main__":
    unittest.main()
