import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import quick_launch_listary_report as report


def sample(product, phase, case_id, rows, repetition=1, first=12, error=None, focus=True, restored=True):
    return {
        "product": product,
        "phase": phase,
        "repetition": repetition,
        "case_id": case_id,
        "query": "查询",
        "error": error,
        "input_set_ms": 0,
        "first_result_ms": first,
        "observed_ms": 50,
        "polls": 2,
        "focus_ok": focus,
        "input_restored": restored,
        "observation_status": "quiet_snapshot",
        "rows": rows,
    }


def base_data():
    targets = [
        {"id": "plain", "name": "alpha.txt", "parent_suffix": ""},
        {"id": "team-a", "name": "same.txt", "parent_suffix": "team-a"},
        {"id": "team-b", "name": "same.txt", "parent_suffix": "team-b"},
    ]
    cases = [
        {"id": "hit", "category": "fixture", "query": "α", "expected": ["plain"], "forbidden": [], "expect_empty": False, "measure_quality": True},
        {"id": "duplicates", "category": "fixture", "query": "same", "expected": ["team-a"], "forbidden": ["team-b"], "expect_empty": False, "measure_quality": True},
        {"id": "empty", "category": "negative", "query": "不存在", "expected": [], "forbidden": [], "expect_empty": True, "measure_quality": True},
        {"id": "real", "category": "system", "query": "Windows", "expected": [], "forbidden": [], "expect_empty": False, "measure_quality": False},
    ]
    rows = {
        "plain": {"title": "ALPHA.TXT", "path": r"C:\fixture\alpha.txt"},
        "team-a": {"title": "same.txt", "path": r"C:\fixture\team-a\same.txt"},
        "team-b": {"title": "same.txt", "path": r"C:\fixture\team-b"},
    }
    samples = []
    for product in report.PRODUCTS:
        for target in targets:
            samples.append(sample(product, "availability", "availability:" + target["id"], [rows[target["id"]]]))
    return {
        "schema_version": 1,
        "run_id": "run-例",
        "started_at": "2026-01-01T00:00:00Z",
        "finished_at": "2026-01-01T00:01:00Z",
        "parameters": {"repetitions": 1, "poll_ms": 10, "observation_ms": 50, "quiet_ms": 20, "timeout_ms": 1000},
        "applications": {
            "nxtools": {"path": r"C:\NxTools.exe", "file_version": "1.0", "note": "本地"},
            "listary": {"path": r"C:\Listary.exe", "file_version": "6.0"},
        },
        "corpus_root": r"C:\fixture",
        "targets": targets,
        "cases": cases,
        "samples": samples,
        "failure": None,
        "cleanup": {"input_restored": True},
    }


def add_measurements(data, case_id, rows, **kwargs):
    for product in report.PRODUCTS:
        data["samples"].append(sample(product, "measurement", case_id, rows, **kwargs))


