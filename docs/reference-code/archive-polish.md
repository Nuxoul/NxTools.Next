# Archive creation commands and compact preferences

## Reference evidence

Bandizip `D:\Tool\Bandizip\langs\SimpChinese.lang:480–502` defines the source list, add/remove, output name/destination, format/level, password, more options and Start controls. Lines677–708 distinguish filename ZIP, filename7z and editable creation, including selected-item naming for multi-selection. The indexed legacy NxTools search in [archive](archive.md) identified no general archive manager.

The retained raw `bdzshl-delivery-static-routing.txt:10–90` maps ordinary format actions to the selected output component and joins it to the destination. `bandizip-create-p-0x14013d910.json` records the ordinary combined-source task versus separately planned outputs. The connected parser/bootstrap/coordinator chain is documented in [Shell delivery](../../.trellis/tasks/10-31-archive-research/bandizip-shell-delivery.md) and [creation](../../.trellis/tasks/10-31-archive-research/bandizip-create-edit-repair.md). These are reconstructed static observations, not proprietary source or live acceptance.

## Official Explorer boundary

Windows SDK10.0.26100.0 `um\ShObjIdl_core.idl:7535–7588` defines `IExplorerCommand`, IID `A08CE4D0-FA25-44AB-B57C-C7B1C323E0B9`. `GetTitle(IShellItemArray*, LPWSTR**)` at7568 is the actual selection-aware title query; `Invoke(IShellItemArray*, IBindCtx*)` at7583 supplies the complete invocation. Its definition is not marked `local`, unlike adjacent `IDefaultExtractIconInit` at7498–7504. The method order is GetTitle, GetIcon, GetToolTip, GetCanonicalName, GetState, Invoke, GetFlags, EnumSubCommands. The Shell verb's `ExplorerCommandHandler` selects this interface; `MUIVerb` is not a filename template processor.

The existing x64 COM local server remains the activation owner: `ShellSelectionHost.cs::LocalServer.Run` initializes STA, registers class factories with `CLSCTX_LOCAL_SERVER`, resumes and pumps until original object/lock retirement. `ShellSelectionCore.cs::SelectionCommandIdentity` and `ShellExplorerCommandIdentity.cs` share one canonical IUnknown and synchronous final Release. `ShellExplorerCommand.cs` implements the title/state/invocation methods. Quick commands extend that identity; they do not load managed code into Explorer. `GetTitle` must not submit, retire or mutate the selection. `Invoke` binds the supplied array to the same once-only Execute/read/forward owner. Existing grouped editable creation retains IExecuteCommand/IObjectWithSelection.

## Preserved interaction invariants

- One selection, one ordered complete array, one Desktop launch and one original creation job; no timing union, prefix truncation or per-file `%1` creation.
- Quick ZIP/7z use the first ordered source's filename stem (folder keeps its complete name), its parent directory and an explicit fixed format. The queried title and Rust draft use this rule. The editable New Archive command opens the original form.
- Rust owns validation, CreateDraft and job admission; C# owns COM and Windows output. Existing destination/overwrite/password prompts, source leases/checks, cancellation, readback and verified publication remain authoritative.
- Form creation remains in the original operation window/Root, with original owner/focus/close callbacks and fixed footer. The source list gets its own bounded scroll owner; repaint keeps input entities and draft identity. Picker cancellation does not replace the draft.
- Missing saved menu choices use recommendations. Explicit empty/custom choices remain authoritative. No settings load/render/save applies Registry state; only the existing explicit Apply/Revert transaction owns that operation.
- Cross-owner effects remain deferred, including immediate replies. Diagnostics carry operation IDs, count/state/elapsed/native codes, never filenames, paths or secrets.

The production chain is `ArchiveSelectionHandlerPlan` → owned `ExplorerCommandHandler`/`LocalServer32` values → original `ArchiveSelectionClassFactory`/`SelectionCommandIdentity` → `IExplorerCommand::GetTitle` or `Invoke` → original `ShellSelectionReader`/`DesktopIntentForwarder` → `archive_launch::parse_arguments` → original bounded desktop-intent queue → `ArchiveApplication::launch_intent` → `CreateDraft::requests` → `submit_job_with_id` → original FIFO/native ArchiveJob. Quick argv includes `--archive-correlation-id`, making the Shell correlation the native job operation/session ID. Original single-output creation admits one request, with native existing-target confirmation; separate-output naming remains in `create_output_plan.rs` for the editable form's existing each-source option. Explicit five-command saved arrays remain authoritative, as do empty and custom arrays; only absent fields acquire recommendations.

## Released UI composition

The requested form is a bounded compact source list above ordinary output and format controls. Released Component0.8.0 `form/form.rs` exposes Form label layout/columns/Small size, `form/field.rs` exposes Field labels/public child slots, and existing Input/Select/Checkbox/Collapsible/Button keep their owners. Source names use actual Tooltip for full paths. Footer buttons retain Medium32DIP and direct outlined/primary composition.

Preferences follow the existing `downloads/settings_view.rs:2191–2343` composition: Settings → SettingPage → filled SettingGroup panels, shared `aligned_setting_row` for label/help/right-control rows, and original SettingField/Switch/dropdown/NumberInput/action controls. Exact public `SettingItem::render`, `SettingFieldElement::render_field`, typed field variants and `GroupBoxVariant::Fill` are in `vendor/gpui-component-080/src/setting/{item.rs,group.rs,fields/*}`. Standalone commands remain direct/content outlined Buttons; Toolbar.child would force ghost+compact.

## Acceptance boundary

COM vtable fixtures use owned STA and fake Shell arrays only; GPUI tests mount production forms/settings and inspect actual bounds/actions. Source and compiled fixtures do not establish real Explorer activation/proxy behavior, Registry installation, physical pointer/keyboard, DPI, visual or user acceptance. No Registry/Explorer or product execution is required for this slice's development tests.
