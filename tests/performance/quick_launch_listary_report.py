#!/usr/bin/env python3
"""Summarize an offline NxTools Quick Launch/Listary black-box benchmark."""

import argparse
import json
import math
import ntpath
from pathlib import Path

PRODUCTS = ("nxtools", "listary")


def _percentile(values, proportion):
    if not values:
        return None
    ordered = sorted(values)
    return ordered[math.ceil(len(ordered) * proportion) - 1]


def _timings(values):
    return {
        "count": len(values),
        "min_ms": min(values) if values else None,
        "p50_ms": _percentile(values, 0.50),
        "p95_ms": _percentile(values, 0.95),
        "max_ms": max(values) if values else None,
    }


def _normalized_path(value):
    return value.replace("/", "\\").rstrip("\\").casefold()


def _row_matches_target(row, target):
    if row.get("title", "").casefold() != target["name"].casefold():
        return False
    suffix = _normalized_path(target.get("parent_suffix", ""))
    if not suffix:
        return True
    path = _normalized_path(row.get("path", ""))
    parent = ntpath.dirname(path).rstrip("\\") if ntpath.basename(path) == target["name"].casefold() else path
    return parent == suffix or parent.endswith("\\" + suffix)


def _sample_error(sample):
    if sample.get("error") is not None:
        return str(sample["error"])
    if not sample.get("focus_ok", False):
        return "focus acquisition failed"
    if not sample.get("input_restored", False):
        return "input restoration failed"
    return None


def _availability(data, targets):
    latest = {}
    for sample in data["samples"]:
        if sample["phase"] == "availability":
            target_id = sample["case_id"].removeprefix("availability:")
            if target_id in targets:
                latest[(sample["product"], target_id)] = sample
    products = {}
    for product in PRODUCTS:
        results = {}
        for target_id, target in targets.items():
            sample = latest.get((product, target_id))
            available = bool(
                sample
                and _sample_error(sample) is None
                and any(_row_matches_target(row, target) for row in sample["rows"])
            )
            results[target_id] = {
                "available": available,
                "repetition": sample.get("repetition") if sample else None,
                "error": _sample_error(sample) if sample else "no availability sample",
            }
        products[product] = {
            "available_targets": sum(item["available"] for item in results.values()),
            "total_targets": len(results),
            "targets": results,
        }
    joint = sum(
        all(products[product]["targets"][target_id]["available"] for product in PRODUCTS)
        for target_id in targets
    )
    return {"products": products, "jointly_available_targets": joint, "total_targets": len(targets)}


def _case_eligibility(case, availability):
    missing = []
    for target_id in dict.fromkeys(case["expected"] + case["forbidden"]):
        for product in PRODUCTS:
            if not availability["products"][product]["targets"][target_id]["available"]:
                missing.append(f"{product}:{target_id}")
    return not missing, missing


def _measurement_result(sample, case, targets):
    ranks = {}
    for target_id in case["expected"]:
        ranks[target_id] = next(
            (rank for rank, row in enumerate(sample["rows"][:5], 1) if _row_matches_target(row, targets[target_id])),
            None,
        )
    forbidden = [
        target_id
        for target_id in case["forbidden"]
        if any(_row_matches_target(row, targets[target_id]) for row in sample["rows"])
    ]
    error = _sample_error(sample)
    return {
        "repetition": sample["repetition"],
        "success": error is None and all(ranks.values()) and not forbidden and (not case["expect_empty"] or not sample["rows"]),
        "error": error,
        "first_result_ms": sample["first_result_ms"],
        "first_target_ms": sample.get("first_target_ms"),
        "all_targets_ms": sample.get("all_targets_ms"),
        "last_result_change_ms": sample.get("last_result_change_ms"),
        "expected_ranks_at5": ranks,
        "forbidden_found": forbidden,
        "visible_row_count": len(sample["rows"]),
        "top5": sample["rows"][:5],
    }


