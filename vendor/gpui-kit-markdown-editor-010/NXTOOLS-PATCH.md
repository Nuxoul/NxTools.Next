# NxTools approved passive presentation and stable Rust compatibility

Original Apache-2.0 source: https://github.com/wabzqem/gpui-kit-markdown-editor at31608fa74bdbeca79d9901bf2daa48ea7ecf1ecc,v0.1.0. ArchiveSHA2566afd8328d46500dcde307e4147b63636a1a5531cde48af9c6a91b96f4837ba90; LICENSE-APACHE and57originalfilehashes retained in NXTOOLS-UPSTREAM.json.

User-approved exact plan2026-10-22 permits only src/element.rs passive presentation: same cached model-to-KitBlockNode renderer/culling/styles, with optional no-input/no-focus/no-caret/no-grip/noneditable-document mode. Default interactive behavior stays unchanged. Local regressions are in the same file.

Additional explicit user approval after actual Rust1.96.0 E0658 permits exactly3replacements in src/cull.rs: unstable i.isolate_lowest_one() -> stable i & i.wrapping_neg(), preserving Fenwick lowbit semantics. No nightly/BOOTSTRAP/toolchain upgrade. No other original source/manifests modified; Cargo optional lock entries do not imply enabledfeatures. Root consumer disablesdefaults and reuses its existing15grammarfeatures. No GPUI framework patches added.

Source/provenance/compiledfixture/native/physical/useracceptance remain separate. This patch does not authorize package/install/runtime/service actions.
