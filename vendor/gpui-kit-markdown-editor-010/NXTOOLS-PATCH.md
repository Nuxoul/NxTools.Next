# NxTools approved passive presentation and stable Rust compatibility

Original Apache-2.0 source: https://github.com/wabzqem/gpui-kit-markdown-editor at31608fa74bdbeca79d9901bf2daa48ea7ecf1ecc,v0.1.0. ArchiveSHA2566afd8328d46500dcde307e4147b63636a1a5531cde48af9c6a91b96f4837ba90; LICENSE-APACHE and57originalfilehashes retained in NXTOOLS-UPSTREAM.json.

User-approved exact plan2026-10-22 permits only src/element.rs passive presentation: same cached model-to-KitBlockNode renderer/culling/styles, with optional no-input/no-focus/no-caret/no-grip/noneditable-document mode. Default interactive behavior stays unchanged. Local regressions are in the same file.

Additional explicit user approval after actual Rust1.96.0 E0658 permits exactly3replacements in src/cull.rs: unstable i.isolate_lowest_one() -> stable i & i.wrapping_neg(), preserving Fenwick lowbit semantics. No nightly/BOOTSTRAP/toolchain upgrade. No other original source/manifests modified; Cargo optional lock entries do not imply enabledfeatures. Root consumer disablesdefaults and reuses its existing15grammarfeatures. No GPUI framework patches added.

Explicit integration-closeout approval2026-10-22 additionally permits ONE expression in src/serialize.rs::list_item:
`spread || bare_nested || (!is_list && !prev_is_list) || prev_is_list` becomes
`spread || bare_nested || !is_list || prev_is_list`. All16Boolean combinations are identical;
the original file hash must be restored by reversing exactly this one substitution. No serializer
behavior change, lint allow, dependency upgrade or other vendor formatting/edit is authorized.
Supplemental explicit approval in the same closeout permits only rustfmt whitespace in this
serializer push_str fragment and the passive link callback's if block in element.rs. The gate
reverses the exact serializer fragment to its upstream hash and the callback formatting to the
approved passive baseline SHA2562a44a51e00c803b278b9b4c925a2c0dbe0625cf94334c7c16658e9c2836660a8.
No whole-vendor format sweep or further source change is authorized.

## Kit 0.10.0 compatibility
The approved 2026-11-20 migration updates normal/dev dependency bindings in Cargo.toml to Kit 0.10.0/pre =0.3.12 and the two link-click/link-hover event.modifiers.secondary(cx) calls in element.rs. New KeyBinding::new requires static strings:28 state.rs formatted keys use equivalent static cmd/alt-versus-ctrl branches with unchanged actions/contexts; rustfmt affects only those expressions. The direct.rs ListItem bridge adds task_marker: None, matching its existing span: None and the official model-built editor-host nodes. The original revision and all original file hashes remain. The provenance gate reverses these exact substitutions before comparing the manifest/state/direct hashes and prior passive baseline, and checks both platform shortcut mappings. Earlier passive, lowbit, serializer and formatting exceptions remain unchanged; no editor replacement, additional feature or whole-vendor formatting is included.
Two link-click/hover fixtures in src/tests/view.rs obtain Modifiers::secondary_key(cx) through VisualTestContext before dispatch; the event values and assertions remain. Exact reversal restores the original test-file hash.

## Approved Base editor-host geometry compatibility
The editor revision, original model and all original fixtures/assertions remain fixed. Base0.10.0's separately approved five-file patch exposes LeafGeometry::table_cell_bounds() for actual column slack/dividers and row frames, plus table_cell_content_bounds() for content regions resolved from final Div style padding/border. Text position/index already reflect actual alignment. Its src/text/state.rs difference is only preserve_editor_whitespace:false in the complete NodeContext initializer; only editor_host::parse_markdown opts into whitespace fidelity. Ordinary TextView retains upstream0.10.0 behavior.

src/frame.rs consumes real grid boxes, takes bounds' horizontal range from the actual content box while retaining the text box's vertical range, and removes old row_shift. src/element.rs removes only the geometry call's shape-aligns and padding/right reverse-inference parameters; other approved passive/API adaptations remain unchanged.

`tests/architecture/markdown-editor-frame-0100.json` records six readable exact upstream/compatible replacement fragments. The editor gate reverses each fragment and compares the restored frame.rs digest with the original upstream value. The element call is also exactly reversed to the original approved passive baseline. No compatible-source replacement digest or opaque baseline is introduced. The original d20 suite and complete234-test editor library pass unchanged. Native/physical/visual acceptance remains separate.

Source/provenance/compiledfixture/native/physical/useracceptance remain separate. This patch does not authorize package/install/runtime/service actions.
