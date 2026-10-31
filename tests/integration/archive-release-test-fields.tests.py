"""Small tool-only tests; no product process, source lease or native acceptance."""

import copy
import importlib.util
import io
from pathlib import Path
import struct
import tempfile
import unittest
from unittest.mock import Mock, patch


SPEC = importlib.util.spec_from_file_location(
    "release_fields", Path(__file__).with_name("archive-release-test-fields.py"))
DRIVER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DRIVER)


class ReleaseFieldDriverTests(unittest.TestCase):
    def negative(self):
        return {"bounded_exit": True, "forced": False, "driver_error": None,
                "stderr_bytes": 0, "stdout_bytes": 0, "exit_code": 1,
                "frames": [], "frame_sequence": [], "native_actions": [],
                "operation_id": "synthetic-id"}

    def test_framing_partial_and_invalid_lengths(self):
        wire = DRIVER.pack({"op": "accepted"})
        self.assertEqual(DRIVER.decode_available(wire), ([{"op": "accepted"}], len(wire)))
        self.assertEqual(DRIVER.decode_available(wire[:-1]), ([], 0))
        for raw in (struct.pack("<I", 0), struct.pack("<I", DRIVER.MAX_FRAME + 1),
                    b"x" * (DRIVER.MAX_CAPTURE + 1), DRIVER.pack([])):
            with self.subTest(raw_size=len(raw)), self.assertRaises(AssertionError):
                DRIVER.decode_available(raw)

    def test_exact_six_requests_only_mutate_expected_fields(self):
        cases = DRIVER.requests(Path("synthetic.zip"))
        self.assertEqual([row[0] for row in cases], ["control_test", "test_fixture_object",
                         "test_fixture_null", "extract_fixture_object", "extract_fixture_null",
                         "unknown_field"])
        base = cases[0][1]
        self.assertEqual(base["operation"], "test")
        self.assertEqual(len(DRIVER.PAYLOAD), 30)
        self.assertEqual(sum(row[2] for row in cases), 1)
        for name, request, _ in cases[1:]:
            self.assertEqual(request["operation"], "extract" if name.startswith("extract_") else "test")
            reduced = copy.deepcopy(request)
            if name == "unknown_field":
                self.assertTrue(reduced.pop("unknown_release_probe"))
            else:
                self.assertIn("zip_risk_fixture", reduced)
                value = reduced.pop("zip_risk_fixture")
                self.assertEqual(value, None if name.endswith("null") else {"bytes": 1, "ratio": 1})
            reduced["operation"] = "test"
            self.assertEqual(reduced, base)

    def test_parser_nonzero_is_not_enough_for_rejection(self):
        DRIVER.verdict(self.negative(), False)
        changes = [{"exit_code": 0}, {"exit_code": 2}, {"exit_code": 0xC0000409},
                   {"forced": True}, {"bounded_exit": False}, {"driver_error": "timeout"},
                   {"stderr_bytes": 1}, {"stdout_bytes": 1},
                   {"frames": [{"schema": 1, "operation_id": "synthetic-id", "op": "accepted"}]},
                   {"native_actions": ["source_acquire"]}]
        for change in changes:
            result = self.negative()
            result.update(change)
            with self.subTest(change=change), self.assertRaises(AssertionError):
                DRIVER.verdict(result, False)

    def test_control_requires_complete_literal_counts_and_identity(self):
        result = self.negative()
        result.update({"exit_code": 0, "frame_sequence": ["accepted", "native_call", "complete"],
                       "native_actions": ["source_acquire"], "stdout_bytes": 200,
                       "frames": [{"schema": 1, "operation_id": "synthetic-id", "op": "complete",
                                   "files_verified": 2, "bytes_verified": 30}]})
        DRIVER.verdict(result, True)
        for change in ({"frame_sequence": ["accepted"]}, {"native_actions": ["write"]}):
            altered = copy.deepcopy(result)
            altered.update(change)
            with self.subTest(change=change), self.assertRaises(AssertionError):
                DRIVER.verdict(altered, True)
        for key, value in (("files_verified", 1), ("bytes_verified", 0),
                           ("schema", 2), ("operation_id", "foreign")):
            altered = copy.deepcopy(result)
            altered["frames"][-1][key] = value
            with self.subTest(key=key), self.assertRaises(AssertionError):
                DRIVER.verdict(altered, True)

    def test_build_identity_rejects_debug_profile_hash_and_feature_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            worker = (Path(directory) / "worker.exe").resolve()
            worker.write_bytes(b"tool-only-identity-not-executed")
            record = {"sha256": DRIVER.digest(worker), "artifact": {
                "reason": "compiler-artifact", "target": {"name": "nxtools-archive-worker", "kind": ["bin"]},
                "profile": {"debug_assertions": False, "opt_level": "3", "test": False},
                "features": ["isolated-codecs"], "executable": str(worker)}}
            self.assertEqual(DRIVER.release_identity(worker, record)["sha256"], record["sha256"])
            changed = copy.deepcopy(record)
            changed["artifact"]["profile"]["debug_assertions"] = True
            with self.assertRaises(AssertionError):
                DRIVER.release_identity(worker, changed)
            changed = copy.deepcopy(record)
            changed["sha256"] = "0" * 64
            with self.assertRaises(AssertionError):
                DRIVER.release_identity(worker, changed)
            changed = copy.deepcopy(record)
            changed["artifact"]["features"] = []
            with self.assertRaises(AssertionError):
                DRIVER.release_identity(worker, changed)
    def test_already_exited_but_late_child_is_not_rejection_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            child = Mock(pid=123, returncode=1, stdin=io.BytesIO())
            child.poll.return_value = 1
            child.wait.return_value = 1
            with patch.object(DRIVER.subprocess, "Popen", return_value=child), \
                    patch.object(DRIVER.time, "monotonic", side_effect=[0, 11, 11]):
                with self.assertRaisesRegex(AssertionError, "exited after deadline"):
                    DRIVER.run_case(Path("not-executed.exe"), Path(directory), "late",
                                    {"operation_id": "synthetic-id"}, False, 10)
            self.assertFalse(child.kill.called)


    def test_assertion_helper_cannot_be_optimized_away(self):
        with self.assertRaisesRegex(AssertionError, "sentinel"):
            DRIVER.check(False, "sentinel")


if __name__ == "__main__":
    unittest.main()
