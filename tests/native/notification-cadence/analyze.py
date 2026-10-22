"""Analyze bounded owned-HWND paints; not physical/user visual acceptance."""
import json
import statistics
import sys
from pathlib import Path

run_file = Path(sys.argv[1])
results = []
for run in json.loads(run_file.read_text(encoding="utf-8-sig")):
    directory = Path(run["directory"])
    sample = json.loads((directory / "cadence.json").read_text())
    native = json.loads((directory / "native.json").read_text())
    marks = dict(sample["milestones_us"])
    frames = sample["frames_us"]
    assert frames and sample["bounded"] and not sample["observer_requests_frames"]
    entry = [v for v in frames if marks["visible"] <= v < marks["dismiss"]]
    exiting = [v for v in frames if marks["dismiss"] <= v <= marks["hidden"]]
    idle = [v for v in frames if marks["visible"] + 800000 < v < marks["dismiss"] - 100000]
    result = {"mode": run["mode"], "refresh_hz": native["operations"][0]["refresh_hz"],
              "idle_frames": len(idle), "hide_after_dismiss_ms": round((marks["hidden"] - marks["dismiss"]) / 1000, 2)}
    for name, times in (("entry", entry), ("exit", exiting)):
        deltas = [(b-a)/1000 for a,b in zip(times, times[1:]) if 1000 < b-a < 100000]
        result[name] = {"frames": len(times), "median_ms": round(statistics.median(deltas), 3) if deltas else None}
    assert not idle, result
    results.append(result)
by_mode = {item["mode"]: item for item in results}
current = by_mode["current"]
legacy = by_mode["legacy"]
assert by_mode["reduced"]["entry"]["frames"] <= 2, "reduced motion must not drive an entrance loop"
if current["refresh_hz"] > 45:
    period = 1000 / current["refresh_hz"]
    assert current["entry"]["median_ms"] is not None
    assert current["entry"]["median_ms"] <= period * 1.8
    assert legacy["entry"]["median_ms"] >= 30
    assert current["exit"]["median_ms"] < legacy["exit"]["median_ms"]
else:
    print("Cadence comparison unavailable at this reported refresh rate; native/idle evidence only.")
output = json.dumps(results, indent=2)
print(output)
(run_file.parent / "summary.json").write_text(output, encoding="utf-8")
