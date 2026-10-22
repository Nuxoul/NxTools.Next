"""Reproducible R8 algorithm-model fixture/benchmark; NOT compiled Rust evidence.
No disk scan, service access, user settings or dependencies. Run from workspace:
python rust/apps/nxtools-indexer/tests/search_synthetic.py
Production comparison is tests::r8_synthetic_search_benchmark (parent owns Cargo).
"""
import json
import statistics
import time
import tracemalloc
from pathlib import Path


def score(pattern, name, kind="fuzzy"):
    candidate = name.lower()
    offset = candidate.find(pattern)
    byte_offset = len(candidate[:offset].encode()) if offset >= 0 else -1
    if kind == "exact":
        return 5000 - byte_offset if offset >= 0 else None
    if kind == "prefix":
        return 6500 if candidate.startswith(pattern) else None
    if kind == "suffix":
        return 6500 if candidate.endswith(pattern) else None
    if candidate == pattern:
        return 7000
    if candidate.startswith(pattern):
        return 6000 - len(candidate.encode()) + len(pattern.encode())
    if offset >= 0:
        return 5000 - byte_offset
    position, first, last = 0, None, 0
    for char in pattern:
        index = candidate.find(char, position)
        if index < 0:
            return None
        current = len(candidate[:index].encode())
        first = current if first is None else first
        last = current
        position = index + 1
    return 2000 - (last - first)


def build(names):
    groups, ids, postings, non_ascii = [], {}, {}, []
    for row, name in enumerate(names):
        uid = ids.get(name)
        if uid is None:
            uid = len(groups)
            ids[name] = uid
            groups.append([name, []])
        groups[uid][1].append(row)
    for uid, (name, _) in enumerate(groups):
        if not name.isascii():
            non_ascii.append(uid)
            continue
        folded = name.lower()
        for gram in set(folded[i:i+3] for i in range(len(folded)-2)):
            postings.setdefault(gram, []).append(uid)
    return groups, postings, non_ascii


def candidates(data, pattern, kind):
    groups, postings, non_ascii = data
    if kind == "fuzzy" or not pattern.isascii() or not 3 <= len(pattern) <= 256:
        return range(len(groups))
    rarest = min((postings.get(pattern[i:i+3], []) for i in range(len(pattern)-2)), key=len)
    count = len(rarest) + len(non_ascii)
    if count > 32768 or count > len(groups)//4:
        return range(len(groups))
    return sorted(rarest + non_ascii)


def scan(names, pattern, kind):
    hits = []
    for row, name in enumerate(names):
        value = score(pattern, name, kind)
        if value is not None:
            hits.append((value, row))
    return sorted(hits, key=lambda hit: (-hit[0], hit[1]))[:800]


def accelerated(data, pattern, kind):
    groups = data[0]
    hits = []
    for uid in candidates(data, pattern, kind):
        name, rows = groups[uid]
        value = score(pattern, name, kind)
        if value is not None:
            hits.extend((value, row) for row in rows)
    return sorted(hits, key=lambda hit: (-hit[0], hit[1]))[:800]


def percentile(samples, quantile):
    return sorted(samples)[min(len(samples)-1, int(len(samples)*quantile))]


def main():
    root = Path(__file__).resolve().parents[4]
    query_source = (root / "rust/crates/nxtools-finder/src/query.rs").read_text(encoding="utf-8")
    index_source = (root / "rust/apps/nxtools-indexer/src/lib.rs").read_text(encoding="utf-8")
    assert "required_ascii_substring" in query_source and "term.kind != MatchKind::Fuzzy" in query_source
    assert "search_name_only_with_data" in index_source and "data.candidates(query)" in index_source
    names = [f"file-{row % 10000:05}.txt" for row in range(100000)]
    names += ["rare-needle.txt", "Kelvin.txt", "计算器.txt", "re-port.txt"]
    tracemalloc.start()
    started = time.perf_counter_ns()
    data = build(names)
    build_ms = (time.perf_counter_ns()-started)/1e6
    retained, peak = tracemalloc.get_traced_memory()
    tracemalloc.stop()
    matrix = [("f", "fuzzy"), ("fi", "fuzzy"), ("file", "fuzzy"), ("fle", "fuzzy"),
              ("计", "fuzzy"), ("needle", "exact"), ("kelvin", "exact"),
              ("00123", "exact"), ("file", "exact"), ("rare", "prefix"), ("txt", "suffix")]
    outcomes = []
    for pattern, kind in matrix:
        expected = scan(names, pattern, kind)
        assert accelerated(data, pattern, kind) == expected, (pattern, kind)
        old, new = [], []
        for _ in range(9):
            started = time.perf_counter_ns()
            assert scan(names, pattern, kind) == expected
            old.append((time.perf_counter_ns()-started)/1e6)
            started = time.perf_counter_ns()
            assert accelerated(data, pattern, kind) == expected
            new.append((time.perf_counter_ns()-started)/1e6)
        outcomes.append({"query": pattern, "kind": kind, "candidate_groups": len(candidates(data, pattern, kind)),
                         "old_p50_ms": round(statistics.median(old), 3), "old_p95_ms": round(percentile(old, .95), 3),
                         "new_p50_ms": round(statistics.median(new), 3), "new_p95_ms": round(percentile(new, .95), 3)})
    print(json.dumps({"evidence": "Python algorithm model, not production Rust timing", "rows": len(names),
                      "groups": len(data[0]), "samples": 9, "build_ms_with_tracemalloc": round(build_ms, 3),
                      "python_retained_bytes": retained, "python_peak_bytes": peak, "matrix": outcomes}, indent=2))


if __name__ == "__main__":
    main()