def summarize(data):
    """Build a JSON-serializable benchmark summary from the fixed raw schema."""
    targets = {target["id"]: target for target in data["targets"]}
    cases = {case["id"]: case for case in data["cases"]}
    availability = _availability(data, targets)
    measurements = [sample for sample in data["samples"] if sample["phase"] == "measurement"]
    case_summaries = []
    excluded = []
    for case in data["cases"]:
        eligible, missing = _case_eligibility(case, availability)
        included = bool(case["measure_quality"] and eligible)
        if case["measure_quality"] and not eligible:
            excluded.append({"case_id": case["id"], "missing_availability": missing})
        products = {
            product: {
                "repetitions": [
                    _measurement_result(sample, case, targets)
                    for sample in measurements
                    if sample["product"] == product and sample["case_id"] == case["id"]
                ]
            }
            for product in PRODUCTS
        }
        case_summaries.append(
            {
                "id": case["id"],
                "category": case["category"],
                "query": case["query"],
                "measure_quality": case["measure_quality"],
                "quality_included": included,
                "exclusion_reason": "illustrative real-system query" if not case["measure_quality"] else (", ".join(missing) or None),
                "products": products,
            }
        )

    quality_products, timing_products = {}, {}
    sample_errors = []
    for sample in data["samples"]:
        error = _sample_error(sample)
        if error is not None:
            sample_errors.append({"product": sample["product"], "phase": sample["phase"], "case_id": sample["case_id"], "repetition": sample["repetition"], "error": error})
    for product in PRODUCTS:
        included_results, reciprocal_ranks = [], []
        target_hits = target_opportunities = 0
        for case_summary in case_summaries:
            if not case_summary["quality_included"]:
                continue
            case = cases[case_summary["id"]]
            results = case_summary["products"][product]["repetitions"]
            included_results.extend(results)
            for result in results:
                target_opportunities += len(case["expected"])
                if result["error"] is None:
                    target_hits += sum(rank is not None for rank in result["expected_ranks_at5"].values())
                if case["expected"]:
                    rank = result["expected_ranks_at5"][case["expected"][0]] if result["error"] is None else None
                    reciprocal_ranks.append(0 if rank is None else 1 / rank)
        successes = sum(result["success"] for result in included_results)
        quality_products[product] = {
            "case_success_count": successes,
            "case_observation_count": len(included_results),
            "case_success_rate": successes / len(included_results) if included_results else None,
            "target_hits_at5": target_hits,
            "target_opportunities_at5": target_opportunities,
            "mrr_at5_first_expected": sum(reciprocal_ranks) / len(reciprocal_ranks) if reciprocal_ranks else None,
        }

        product_samples = [sample for sample in measurements if sample["product"] == product]
        first_results, input_sets = [], []
        no_result = errors = positive_candidates = 0
        for sample in product_samples:
            error = _sample_error(sample)
            if error is not None:
                errors += 1
                continue
            input_sets.append(sample["input_set_ms"])
            if cases[sample["case_id"]]["expect_empty"]:
                continue
            positive_candidates += 1
            if sample["first_result_ms"] is None:
                no_result += 1
            else:
                first_results.append(sample["first_result_ms"])
        valid_positive = [sample for sample in product_samples if _sample_error(sample) is None and not cases[sample["case_id"]]["expect_empty"]]
        target_samples = [sample for sample in valid_positive if cases[sample["case_id"]]["expected"] and "first_target_ms" in sample]
        timing_products[product] = {
            "measurement_samples": len(product_samples),
            "errors": errors,
            "positive_latency_candidates": positive_candidates,
            "null_no_result": no_result,
            "first_result_latency": _timings(first_results),
            "input_set_latency": _timings(input_sets),
            "target_latency_candidates": len(target_samples),
            "target_not_observed": sum(sample["first_target_ms"] is None for sample in target_samples),
            "first_target_latency": _timings([sample["first_target_ms"] for sample in target_samples if sample["first_target_ms"] is not None]),
            "all_targets_latency": _timings([sample["all_targets_ms"] for sample in target_samples if sample.get("all_targets_ms") is not None]),
            "last_change_latency": _timings([sample["last_result_change_ms"] for sample in valid_positive if sample.get("last_result_change_ms") is not None]),
        }

    repetitions = data["parameters"]["repetitions"]
    paired_case_ids = []
    for case_summary in case_summaries:
        case = cases[case_summary["id"]]
        if not case_summary["quality_included"] or case["expect_empty"]:
            continue
        complete_success = all(
            len(case_summary["products"][product]["repetitions"]) == repetitions
            and {item["repetition"] for item in case_summary["products"][product]["repetitions"]} == set(range(1, repetitions + 1))
            and all(item["success"] and item["first_result_ms"] is not None for item in case_summary["products"][product]["repetitions"])
            for product in PRODUCTS
        )
        if complete_success:
            paired_case_ids.append(case["id"])
    paired_timing = {
        "case_ids": paired_case_ids,
        "products": {
            product: _timings([
                sample["first_result_ms"] for sample in measurements
                if sample["product"] == product and sample["case_id"] in paired_case_ids
            ])
            for product in PRODUCTS
        },
    }

    return {
        "schema_version": 1,
        "run": {key: data[key] for key in ("run_id", "started_at", "finished_at", "failure", "parameters", "corpus_root", "cleanup")},
        "applications": data["applications"],
        "counts": {"targets": len(targets), "cases": len(cases), "measurement_samples": len(measurements)},
        "availability": availability,
        "quality": {
            "included_case_count": sum(case["quality_included"] for case in case_summaries),
            "excluded_cases": excluded,
            "products": quality_products,
        },
        "timing": {"products": timing_products, "paired_successful_fixtures": paired_timing},
        "cases": case_summaries,
        "sample_errors": sample_errors,
    }


