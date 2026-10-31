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

Source/provenance/compiledfixture/native/physical/useracceptance remain separate. This patch does not authorize package/install/runtime/service actions.
