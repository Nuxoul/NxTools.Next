"""Synthetic contracts for material CPU, pairing and frame analysis."""
import csv
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("analysis", Path(__file__).with_name("analyze-native-material.py"))
analysis = importlib.util.module_from_spec(spec)
spec.loader.exec_module(analysis)


def sample(qpc, cpu, status="available"):
    return {"Qpc": qpc, "Samples": [{"ProcessId": 42, "ProcessStatus": status,
        "CpuTotalMilliseconds": cpu, "PrivateBytes": 2 * 1024**2,
        "WorkingSetBytes": 3 * 1024**2, "HandleCount": 4,
        "GpuEngineStatus": "no_instance", "GpuMaxEnginePercent": None,
        "GpuDedicatedMemoryStatus": "available", "GpuDedicatedBytes": 0,
        "GpuSharedMemoryStatus": "access_denied", "GpuSharedBytes": None}]}


class AnalysisTests(unittest.TestCase):
    def test_cpu_normalization_and_unavailable_are_distinct(self):
        result = analysis.role_summary([sample(100, 0), sample(1100, 100)], 42, 1000, 12)
        self.assertEqual(result["cpu_one_core_percent"], 10)
        self.assertAlmostEqual(result["cpu_machine_percent"], 10 / 12)
        self.assertEqual(result["private_mib"]["median"], 2)
        self.assertEqual(result["working_set_mib"]["median"], 3)
        self.assertIsNone(result["gpu_max_engine_percent"]["median"])
        self.assertEqual(result["gpu_dedicated_mib"]["median"], 0)
        missing = analysis.role_summary([sample(100, 0), sample(1100, None, "access_denied")], 42, 1000, 12)
        self.assertIsNone(missing["cpu_one_core_percent"])

    def test_pairing_uses_each_pair_not_difference_of_medians(self):
        runs = [{"scenario": "idle", "repetition": repetition, "material": material,
                 "roles": {"gpui": {metric: value for metric in (
                     "cpu_one_core_percent", "cpu_machine_percent", "private_mib", "working_set_mib",
                     "handles", "gpu_max_engine_percent", "gpu_dedicated_mib", "gpu_shared_mib")}}}
                for repetition, solid, acrylic in [(1, 0, 10), (2, 100, 101), (3, 20, 25)]
                for material, value in [("solid", solid), ("acrylic", acrylic)]]
        result = analysis.paired_summaries(runs)[0]
        self.assertEqual(result["paired_delta"]["median"], 5)
        with self.assertRaises(ValueError):
            analysis.paired_summaries(runs[:-1])

    def test_qpc_frames_are_pid_filtered_and_keep_chains_separate(self):
        with tempfile.TemporaryDirectory(dir=os.environ["PI_SCRATCH_DIR"]) as directory:
            path = Path(directory) / "frames.csv"
            with path.open("w", newline="", encoding="utf-8") as stream:
                writer = csv.writer(stream)
                writer.writerow(["ProcessID", "CPUStartQPC", "SwapChainAddress", "FrameTime", "CPUBusy", "CPUWait"])
                writer.writerows([[42, 5, "a", 50, 1, 49], [42, 15, "a", 50, 2, 48], [42, 20, "b", 16, 3, 13]])
            result = analysis.frame_summary(path, 42, 10, 25)
            self.assertEqual(result["frames"], 2)
            self.assertEqual(len(result["swapchains"]), 2)
            self.assertEqual(result["swapchains"]["a"]["metrics_ms"]["CPUBusy"]["median"], 2)
            self.assertEqual(analysis.frame_summary(path, 42, 30, 40)["status"], "unavailable")
            with self.assertRaises(ValueError):
                analysis.frame_summary(path, 43, 10, 25)


    def test_gpu_mean_weights_observed_intervals(self):
        rows = [sample(0, 0), sample(100, 10), sample(400, 20)]
        for row, value in zip(rows, [0, 10, 20]):
            row["Samples"][0].update(GpuEngineStatus="available", GpuMaxEnginePercent=value)
        result = analysis.role_summary(rows, 42, 1000, 12)
        self.assertEqual(result["gpu_max_engine_percent"]["time_weighted_mean"], 17.5)
        rows[1]["Samples"][0].update(GpuEngineStatus="no_instance", GpuMaxEnginePercent=None)
        self.assertIsNone(analysis.role_summary(rows, 42, 1000, 12)["gpu_max_engine_percent"]["time_weighted_mean"])


if __name__ == "__main__":
    unittest.main()