def _display(value):
    return "—" if value is None else str(value)


def _rate(value):
    return "—" if value is None else f"{value:.1%}"


def _cell(value):
    return str(value).replace("|", "\\|").replace("\n", " ")


def render_markdown(summary, data):
    """Render the summary as an evidence-oriented Markdown report."""
    run = summary["run"]
    lines = [
        "# Quick Launch external benchmark report", "", "## Run", "",
        f"- Run ID: `{_cell(run['run_id'])}`",
        f"- Started: `{_cell(run['started_at'])}`",
        f"- Finished: `{_cell(run['finished_at'])}`",
        f"- Run failure: {_cell(run['failure']) if run['failure'] is not None else 'none'}",
        f"- Corpus root: `{_cell(run['corpus_root'])}`",
        f"- Parameters: `{_cell(json.dumps(run['parameters'], ensure_ascii=False, sort_keys=True))}`",
        f"- Cleanup: `{_cell(json.dumps(run['cleanup'], ensure_ascii=False, sort_keys=True))}`", "",
        "| Product | Executable | FileVersion |", "| --- | --- | --- |",
    ]
    for product in PRODUCTS:
        app = summary["applications"][product]
        lines.append(f"| {product} | `{_cell(app['path'])}` | `{_cell(app['file_version'])}` |")

    availability = summary["availability"]
    lines += ["", "## Fixture availability", "", "Quality membership requires the latest literal availability observation for every referenced target in both products.", "", "| Product | Available targets | Index coverage |", "| --- | ---: | ---: |"]
    for product in PRODUCTS:
        item = availability["products"][product]
        coverage = item["available_targets"] / item["total_targets"] if item["total_targets"] else None
        lines.append(f"| {product} | {item['available_targets']}/{item['total_targets']} | {_rate(coverage)} |")
    lines.append(f"\nJointly available: {availability['jointly_available_targets']}/{availability['total_targets']} targets.")

    lines += ["", "## Common top-5 quality", "", "Only quality-enabled cases whose expected and forbidden fixtures were available in both products are counted.", "", "| Product | Successful case observations | Success rate | Target hits@5 | MRR@5 (first expected) |", "| --- | ---: | ---: | ---: | ---: |"]
    for product in PRODUCTS:
        item = summary["quality"]["products"][product]
        lines.append(f"| {product} | {item['case_success_count']}/{item['case_observation_count']} | {_rate(item['case_success_rate'])} | {item['target_hits_at5']}/{item['target_opportunities_at5']} | {_display(item['mrr_at5_first_expected'])} |")
    if summary["quality"]["excluded_cases"]:
        lines += ["", "Excluded quality cases:"]
        for item in summary["quality"]["excluded_cases"]:
            lines.append(f"- `{_cell(item['case_id'])}`: {_cell(', '.join(item['missing_availability']))}")

    lines += ["", "## Timing", "", "First-result latency measures input SetValue to an accessible result observed by UI Automation polling. It is not physical typing latency, paint latency, or native engine execution time, and is conditional on a result being observed.", "", "| Product | Measurement samples | Errors | No result | First-result n | p50 ms | p95 ms | Input-set n |", "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"]
    for product in PRODUCTS:
        item = summary["timing"]["products"][product]
        latency = item["first_result_latency"]
        lines.append(f"| {product} | {item['measurement_samples']} | {item['errors']} | {item['null_no_result']} | {latency['count']} | {_display(latency['p50_ms'])} | {_display(latency['p95_ms'])} | {item['input_set_latency']['count']} |")

    if any("first_target_ms" in sample for sample in data["samples"] if sample["phase"] == "measurement"):
        lines += ["", "## Target arrival and observed list changes", "", "Target arrival uses the declared target names and parent identities. The last-change timestamp is the last content/order change seen inside the observation window, not provider completion or the fixed observation duration.", "", "| Product | Metric | n | p50 ms | p95 ms |", "| --- | --- | ---: | ---: | ---: |"]
        for product in PRODUCTS:
            for key, label in (("first_target_latency", "First declared target"), ("all_targets_latency", "All declared targets"), ("last_change_latency", "Last observed list change")):
                item = summary["timing"]["products"][product][key]
                lines.append(f"| {product} | {label} | {item['count']} | {_display(item['p50_ms'])} | {_display(item['p95_ms'])} |")
            missing = summary["timing"]["products"][product]
            lines.append(f"\n{product}: targets not observed {missing['target_not_observed']}/{missing['target_latency_candidates']} instrumented target queries.")

    paired = summary["timing"]["paired_successful_fixtures"]
    lines += ["", "## Paired successful-fixture timing", "", "The same controlled queries must pass top-5 quality in every measured repetition in both products. Empty-result checks and illustrative system queries are excluded. This is a jointly successful subset, not an overall speed score.", "", "Case IDs: " + ", ".join(f"`{case_id}`" for case_id in paired["case_ids"]), "", "| Product | Matched samples | p50 ms | p95 ms |", "| --- | ---: | ---: | ---: |"]
    for product in PRODUCTS:
        item = paired["products"][product]
        lines.append(f"| {product} | {item['count']} | {_display(item['p50_ms'])} | {_display(item['p95_ms'])} |")

    lines += ["", "## Case evidence", ""]
    for case in summary["cases"]:
        status = "included" if case["quality_included"] else f"excluded: {case['exclusion_reason']}"
        lines += [f"### `{_cell(case['id'])}`", "", f"Query: `{_cell(case['query'])}`  ", f"Quality: {status}", ""]
        for product in PRODUCTS:
            lines.append(f"**{product}**")
            repetitions = case["products"][product]["repetitions"]
            if not repetitions:
                lines.append("\nNo measurement samples.\n")
                continue
            for result in repetitions:
                rows = "; ".join(f"{row.get('title', '')} — {row.get('path', '')}" for row in result["top5"]) or "(no visible rows)"
                detail = f"; error: {result['error']}" if result["error"] else ""
                outcome = ("pass" if result["success"] else "fail") if case["quality_included"] else ("error" if result["error"] else "observed")
                lines.append(f"- repetition {result['repetition']}: {outcome}; first result {_display(result['first_result_ms'])} ms{_cell(detail)}; top 5: {_cell(rows)}")
                if result["last_result_change_ms"] is not None:
                    lines.append(f"  Target timing: first {_display(result['first_target_ms'])} ms; all {_display(result['all_targets_ms'])} ms; last observed change {_display(result['last_result_change_ms'])} ms.")
            lines.append("")

    lines += ["## Sample errors", ""]
    if summary["sample_errors"]:
        for item in summary["sample_errors"]:
            lines.append(f"- {item['product']} {item['phase']} `{_cell(item['case_id'])}` repetition {item['repetition']}: {_cell(item['error'])}")
    else:
        lines.append("No sample errors were recorded.")
    lines += [
        "", "## Limitations", "",
        "- Search results were not opened; measurements used value updates on the installed launchers.",
        "- Existing current-configuration history is not a clean-room normalized baseline, so the report does not declare an overall winner from conditional or incomparable measurements.",
        "- Visible UI Automation rows are evidence about the captured result window, not total recall of either product's index.",
        "- Syntax cases measure compatibility with the manifest's query strings; product-specific equivalent expressions were not substituted.",
        "- Required-target success does not penalize every unrelated returned row; only designated forbidden targets are explicit negative checks.",
        "- Fixture membership is gated by literal availability in both products; title and parent-suffix matching is exact apart from path separator and case normalization.",
        "- FileVersion identifies the inspected executable but does not prove that every source worktree change was packaged into it.",
    ]
    return "\n".join(lines) + "\n"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    args = parser.parse_args(argv)
    with args.input.open("r", encoding="utf-8") as handle:
        data = json.load(handle)
    summary = summarize(data)
    args.output_dir.mkdir(parents=True, exist_ok=True)
    with (args.output_dir / "summary.json").open("w", encoding="utf-8") as handle:
        json.dump(summary, handle, ensure_ascii=False, indent=2)
        handle.write("\n")
    (args.output_dir / "report.md").write_text(render_markdown(summary, data), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
