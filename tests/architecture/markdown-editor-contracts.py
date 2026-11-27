#!/usr/bin/env python3
"""Pinned editor provenance and integration source guard; not visual/native acceptance."""
import hashlib
import json
from itertools import product
from pathlib import Path
import tomllib
import re
root=Path(__file__).resolve().parents[2]
vendor=root/'vendor/gpui-kit-markdown-editor-010'
metadata=json.loads((vendor/'NXTOOLS-UPSTREAM.json').read_text('utf-8'))
workspace=tomllib.loads((root/'Cargo.toml').read_text('utf-8-sig'))['workspace']
build=(root/'scripts/build.ps1').read_text('utf-8-sig')
assert 'cargo clippy --workspace --all-targets --exclude gpui-kit-markdown-editor --jobs 1 -- -D warnings' in build,'first-party all-target strict lint must retain its scope'
assert 'cargo clippy -p gpui-kit-markdown-editor --lib --jobs 1 -- -D warnings' in build,'fixed editor library must retain strict lint'
assert workspace['dependencies']['gpui-kit-markdown-editor']['path']=='vendor/gpui-kit-markdown-editor-010','product must retain the fixed editor path dependency'
assert metadata['revision']=='31608fa74bdbeca79d9901bf2daa48ea7ecf1ecc'
assert metadata['modified_files']==['src/element.rs','src/cull.rs','src/serialize.rs','Cargo.toml','src/state.rs','src/direct.rs','src/tests/view.rs','src/frame.rs']
for name,digest in metadata['files'].items():
    if name not in metadata['modified_files']:
        assert hashlib.sha256((vendor/name).read_bytes()).hexdigest()==digest,name
actual={p.relative_to(vendor).as_posix() for p in vendor.rglob('*') if p.is_file()}
assert actual-set(metadata['files'])=={'NXTOOLS-UPSTREAM.json','NXTOOLS-PATCH.md'}
assert (vendor/'LICENSE-APACHE').is_file()
editor_manifest=(vendor/'Cargo.toml').read_bytes()
assert editor_manifest.count(b'version = "=0.3.12"')==2,'editor normal/dev backend pins must match'
assert editor_manifest.count(b'"0.10.0"')==2,'editor normal/dev Kit pins must match'
restored_manifest=editor_manifest.replace(b'"=0.3.12"',b'"=0.3.9"').replace(b'"0.10.0"',b'"0.8.0"')
assert hashlib.sha256(restored_manifest).hexdigest()==metadata['files']['Cargo.toml'],'editor manifest delta exceeds exact version bindings'
state=(vendor/'src/state.rs').read_text('utf-8')
binding_pattern=re.compile(r'        KeyBinding::new\(\s*if (m|w|doc) == "(cmd|alt)" \{\s*"((?:cmd|alt)-[^"]+)"\s*\} else \{\s*"(ctrl-[^"]+)"\s*\},\s*(\w+),\s*ctx,?\s*\),')
bindings=list(binding_pattern.finditer(state))
assert len(bindings)==28,'editor must retain exactly 28 platform-specific static bindings'
def restore_binding(match):
    modifier,mac_modifier,mac_key,other_key,action=match.groups()
    assert mac_modifier==('alt' if modifier=='w' else 'cmd')
    suffix=mac_key.removeprefix(mac_modifier+'-')
    assert other_key=='ctrl-'+suffix,'editor platform shortcut changed'
    return '        KeyBinding::new(&format!("{'+modifier+'}-'+suffix+'"), '+action+', ctx),'
