# Design — managed ZIP and confirmation

## Architecture / data flow
C# existing native picker returns selected .zip; original PlatformBridge routes one correlated result. Rust Extensions archive reader uses existing workspace zip8.3.0, validates each name/type/declared+actual size under bounded work, resolves root or one wrapper, reads manifest/listed package files into original immutable ImportPreview. Avoid filesystem extraction before confirmation: parsing/decompression is validation, not activation. Existing stage_into writes only validated immutable candidate to managed content-addressed directory, then original locked Core config merge/install commits. Inspect/fix only concrete stage/config failure cleanup introduced by this path; never delete pre-existing snapshots.

Public API should be narrow (`ImportPreview::from_archive` or equivalent), typed archive errors for actionable GUI status. Retain legacy lower-level APIs and old config. Package-managed source snapshot includes manifest for declarative packages too if needed by new import, without changing hash/trust semantics or assuming staged code for non-script actions. Source ZIP removal must not affect launch.

## Reference and adoption
Complete official component index read. Published0.7.1 dialog/dialog.rs::Dialog new/content/title/footer/button_props/on_ok/on_cancel/on_close; DialogContent/Header/Title/Description/Footer and one Base Root/native window. Read exact overlay/render/close/focus APIs before implementation. DescriptionList::columns(1)/label_width/bordered/item gives structured metadata. GroupBox semantic grouping and ListItem/Tag summarize declared feature capabilities. Accordion/AccordionItem::title/open(false)/children contains technical metadata only; warning and permissions not hidden. Use released default Button variant/Medium32, no custom palette, font or drawn frames. Published source `.cargo/registry/.../gpui-component-0.7.1/src/{dialog,description_list.rs,group_box.rs,accordion.rs}` inspected; final chosen composition needs compiled capability regression, not availability claim.

## Layout
One centered bounded dialog, not appended text below import form:
- Header: Import extension / Replace extension; name, version, description.
- Body: package source/runtime summary; declared features grouped; visible permission warning and author reasons.
- Technical details collapsed: package ID/hash/entry/budgets/full source path.
- Footer: Cancel | Import extension (or explicit Replace), fixed outside scroll.
KitDefaultLight/Dark, existing CJK/mono, normal/min windows unchanged. Long body scrolls within dialog. Imported package stays disabled/untrusted; success next steps point to environment/trust/enable.

## Lifecycle / integrity
Preview entity/application generation owns candidate. Dialog accepts only exact current candidate; changed input/new preview/module close/replacement cannot commit stale content. Callbacks update local state then defer original application transaction, pending-before-dispatch, one submission. Keep dialog open on failure with local retry; successful commit retires candidate and presents installed item. Dialog/Root own focus trapping/restoration and Escape; no parallel window or native focus owner. Closing busy transaction must not falsely claim cancellation/rollback after accepted commit; exact policy tested using existing commit semantics.

## Safety / compatibility
ZIP only ordinary UI; archive root and single enclosing folder accepted, no arbitrary multi-package root. Existing path rules, reserved names/link rejection and manifest/code budgets reused. File-count and streaming/decompressed bytes bound even forged ZIP central directory. Reject duplicate/case aliases and encrypted/unsupported compression; fixed typed errors, no payload logging. Same release dependencies and old runtime/trust verification retained. No generic Windows unpack API, reference executable, product startup, package/service changes.
