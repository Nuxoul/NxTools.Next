#!/usr/bin/env python3
"""Production notification configuration guard; not a measured-FPS acceptance test."""
from pathlib import Path
import re

root = Path(__file__).resolve().parents[2]
source = (root / 'rust/apps/nxtools-desktop/src/notifications/screen.rs').read_text('utf-8')
options = source.split('fn notification_window_options(', 1)[1].split('\nfn open_surface(', 1)[0]
assert re.search(r'inactive_frame_interval\s*:\s*None', options), 'passive notification inherits the 33.333ms inactive frame cap'
surface = source.split('fn open_surface(', 1)[1]
assert 'let options = notification_window_options(display_id)' in surface
assert '.open_window(options,' in surface
assert '#[cfg(debug_assertions)]' in surface and 'preview_cadence::options(options, cx)' in surface
for required in ('focus: false', 'show: false', 'WindowKind::PopUp'):
    assert required in options, required
print('PASS notification production options: passive/hidden and no inactive animation cap')
print('Scope: source configuration only; compiled scheduler/native cadence and user acceptance separate.')
