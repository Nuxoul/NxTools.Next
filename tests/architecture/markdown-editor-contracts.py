#!/usr/bin/env python3
"""Pinned editor provenance and integration source guard; not visual/native acceptance."""
import hashlib
import json
from itertools import product
from pathlib import Path
import re
root=Path(__file__).resolve().parents[2]
vendor=root/'vendor/gpui-kit-markdown-editor-010'
metadata=json.loads((vendor/'NXTOOLS-UPSTREAM.json').read_text('utf-8'))
assert metadata['revision']=='31608fa74bdbeca79d9901bf2daa48ea7ecf1ecc'
assert metadata['modified_files']==['src/element.rs','src/cull.rs','src/serialize.rs']
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
serialize=(vendor/'src/serialize.rs').read_bytes()
old=b'''            body.push_str(
                if spread || bare_nested || (!is_list && !prev_is_list) || prev_is_list {
                    "\\n\\n"
                } else {
                    "\\n"
                },
            );'''
new=b'''            body.push_str(if spread || bare_nested || !is_list || prev_is_list {
                "\\n\\n"
            } else {
                "\\n"
            });'''
assert serialize.count(new)==1 and old not in serialize,'serialize must contain exactly one approved expression/format patch'
assert hashlib.sha256(serialize.replace(new,old)).hexdigest()==metadata['files']['src/serialize.rs'],'serialize delta exceeds one approved expression/format patch'
for spread,bare_nested,is_list,prev_is_list in product((False,True),repeat=4):
    assert (spread or bare_nested or (not is_list and not prev_is_list) or prev_is_list)==(spread or bare_nested or not is_list or prev_is_list)
element=(vendor/'src/element.rs').read_text('utf-8')
formatted=b'if passive {\n                cx.propagate();\n            }'
element_bytes=(vendor/'src/element.rs').read_bytes()
assert element_bytes.count(formatted)==1,'passive link callback formatting missing'
assert hashlib.sha256(element_bytes.replace(formatted,b'if passive { cx.propagate(); }')).hexdigest()=='2a44a51e00c803b278b9b4c925a2c0dbe0625cf94334c7c16658e9c2836660a8','element delta exceeds approved passive baseline plus one format patch'
for token in ['pub fn passive(', 'Role::Document', '.when(!self.passive', 'if self.passive {','cx.propagate()']:
    assert token in element,token
for path in ['ui/history.rs','ui/preview_rich.rs','quick_launch/preview.rs','ui/markdown_edit.rs']:
    source=(root/'rust/apps/nxtools-desktop/src'/path).read_text('utf-8')
    assert 'TextView::' not in source,path
    assert 'markdown_' in source or 'markdown_document' in source,path
manifest=(root/'Cargo.toml').read_text('utf-8')
assert re.search(r'gpui-kit-markdown-editor\s*=.*default-features\s*=\s*false',manifest)
print('PASS pinned editor:57originalfiles; passive element,3stable lowbits,1exact equivalent serializer expression(16cases); four production seams')
print('Scope: provenance/source only; compiled interaction, real HWND/IME/DPI/user acceptance separate.')
