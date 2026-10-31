"""Verify the Queue04 frozen delivery without modifying files."""
from pathlib import Path
import hashlib
import json
import subprocess
import sys

TASK = Path(__file__).resolve().parents[1]
ROOT = TASK.parents[2]
SHA = lambda data: hashlib.sha256(data).hexdigest()
load = lambda path: json.loads(path.read_text(encoding='utf-8-sig'))
manifest = load(TASK / 'evidence/manifest.json')
for row in manifest['evidence']:
    data = (TASK / row['path']).read_bytes()
    assert SHA(data) == row['sha256'], row['path']
    assert len(data) == row['bytes'], row['path']
for row in manifest['sources'] + manifest['documents']:
    assert SHA((ROOT / row['path']).read_bytes()) == row['sha256'], row['path']
for row in load(TASK / 'evidence/own-deltas.json'):
    before = (TASK / row['baseline']).read_bytes()
    after = (TASK / row['snapshot']).read_bytes()
    assert SHA(before) == row['baseline_sha256']
    assert SHA(after) == row['after_sha256']
    forward = before.decode('utf8').splitlines(keepends=True)
    reverse = after.decode('utf8').splitlines(keepends=True)
    for operation in reversed(row['operations']):
        a, b = operation['before_start'], operation['before_end']
        c, d = operation['after_start'], operation['after_end']
        assert forward[a:b] == operation['before_lines']
        assert reverse[c:d] == operation['after_lines']
        forward[a:b] = operation['after_lines']
        reverse[c:d] = operation['before_lines']
    assert ''.join(forward).encode('utf8') == after
    assert ''.join(reverse).encode('utf8') == before
    assert (ROOT / row['path']).read_bytes() == after
result = subprocess.run([sys.executable, str(TASK / 'evidence/inspect.py')], cwd=ROOT, capture_output=True, text=True)
assert result.returncode == 0, result.stderr
print(result.stdout.splitlines()[-1])
print(f"PASS {len(manifest['evidence'])} evidence hashes, {len(manifest['sources'])} live sources, three forward/reverse dirty-baseline deltas; no files written")
