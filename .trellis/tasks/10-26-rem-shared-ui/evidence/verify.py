"""Read-only Queue07 evidence, dirty-delta replay and owner/history verification.
Run from any directory. External observations are immutable dated evidence, not locks.
"""
from pathlib import Path
import hashlib
import json
import re

T = Path(__file__).resolve().parents[1]
R = T.parents[2]
E = T / 'evidence'
sha = lambda data: hashlib.sha256(data).hexdigest()
load = lambda path: json.loads(path.read_text(encoding='utf-8-sig'))
norm = lambda text: re.sub(r'\s+', '', text)

def end_args(text, index):
    depth, mode, escape = 1, None, False
    while index < len(text):
        char, pair = text[index], text[index:index + 2]
        if mode == 'line':
            if char == '\n': mode = None
        elif mode == 'block':
            if pair == '*/': mode = None; index += 1
        elif mode == 'string':
            if escape: escape = False
            elif char == '\\': escape = True
            elif char == '"': mode = None
        elif pair == '//': mode = 'line'; index += 1
        elif pair == '/*': mode = 'block'; index += 1
        elif char == '"': mode = 'string'
        elif char == '(': depth += 1
        elif char == ')':
            depth -= 1
            if depth == 0: return index + 1
        index += 1
    raise AssertionError('unbalanced callback')

def callbacks(text):
    pattern = r'\.(on_click|on_change|on_open_change|on_mouse_down|on_key_down|on_key_up|capture_key_down|capture_key_up)\('
    return [(match.group(1), norm(text[match.start():end_args(text, match.end())]))
            for match in re.finditer(pattern, text)]

manifest = load(E / 'manifest.json')
for row in manifest['evidence']:
    data = (T / row['path']).read_bytes()
    assert len(data) == row['bytes'] and sha(data) == row['sha256'], row['path']
for row in manifest['sources'] + manifest['documents']:
    assert sha((R / row['path']).read_bytes()) == row['sha256'], row['path']
existing = new = callback_count = owner_files = 0
for row in load(E / 'own-deltas.json'):
    after = (T / row['snapshot']).read_bytes()
    assert sha(after) == row['after_sha256'] and (R / row['path']).read_bytes() == after, row['path']
    assert b'DEBUG-SHARED-REM' not in after, row['path']
    if row['kind'] == 'new':
        new += 1
        continue
    existing += 1
    before = (T / row['baseline']).read_bytes()
    assert sha(before) == row['baseline_sha256'], row['path']
    forward, reverse = before.decode('utf8').splitlines(True), after.decode('utf8').splitlines(True)
    for op in reversed(row['operations']):
        i, j, k, l = (op[key] for key in ['before_start', 'before_end', 'after_start', 'after_end'])
        assert forward[i:j] == op['before_lines'] and reverse[k:l] == op['after_lines'], row['path']
        forward[i:j], reverse[k:l] = op['after_lines'], op['before_lines']
    assert ''.join(forward).encode() == after and ''.join(reverse).encode() == before, row['path']
    path, b, a = row['path'], before.decode(), after.decode()
    if path.endswith(('ui/efficiency_tests.rs', 'network_settings/kit_migration_tests.rs', 'main_window.rs', 'ui/components/mod.rs')):
        assert a.startswith(b), path + ': original bytes changed instead of appended tests'
    elif path.endswith('.rs'):
        bc, ac = callbacks(b), callbacks(a)
        assert bc == ac, path + ': original callback changed'
        callback_count += len(bc); owner_files += 1
        marker = '\n#[cfg(test)]\nmod tests {'
        if marker in b:
            assert norm(a[a.rindex(marker):]) == norm(b[b.rindex(marker):]), path + ': old test suffix changed'
    elif path == 'docs/reference-code/gpui.md':
        assert after.startswith(before), 'reference history rewritten'
protected = load(E / 'protected-sources.json')
for row in protected:
    assert sha((R / row['path']).read_bytes()) == row['sha256'], row['path']
history = load(E / 'historical-baseline.json')
for row in history:
    assert sha((R / row['path']).read_bytes()) == row['sha256'], row['path']
print(f'PASS {len(manifest["evidence"])} evidence hashes; {existing} reversible existing dirty deltas; {new} new test snapshots')
print(f'PASS {callback_count} original callbacks in {owner_files} production files; old tests/reference prefix; {len(protected)} protected Desktop sources; {len(history)} historical records')
print('Production-only strict Clippy remains FAILED on task-start-identical DecodedMedia.animated; test strict/check/fmt pass. No full Desktop/native/visual acceptance claimed. No files written.')
