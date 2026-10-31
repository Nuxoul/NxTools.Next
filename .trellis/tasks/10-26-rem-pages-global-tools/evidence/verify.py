"""Read-only Queue08 BLOCKED checkpoint integrity verification, not acceptance.
External snapshots are observations, not locks on other work. No files are written.
"""
from pathlib import Path
import hashlib
import json
import re

T = Path(__file__).resolve().parents[1]
R = T.parents[2]
E = T / 'evidence'
load = lambda path: json.loads(path.read_text(encoding='utf-8-sig'))
sha = lambda data: hashlib.sha256(data).hexdigest()
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
existing = new = callback_count = owner_count = 0
baselines = {}
for row in load(E / 'own-deltas.json'):
    after = (T / row['snapshot']).read_bytes()
    assert sha(after) == row['after_sha256'] and (R / row['path']).read_bytes() == after, row['path']
    if row['kind'] == 'new':
        new += 1
        continue
    existing += 1
    before = (T / row['baseline']).read_bytes()
    assert sha(before) == row['baseline_sha256'], row['path']
    baselines[row['path']] = before
    forward, reverse = before.decode('utf8').splitlines(True), after.decode('utf8').splitlines(True)
    for op in reversed(row['operations']):
        i, j, k, l = (op[key] for key in ['before_start', 'before_end', 'after_start', 'after_end'])
        assert forward[i:j] == op['before_lines'] and reverse[k:l] == op['after_lines'], row['path']
        forward[i:j], reverse[k:l] = op['after_lines'], op['before_lines']
    assert ''.join(forward).encode() == after and ''.join(reverse).encode() == before, row['path']
    path, b, a = row['path'], before.decode(), after.decode()
    if path.endswith('/main_window.rs'):
        assert a.startswith(b), 'main test registration changed original prefix'
    elif path.endswith('.rs'):
        bc, ac = callbacks(b), callbacks(a)
        assert bc == ac, path + ': original callbacks changed'
        callback_count += len(bc); owner_count += 1
        marker = '\n#[cfg(test)]\nmod tests {'
        if marker in b:
            old, current = b[b.rindex(marker):], a[a.rindex(marker):]
            if path.endswith('/network_settings.rs'):
                current = current.split('\n#[cfg(test)]\nmod page_rem_tests;')[0]
            assert norm(old) == norm(current), path + ': old tests changed'
    elif path == 'docs/reference-code/gpui.md':
        assert after.startswith(before), 'reference history changed'
root = 'rust/apps/nxtools-desktop/src/'
path = root + 'network_settings.rs'
b, a = baselines[path].decode(), (R / path).read_text(encoding='utf8')
marker = 'impl NetworkSettingsView {\n    fn render_setting_control('
assert b[:b.index(marker)].replace('use crate::ui::metrics::SPACING_8;\n', '') == a[:a.index(marker)], 'original Global save/constructor owner changed'
path = root + 'main_window/tools_page.rs'
b, a = baselines[path].decode(), (R / path).read_text(encoding='utf8')
marker = 'fn open_tool_settings_window('
assert b[b.index(marker):] == a[a.index(marker):], 'original Tool native lifecycle changed'
path = root + 'main_window/shell.rs'
b, a = baselines[path].decode(), (R / path).read_text(encoding='utf8')
marker = 'pub(crate) fn window_control(&self'
assert b[b.index(marker):] == a[a.index(marker):], 'original sidebar/native actions/control geometry changed'
protected = load(E / 'protected-sources.json')
for row in protected:
    assert sha((R / row['path']).read_bytes()) == row['sha256'], row['path']
history = load(E / 'historical-baseline.json')
for row in history:
    assert sha((R / row['path']).read_bytes()) == row['sha256'], row['path']
for name in ['catalog.rs', 'import_confirmation.rs', 'trust_confirmation.rs']:
    path = root + 'main_window/extensions_page/' + name
    assert (R / path).read_bytes() == (E / 'baselines' / (path + '.baseline')).read_bytes(), path + ': preparation restoration'
status = load(E / 'checkpoint-status.json')
assert status['status'].startswith('INCOMPLETE')
print(f'PASS {len(manifest["evidence"])} checkpoint evidence hashes; {existing} reversible dirty deltas; {new} new test snapshots')
print(f'PASS {callback_count} original callbacks in {owner_count} production files; native/save/old tests; {len(protected)} protected files; {len(history)} historical records; Extensions preparation restored')
print('INCOMPLETE: remaining consumer tests, related suites, strict Clippy/check/fmt NOT RUN because natural Rust idle unavailable. This is integrity evidence only, not acceptance. No files written.')
