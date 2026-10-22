#!/usr/bin/env python3
"""Pinned editor provenance and integration source guard; not visual/native acceptance."""
import hashlib
import json
from pathlib import Path
import re
root=Path(__file__).resolve().parents[2]
vendor=root/'vendor/gpui-kit-markdown-editor-010'
metadata=json.loads((vendor/'NXTOOLS-UPSTREAM.json').read_text('utf-8'))
assert metadata['revision']=='31608fa74bdbeca79d9901bf2daa48ea7ecf1ecc'
assert metadata['modified_files']==['src/element.rs','src/cull.rs']
for name,digest in metadata['files'].items():
    if name not in metadata['modified_files']:
        assert hashlib.sha256((vendor/name).read_bytes()).hexdigest()==digest,name
actual={p.relative_to(vendor).as_posix() for p in vendor.rglob('*') if p.is_file()}
assert actual-set(metadata['files'])=={'NXTOOLS-UPSTREAM.json','NXTOOLS-PATCH.md'}
assert (vendor/'LICENSE-APACHE').is_file()
cull=(vendor/'src/cull.rs').read_text('utf-8')
assert 'isolate_lowest_one' not in cull
assert cull.count('i & i.wrapping_neg()')==3
original_cull=next(value for name,value in metadata['files'].items() if name=='src/cull.rs')
restored=cull.replace('(i & i.wrapping_neg())','(i.isolate_lowest_one())').replace('i & i.wrapping_neg()','i.isolate_lowest_one()')
assert hashlib.sha256(restored.encode()).hexdigest()==original_cull,'cull delta exceeds three approved substitutions'
element=(vendor/'src/element.rs').read_text('utf-8')
for token in ['pub fn passive(', 'Role::Document', '.when(!self.passive', 'if self.passive {','cx.propagate()']:
    assert token in element,token
for path in ['ui/history.rs','ui/preview_rich.rs','quick_launch/preview.rs','ui/markdown_edit.rs']:
    source=(root/'rust/apps/nxtools-desktop/src'/path).read_text('utf-8')
    assert 'TextView::' not in source,path
    assert 'markdown_' in source or 'markdown_document' in source,path
manifest=(root/'Cargo.toml').read_text('utf-8')
assert re.search(r'gpui-kit-markdown-editor\s*=.*default-features\s*=\s*false',manifest)
print('PASS pinned editor:57originalfiles, element passive mode + exactly3stable lowbit substitutions; four production seams')
print('Scope: provenance/source only; compiled interaction, real HWND/IME/DPI/user acceptance separate.')
