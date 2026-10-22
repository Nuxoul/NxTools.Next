"""Owned negative process for tool-stop tests, never a product worker/Job substitute."""
import json
import pathlib
import sys
import time

root = pathlib.Path(sys.argv[1]); mode = sys.argv[2]
partial = root / 'fixture-owned.partial'
with partial.open('xb') as target:
    target.write(b'owned-tool-fixture')
(root / 'fixture-ready').write_text('ready')
# Finite self-expiry even if its parent dies; the test retains force residue deliberately.
deadline = time.monotonic() + 20
while time.monotonic() < deadline:
    if mode == 'cooperate' and (root / 'stop.json').is_file():
        request = json.loads((root / 'stop.json').read_text())
        partial.unlink()
        (root / 'status.json').write_text(json.dumps(dict(schema=2, run_id=request['run_id'], state='Cleaned',
            identities=[], cleanup=dict(status='Confirmed', confirmed=True, scope='controlled_tool_fixture_only'))))
        break
    time.sleep(.005)
