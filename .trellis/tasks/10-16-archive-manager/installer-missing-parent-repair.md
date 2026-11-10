# Installer repair — missing vendor/product parents

## Actual failure

The actual Release56 attempt `20261106233632-is-8JZ6TPER2G-tmp` completed detection with
`missing_updater_root`/exit11, received explicit confirmation and ran the preparation helper
elevated. The helper failed in bootstrap_migration. Host correlation
`6b469c5d-0407-4a76-92b1-05437f224745` reports rejected/root_absence/native2 after323.8431ms,
before any service-stop or enrollment event. The extracted manifest, signature and Host exist.

Read-only directory observation confirms both ProgramData/NxTools and NxTools.Next are absent.
`ValidateProductChildDirectory` requires those directories to exist and its original native
CreateFile returns error2. `RequireMissingRoot` is not the source of this exception: it already
handles ordinary missing file/directory results. The prior fixture replaced parent validation
with a callback and therefore did not exercise this supported initial condition.

## Correct boundary

Only explicit missing-root repair uses `ValidateMissingProductParents`. The existing
CommonApplicationData directory is opened with the original native stable-path lease. Any
existing fixed vendor/product parent is checked with the original directory and protected-ACL
validation. Missing product nodes are permitted without creating anything; denied access,
non-directory/reparse objects, bad existing ACLs and a missing base directory still refuse.

The same validation is used at initial repair admission, under the original deployment gate,
and immediately before original provisioning. The original directory creation/ACL hardening,
service registration, signature/key checks, pending initial deployment, publication and ordinary
upgrade/maintenance behavior remain unchanged. Rejection now distinguishes parent_validation
from root_absence. No service, registry, protected directory or installation was modified here.

## Verification

- A compiled regression using the actual Windows directory lease fails with the original
  requirement that missing vendor/product directories already exist. It then passes with the
  corrected validator and reaches the original deployment transaction.
- Missing-bootstrap fixtures now use the actual native parent validation in GUID-owned roots,
  not a no-op callback. Existing-parent file preservation and missing-base refusal also pass.
- Final C# build:0warnings/0errors. Reviewed exact LIST/RUN15, PASS15/SKIP0/FAIL0/not_run0,
  covering missing parents, original repair/refusal, ordinary maintenance, pending deployment,
  signed update/rollback and pinned enrollment. No real SCM/registry operation occurs.
- A separate read-only invocation of the production validator against this machine's actual
  CommonApplicationData returns success with both product parents still absent, directory state
  unchanged and no service operation. This is not installed repair acceptance.

Evidence: `D:\.pi-desktop\data\scratch\ceebe351-d540-4ca3-a460-dadc71f3a7df\installer-missing-parent-repair/`.
It retains the compiled RED, exact final list/run/build and actual readonly-probe output.

## Limits

Real user confirmation/UAC, service restoration and complete installed-file readback still
require actual installation. This repairs the concrete native2 prerequisite failure without
claiming that later native stages have already succeeded. Original Release56 evidence remains
historical; no automatic retry, deletion of recovery records or trust replacement is added.

## Release58

The formal Release58 installer includes this validator. A coherent8,536-file source snapshot and
independently copied Release cache isolate the build from concurrent working-tree changes.
The same snapshot builds with0warnings/0errors and reruns the exact15 recovery fixtures with
PASS15/SKIP0/FAIL0/not_run0. Release compilation, original signing,16Inno scenarios,
5BrowserHost replacement scenarios and complete distribution verification pass.
Published artifacts match the verified build; no actual installation or real service action occurs.
See [Release58](release58-package-outcomes.md) and
`.diagnostics/packaging/20261107-003108-2d48fb71/`.