restored_state=binding_pattern.sub(restore_binding,state)
assert hashlib.sha256(restored_state.encode()).hexdigest()==metadata['files']['src/state.rs'],'state delta exceeds static shortcut adaptations'
direct=(vendor/'src/direct.rs').read_bytes()
task_marker=b'                task_marker: None,\n'
assert direct.count(task_marker)==1,'direct editor nodes must not invent a source task-marker span'
assert hashlib.sha256(direct.replace(task_marker,b'')).hexdigest()==metadata['files']['src/direct.rs'],'direct delta exceeds one absent source marker'
frame=(vendor/'src/frame.rs').read_text('utf-8')
frame_fragments=json.loads((root/'tests/architecture/markdown-editor-frame-0100.json').read_text('utf-8'))
assert len(frame_fragments)==6,'exactly six approved frame compatibility fragments'
for fragment in frame_fragments:
    assert frame.count(fragment['compatible'])==1,'frame compatibility fragment changed'
    frame=frame.replace(fragment['compatible'],fragment['upstream'])
assert hashlib.sha256(frame.encode()).hexdigest()==metadata['files']['src/frame.rs'],'frame delta exceeds renderer-owned grid adaptation'
view=(vendor/'src/tests/view.rs').read_bytes()
modifier_setup=b'    let modifiers = cx.update(|_, cx| Modifiers::secondary_key(cx));\n'
assert view.count(modifier_setup)==2,'two link fixtures must use platform context'
restored_view=view.replace(modifier_setup,b'')
for adapted,original in [(b'cx.simulate_click(on, modifiers);',b'cx.simulate_click(on, Modifiers::secondary_key());'),(b'cx.simulate_mouse_move(l, None, modifiers);',b'cx.simulate_mouse_move(l, None, Modifiers::secondary_key());')]:
    assert restored_view.count(adapted)==1
    restored_view=restored_view.replace(adapted,original)
assert hashlib.sha256(restored_view).hexdigest()==metadata['files']['src/tests/view.rs'],'view delta exceeds two context modifier fixtures'
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
assert element_bytes.count(b'event.modifiers.secondary(cx)')==2,'editor link modifiers must use current app context'
restored_element=element_bytes.replace(b'event.modifiers.secondary(cx)',b'event.modifiers.secondary()')
adapted_grid=b'                        grid: super::table::grid(doc, *table),\n'
original_grid=adapted_grid+b'''                        aligns: match doc.kind(*table) {
                            Some(super::kind::BlockKindData::Table { aligns }) => aligns.clone(),
                            _ => Vec::new(),
                        },
'''
assert restored_element.count(adapted_grid)==1
restored_element=restored_element.replace(adapted_grid,original_grid)
adapted_layout=b'                frame.layout_tables(&shapes);'
original_layout=b'''                // gpui-kit's cell padding is `px_3`: 0.75rem.
                let pad = window.rem_size() * 0.75;
                let right = frame.content.map(|c| c.right());
                frame.layout_tables(&shapes, pad, right);'''
assert restored_element.count(adapted_layout)==1
restored_element=restored_element.replace(adapted_layout,original_layout)
assert hashlib.sha256(restored_element.replace(formatted,b'if passive { cx.propagate(); }')).hexdigest()=='2a44a51e00c803b278b9b4c925a2c0dbe0625cf94334c7c16658e9c2836660a8','element delta exceeds passive baseline, exact format, two modifiers and renderer-grid call adaptation'
for token in ['pub fn passive(', 'Role::Document', '.when(!self.passive', 'if self.passive {','cx.propagate()']:
    assert token in element,token
for path in ['ui/history.rs','ui/preview_rich.rs','quick_launch/preview.rs','ui/markdown_edit.rs']:
    source=(root/'rust/apps/nxtools-desktop/src'/path).read_text('utf-8')
    assert 'TextView::' not in source,path
    assert 'markdown_' in source or 'markdown_document' in source,path
manifest=(root/'Cargo.toml').read_text('utf-8')
assert re.search(r'gpui-kit-markdown-editor\s*=.*default-features\s*=\s*false',manifest)
print('PASS pinned editor:57originalfiles; exact Kit/pre bindings, two context modifiers,28equivalent static shortcuts,one absent task marker,renderer-owned table grid; passive element,3stable lowbits,1equivalent serializer(16cases); four production seams')
print('Scope: provenance/source only; compiled interaction, real HWND/IME/DPI/user acceptance separate.')
