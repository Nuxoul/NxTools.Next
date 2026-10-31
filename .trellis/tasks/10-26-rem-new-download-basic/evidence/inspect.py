"""Read-only source/evidence verification for the bounded Queue04 slice."""
from pathlib import Path
import hashlib
import json
import re

ROOT = Path(__file__).resolve().parents[4]
TASK = Path(__file__).resolve().parents[1]
OLD = ROOT / '.trellis/tasks/10-25-rem-new-download'
SHA = lambda data: hashlib.sha256(data).hexdigest()
normalize = lambda text: re.sub(r'\s+', '', text)
rows = json.loads((TASK / 'evidence/baseline-manifest.json').read_text(encoding='utf-8-sig'))
checks = []


def section(text, begin, end):
    start = text.index(begin)
    return text[start:text.index(end, start)]


def equal(label, before, after):
    ok = normalize(before) == normalize(after)
    checks.append(dict(label=label, equal=ok, before_sha256=SHA(before.encode()), after_sha256=SHA(after.encode())))
    assert ok, label


def end_args(text, start):
    depth, index, mode, escaped = 1, start, None, False
    while index < len(text):
        char, pair = text[index], text[index:index + 2]
        if mode == 'line':
            if char == '\n': mode = None
        elif mode == 'block':
            if pair == '*/': mode = None; index += 1
        elif mode == 'string':
            if escaped: escaped = False
            elif char == '\\': escaped = True
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
    return [(match.group(1), text[match.start():end_args(text, match.end())])
            for match in re.finditer(r'\.(on_click|on_change|on_open_change|capture_key_down|capture_key_up)\(', text)]


view_path = 'rust/apps/nxtools-desktop/src/downloads/new_download_view.rs'
row = next(row for row in rows if row['path'] == view_path)
before = (TASK / row['baseline']).read_text(encoding='utf8')
after = (ROOT / view_path).read_text(encoding='utf8')
equal('state constructors input Enter IME picker probe TLS submit and footer-action owner',
      section(before, '/// Translates raw engine', 'impl Render for NewDownloadView'),
      section(after, '/// Translates raw engine', 'impl Render for NewDownloadView'))
equal('format native sizes media-selection names and window options',
      section(before, 'pub(crate) fn format_download_size', 'fn new_download_form_field'),
      section(after, 'pub(crate) fn format_download_size', 'fn new_download_form_field'))
equal('category Popover and schedule/calendar internals',
      section(before, '        let schedule_label', '        let footer_leading_actions'),
      section(after, '        let schedule_label', '        let footer_leading_actions'))
equal('media option implementations',
      section(before, 'fn new_download_media_radio', '#[cfg(test)]'),
      section(after, 'fn new_download_media_radio', '#[cfg(test)]'))
equal('resolution Popover HLS DASH calendar and shared error implementation',
      section(before, '    let inspector = if let Some(resolution)', 'pub(crate) fn format_download_size'),
      section(after, '    let inspector = if let Some(resolution)', 'pub(crate) fn format_download_size'))
left, right = callbacks(before), callbacks(after)
assert len(left) == len(right), (len(left), len(right))
for index, ((kind, old), (current_kind, current)) in enumerate(zip(left, right)):
    assert kind == current_kind
    equal(f'original callback {index:02} {kind}', old, current)

protected = []
for row in rows:
    baseline = (TASK / row['baseline']).read_bytes()
    assert SHA(baseline) == row['sha256'], row['path']
    if row['path'] not in [view_path, 'rust/apps/nxtools-desktop/src/downloads/new_download_rem_tests.rs', 'docs/reference-code/download.md']:
        current = (ROOT / row['path']).read_bytes()
        protected.append(dict(path=row['path'], unchanged=SHA(current) == row['sha256'], baseline_sha256=row['sha256'], current_sha256=SHA(current)))
assert all(row['unchanged'] for row in protected), protected

historical_evidence = []
for name in ['checkpoint-01/checkpoint-manifest.json', 'preparation-02/manifest.json']:
    manifest = json.loads((OLD / 'evidence' / name).read_text(encoding='utf-8-sig'))
    for row in manifest['evidence']:
        current = (OLD / row['path']).read_bytes()
        assert SHA(current) == row['sha256'], row['path']
        historical_evidence.append(dict(path=str((OLD / row['path']).relative_to(ROOT)).replace('\\', '/'), sha256=row['sha256']))

historical_observations = []
for name in ['protected-baseline.json', 'frozen-history.json']:
    entries = json.loads((OLD / 'evidence/checkpoint-01' / name).read_text(encoding='utf-8-sig'))
    differences = []
    for row in entries:
        path = ROOT / row['path']
        current_hash = SHA(path.read_bytes()) if path.exists() else None
        if current_hash != row['sha256']:
            differences.append(dict(path=row['path'], historical_sha256=row['sha256'], observed_sha256=current_hash))
    historical_observations.append(dict(manifest=name, count=len(entries), differences=differences))

reference = next(row for row in rows if row['path'] == 'docs/reference-code/download.md')
assert (ROOT / reference['path']).read_bytes().startswith((TASK / reference['baseline']).read_bytes())
result = dict(owner_checks=checks, protected_live_baselines=protected,
              historical_evidence=historical_evidence, historical_source_observations=historical_observations,
              reference_prefix_equal=True)
print(json.dumps(result, indent=2))
print(f'PASS {len(checks)} owner checks, {len(protected)} live protected files, {len(historical_evidence)} immutable checkpoint evidence files')