class ReportTests(unittest.TestCase):
    def test_percentile_empty_single_and_nearest_rank(self):
        self.assertIsNone(report._percentile([], 0.95))
        self.assertEqual(7, report._percentile([7], 0.95))
        self.assertEqual(3, report._percentile([1, 2, 3, 4], 0.75))
        self.assertEqual(4, report._percentile([1, 2, 3, 4], 0.95))

    def test_zero_latency_retained_none_counted_and_warmup_excluded(self):
        data = base_data()
        data["samples"].append(sample("nxtools", "warmup", "hit", [], first=99))
        data["samples"].append(sample("nxtools", "measurement", "hit", [], first=0))
        data["samples"].append(sample("nxtools", "measurement", "hit", [], repetition=2, first=None))
        timing = report.summarize(data)["timing"]["products"]["nxtools"]
        self.assertEqual(2, timing["measurement_samples"])
        self.assertEqual({"count": 1, "min_ms": 0, "p50_ms": 0, "p95_ms": 0, "max_ms": 0}, timing["first_result_latency"])
        self.assertEqual(1, timing["null_no_result"])

    def test_latest_availability_loss_excludes_case(self):
        data = base_data()
        data["samples"].append(sample("nxtools", "availability", "availability:plain", [], repetition=2))
        add_measurements(data, "hit", [{"title": "alpha.txt", "path": r"C:\fixture\alpha.txt"}])
        summary = report.summarize(data)
        self.assertFalse(next(case for case in summary["cases"] if case["id"] == "hit")["quality_included"])
        self.assertEqual(0, summary["quality"]["products"]["nxtools"]["case_observation_count"])

    def test_asymmetric_availability_is_not_scored(self):
        data = base_data()
        data["samples"] = [s for s in data["samples"] if not (s["product"] == "listary" and s["case_id"] == "availability:plain")]
        add_measurements(data, "hit", [{"title": "alpha.txt", "path": ""}])
        summary = report.summarize(data)
        self.assertEqual(0, summary["quality"]["products"]["listary"]["case_observation_count"])
        self.assertIn("listary:plain", summary["quality"]["excluded_cases"][0]["missing_availability"])

    def test_failed_sample_is_not_an_empty_result_pass(self):
        data = base_data()
        add_measurements(data, "empty", [], error="UIA unavailable", first=None)
        summary = report.summarize(data)
        self.assertEqual(0, summary["quality"]["products"]["nxtools"]["case_success_count"])
        self.assertEqual(1, summary["timing"]["products"]["nxtools"]["errors"])
        self.assertEqual("UIA unavailable", summary["sample_errors"][0]["error"])

    def test_full_path_parent_path_and_duplicate_names_are_disambiguated(self):
        data = base_data()
        rows = [{"title": "same.txt", "path": r"C:/fixture/team-a/same.txt"}]
        rows += [{"title": f"other-{n}", "path": ""} for n in range(4)]
        rows.append({"title": "same.txt", "path": r"C:\fixture\team-b"})
        add_measurements(data, "duplicates", rows)
        result = report.summarize(data)["cases"][1]["products"]["nxtools"]["repetitions"][0]
        self.assertEqual({"team-a": 1}, result["expected_ranks_at5"])
        self.assertEqual(["team-b"], result["forbidden_found"])
        self.assertFalse(result["success"])

    def test_expected_target_at_rank_six_is_not_a_hit(self):
        data = base_data()
        rows = [{"title": f"other-{n}", "path": ""} for n in range(5)]
        rows.append({"title": "alpha.txt", "path": r"C:\fixture\alpha.txt"})
        add_measurements(data, "hit", rows)
        result = report.summarize(data)["cases"][0]["products"]["nxtools"]["repetitions"][0]
        self.assertIsNone(result["expected_ranks_at5"]["plain"])
        self.assertFalse(result["success"])

    def test_false_focus_and_restore_are_errors(self):
        data = base_data()
        data["samples"].append(sample("nxtools", "measurement", "hit", [], focus=False))
        data["samples"].append(sample("listary", "measurement", "hit", [], restored=False))
        summary = report.summarize(data)
        self.assertEqual("focus acquisition failed", summary["sample_errors"][0]["error"])
        self.assertEqual("input restoration failed", summary["sample_errors"][1]["error"])

    def test_failed_samples_do_not_earn_target_hits_or_reciprocal_rank(self):
        data = base_data()
        add_measurements(data, "hit", [{"title": "alpha.txt", "path": ""}], error="focus lost")
        quality = report.summarize(data)["quality"]["products"]["nxtools"]
        self.assertEqual(0, quality["target_hits_at5"])
        self.assertEqual(0, quality["mrr_at5_first_expected"])

    def test_real_system_case_is_evidence_but_not_quality(self):
        data = base_data()
        add_measurements(data, "real", [{"title": "Windows Terminal", "path": r"C:\Windows"}])
        summary = report.summarize(data)
        case = next(item for item in summary["cases"] if item["id"] == "real")
        self.assertFalse(case["quality_included"])
        self.assertEqual("illustrative real-system query", case["exclusion_reason"])
        self.assertEqual(0, summary["quality"]["products"]["nxtools"]["case_observation_count"])
        self.assertIn("repetition 1: observed", report.render_markdown(summary, data))

    def test_paired_timing_uses_complete_jointly_successful_fixture_cases(self):
        data = base_data()
        add_measurements(data, "hit", [{"title": "alpha.txt", "path": ""}], first=0)
        add_measurements(data, "duplicates", [
            {"title": "same.txt", "path": r"C:\fixture\team-a"},
            {"title": "same.txt", "path": r"C:\fixture\team-b"},
        ])
        add_measurements(data, "empty", [], first=None)
        add_measurements(data, "real", [{"title": "Windows", "path": ""}])
        paired = report.summarize(data)["timing"]["paired_successful_fixtures"]
        self.assertEqual(["hit"], paired["case_ids"])
        self.assertEqual(1, paired["products"]["nxtools"]["count"])
        self.assertEqual(0, paired["products"]["listary"]["p50_ms"])
        data["samples"] = [s for s in data["samples"] if not (
            s["phase"] == "measurement" and s["case_id"] == "hit" and s["product"] == "listary"
        )]
        self.assertEqual([], report.summarize(data)["timing"]["paired_successful_fixtures"]["case_ids"])

    def test_target_and_change_timings_exclude_missing_legacy_and_failed_samples(self):
        data = base_data()
        missing = sample("nxtools", "measurement", "hit", [{"title": "other.txt", "path": ""}])
        missing.update(first_target_ms=None, all_targets_ms=None, last_result_change_ms=25)
        found = sample("nxtools", "measurement", "hit", [{"title": "alpha.txt", "path": ""}], repetition=2)
        found.update(first_target_ms=35, all_targets_ms=45, last_result_change_ms=70)
        legacy = sample("nxtools", "measurement", "hit", [], repetition=3, first=None)
        failed = sample("nxtools", "measurement", "hit", [], repetition=4, error="focus lost")
        failed.update(first_target_ms=1, all_targets_ms=1, last_result_change_ms=1)
        data["samples"].extend([missing, found, legacy, failed])
        timing = report.summarize(data)["timing"]["products"]["nxtools"]
        self.assertEqual(2, timing["target_latency_candidates"])
        self.assertEqual(1, timing["target_not_observed"])
        self.assertEqual(35, timing["first_target_latency"]["p50_ms"])
        self.assertEqual(45, timing["all_targets_latency"]["p50_ms"])
        self.assertEqual(2, timing["last_change_latency"]["count"])
        self.assertEqual(25, timing["last_change_latency"]["p50_ms"])
        self.assertIn("Target arrival", report.render_markdown(report.summarize(data), data))

    def test_unicode_rendering_and_cli_outputs_parse(self):
        data = base_data()
        data["failure"] = "部分失败"
        add_measurements(data, "hit", [{"title": "alpha.txt", "path": r"C:\资料\alpha.txt"}], first=0)
        markdown = report.render_markdown(report.summarize(data), data)
        self.assertIn("α", markdown)
        self.assertIn("部分失败", markdown)
        self.assertIn("FileVersion", markdown)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "raw.json"
            output_path = root / "out"
            input_path.write_text(json.dumps(data, ensure_ascii=False), encoding="utf-8")
            self.assertEqual(0, report.main(["--input", str(input_path), "--output-dir", str(output_path)]))
            parsed = json.loads((output_path / "summary.json").read_text(encoding="utf-8"))
            self.assertEqual("run-例", parsed["run"]["run_id"])
            self.assertIn("资料", (output_path / "report.md").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
