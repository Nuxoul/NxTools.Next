"""Analyze paired, isolated GPUI material runs; preserve unavailable observations."""
import argparse
import csv
import json
import math
import statistics
from collections import Counter, defaultdict
from pathlib import Path


def load(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def percentile(values, fraction):
    if not values:
        return None
    values = sorted(values)
    position = (len(values) - 1) * fraction
    low = math.floor(position)
    high = math.ceil(position)
    return values[low] + (values[high] - values[low]) * (position - low)


def distribution(values):
    if not values:
        return {"count": 0, "median": None, "min": None, "max": None, "p95": None}
    return {"count": len(values), "median": statistics.median(values),
            "min": min(values), "max": max(values), "p95": percentile(values, .95)}


def role_summary(rows, process_id, frequency, logical_processors):
    selected = []
    for row in rows:
        matches = [sample for sample in row["Samples"] if sample["ProcessId"] == process_id]
        if len(matches) != 1:
            raise ValueError("Each sample must contain exactly one requested process")
        selected.append((row, matches[0]))
    if len(selected) < 2:
        raise ValueError("Insufficient measurement samples")
    statuses = Counter(sample["ProcessStatus"] for _, sample in selected)
    elapsed = (selected[-1][0]["Qpc"] - selected[0][0]["Qpc"]) / frequency
    result = {"samples": len(selected), "observed_seconds": elapsed, "process_status": dict(statuses)}
    result["memory_status"] = dict(Counter(sample.get("MemoryStatus", "not_recorded") for _, sample in selected))
    if len(selected) < 2 or elapsed <= 0:
        raise ValueError("Insufficient positive-duration samples")
    if statuses == {"available": len(selected)}:
        cpu_ms = selected[-1][1]["CpuTotalMilliseconds"] - selected[0][1]["CpuTotalMilliseconds"]
        if cpu_ms < 0:
            raise ValueError("Process CPU counter went backwards")
        result.update(cpu_ms=cpu_ms, cpu_one_core_percent=cpu_ms / (elapsed * 10),
                      cpu_machine_percent=cpu_ms / (elapsed * 10 * logical_processors))
    else:
        result.update(cpu_ms=None, cpu_one_core_percent=None, cpu_machine_percent=None)
    for source, target in [("PrivateBytes", "private_mib"), ("WorkingSetBytes", "working_set_mib"),
                           ("HandleCount", "handles")]:
        scale = 1 if source == "HandleCount" else 1024 ** 2
        result[target] = distribution([sample[source] / scale for _, sample in selected
                                       if sample["ProcessStatus"] == "available" and sample[source] is not None])
    for source, status_key, target, scale in [
        ("GpuMaxEnginePercent", "GpuEngineStatus", "gpu_max_engine_percent", 1),
        ("GpuDedicatedBytes", "GpuDedicatedMemoryStatus", "gpu_dedicated_mib", 1024 ** 2),
        ("GpuSharedBytes", "GpuSharedMemoryStatus", "gpu_shared_mib", 1024 ** 2),
    ]:
        result[target] = distribution([sample[source] / scale for _, sample in selected
                                       if sample[status_key] == "available" and sample[source] is not None])
        result[target]["status"] = dict(Counter(sample[status_key] for _, sample in selected))
        if source == "GpuMaxEnginePercent":
            complete = all(sample[status_key] == "available" and sample[source] is not None for _, sample in selected)
            result[target]["time_weighted_mean"] = (sum(
                second[1][source] * (second[0]["Qpc"] - first[0]["Qpc"])
                for first, second in zip(selected, selected[1:]))
                / (selected[-1][0]["Qpc"] - selected[0][0]["Qpc"])) if complete else None
    return result


def frame_summary(path, process_id, start, end):
    if not path.exists():
        return {"status": "unavailable", "reason": "csv_missing"}
    with path.open(encoding="utf-8-sig", newline="") as stream:
        reader = csv.DictReader(stream)
        fields = reader.fieldnames or []
        qpc_column = "CPUStartQPC" if "CPUStartQPC" in fields else None
        if qpc_column is None or "ProcessID" not in fields:
            return {"status": "unavailable", "reason": "qpc_or_pid_column_missing", "columns": fields}
        chains = defaultdict(list)
        for row in reader:
            if int(row["ProcessID"]) != process_id:
                raise ValueError("PID-filtered frame CSV contains a foreign process")
            timestamp = float(row[qpc_column])
            if start <= timestamp <= end:
                chains[row.get("SwapChainAddress", "unknown")].append(row)
    if not chains:
        return {"status": "unavailable", "reason": "no_phase_frames", "columns": fields}
    result = {"status": "available", "frames": sum(map(len, chains.values())), "swapchains": {}}
    for chain, rows in chains.items():
        metrics = {}
        for field in ("FrameTime", "CPUBusy", "CPUWait", "GPUBusy", "GPUTime", "GPUWait", "DisplayLatency"):
            if field not in fields:
                continue
            values = []
            for row in rows:
                try:
                    value = float(row[field])
                except (ValueError, TypeError):
                    continue
                if math.isfinite(value) and value >= 0:
                    values.append(value)
            metrics[field] = distribution(values)
        result["swapchains"][chain] = {"frames": len(rows), "metrics_ms": metrics,
                                      "present_modes": dict(Counter(row.get("PresentMode", "") for row in rows))}
    return result


def evidence_path(directory, value):
    path = Path(value)
    return path if path.is_absolute() else directory / path


def analyze_run(directory, logical_processors):
    metadata = load(directory / "metadata.json")
    if metadata["Status"] != "passed" or metadata["ExitCode"] != 0:
        raise ValueError(f"Run was not complete: {directory.name}")
    frequency = metadata["StopwatchFrequency"]
    trim = metadata["AnalysisBoundaryTrimSeconds"] * frequency
    start = metadata["PhaseStartedObservedQpc"] + trim
    end = metadata["PhaseFinishedObservedQpc"] - trim
    all_rows = [json.loads(line) for line in (directory / "samples.jsonl").read_text(encoding="utf-8-sig").splitlines()]
    rows = [row for row in all_rows if row["Qpc"] >= start and row["QpcAfter"] <= end]
    if len(rows) < 2:
        raise ValueError(f"No inner measurement interval: {directory.name}")
    receipts = load(directory / "native-receipts.json")
    if not isinstance(receipts, list):
        receipts = [receipts]
    expected = 3 if metadata["Scenario"] == "multiwindow" else 1
    if len(receipts) != expected or not all(receipt["FrameClientOnly"] and receipt["AccentAccepted"]
                                         and receipt["RequestedAccentState"] == (4 if metadata["Material"] == "acrylic" else 0)
                                         for receipt in receipts):
        raise ValueError(f"Invalid native material receipts: {directory.name}")
    events = [json.loads(line)["Event"] for line in (directory / "events.jsonl").read_text(encoding="utf-8-sig").splitlines()]
    finish = next(event for event in events if event["Event"] == "phase_finished")
    if metadata["Scenario"] == "scroll" and finish.get("MaxRenderedRow", 0) <= 0:
        raise ValueError("Scroll did not render later records")
    if metadata["Scenario"] in ("scroll", "input") and finish["Updates"] <= 0:
        raise ValueError("Dynamic workload did not execute")
    roles = {role: role_summary(rows, process_id, frequency, logical_processors)
             for role, process_id in metadata["Roles"].items()}
    if metadata.get("Frames", {}).get("samplerPath"):
        administrative_rows = [json.loads(line) for line in evidence_path(directory, metadata["Frames"]["samplerPath"]).read_text(encoding="utf-8-sig").splitlines()]
        administrative_rows = [row for row in administrative_rows if row["Qpc"] >= start and row["QpcAfter"] <= end]
        for role, process_id in metadata["Frames"]["roles"].items():
            roles[role] = role_summary(administrative_rows, process_id, frequency, logical_processors)
            roles[role]["source"] = "temporary_elevated_observer"
    frames = {}
    for capture in metadata.get("Frames", {}).get("captures", []):
        frames[capture["role"]] = frame_summary(evidence_path(directory, capture["csv"]), capture["targetProcessId"], start, end)
    intervals = [(second["Qpc"] - first["Qpc"]) / frequency for first, second in zip(rows, rows[1:])]
    return {"run": directory.name, "scenario": metadata["Scenario"], "material": metadata["Material"],
            "repetition": metadata["Repetition"], "correlation": metadata["Correlation"],
            "phase_observed_seconds": (end - start + 2 * trim) / frequency,
            "sample_intervals_seconds": distribution(intervals),
            "sampler_duration_ms": distribution([(row["QpcAfter"] - row["Qpc"]) / frequency * 1000 for row in rows]),
            "workload": finish, "roles": roles, "frames": frames}


def paired_summaries(runs):
    pairs = defaultdict(dict)
    for run in runs:
        key = (run["scenario"], run["repetition"])
        if run["material"] in pairs[key]:
            raise ValueError("Duplicate material in paired run")
        pairs[key][run["material"]] = run
    values = defaultdict(list)
    for (scenario, repetition), pair in pairs.items():
        if set(pair) != {"solid", "acrylic"}:
            raise ValueError(f"Incomplete pair: {scenario}/{repetition}")
        for role in pair["solid"]["roles"]:
            for metric in ("cpu_one_core_percent", "cpu_machine_percent", "private_mib", "working_set_mib", "handles",
                           "gpu_max_engine_percent", "gpu_dedicated_mib", "gpu_shared_mib"):
                solid = pair["solid"]["roles"][role][metric]
                acrylic = pair["acrylic"]["roles"][role][metric]
                if isinstance(solid, dict):
                    solid = solid.get("time_weighted_mean", solid["median"])
                    acrylic = acrylic.get("time_weighted_mean", acrylic["median"])
                if solid is not None and acrylic is not None:
                    values[(scenario, role, metric)].append({"repetition": repetition, "solid": solid, "acrylic": acrylic, "delta": acrylic - solid})
    return [{"scenario": scenario, "role": role, "metric": metric, "pairs": pairs,
             "solid": distribution([pair["solid"] for pair in pairs]),
             "acrylic": distribution([pair["acrylic"] for pair in pairs]),
             "paired_delta": distribution([pair["delta"] for pair in pairs])}
            for (scenario, role, metric), pairs in sorted(values.items())]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--logical-processors", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    runs = [analyze_run(path.parent, arguments.logical_processors) for path in sorted(arguments.directory.glob("*/metadata.json"))]
    if not runs:
        raise ValueError("No completed benchmark runs")
    result = {"schema": 1, "logical_processors": arguments.logical_processors, "runs": runs, "paired": paired_summaries(runs)}
    arguments.output.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"Analyzed {len(runs)} runs and {len(result['paired'])} paired metric groups: {arguments.output}")


if __name__ == "__main__":
    main()
