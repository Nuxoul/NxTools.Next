# W6 declarative command packages and shared management

## Goal / authority
First child of ../10-14-winshun-w6-w9. Declarative package form implemented in shared cross-feature management, not a launcher-only framework. Parent R1–R12/design constraints apply; user approved implementation and source/joint acceptance is complete, not packaged/installed.

## Requirements
- W6-R1: Versioned bounded JSON with package/handler/item identity, module entry metadata and website-search/program/folder/fixed-list actions. W6 actions reuse existing command/template/argv/URL execution; no script interpreter during import or search.
- W6-R2: File picker/read preview validates source/name/version/actions/targets/permissions and conflicts without execution. Import disabled, enabling explicit; no silent replace/manual command overwrite.
- W6-R3: Shared package catalog/registry/provenance/version/disable/remove and Core atomic configuration save. Existing manual commands retain their separate identities/elevation policy; imported actions have no runas.
- W6-R4: Stable namespaced results and command aliases, Unicode/URL encoding/Windows argv/cwd/environment handling via existing owners. Conflict feedback covers enabled packages/manual keywords/duplicate IDs, no priority hiding ordinary conflict.
- W6-R5: Define common manifest/feature operation descriptors needed by second child, but do not ship an enabled incomplete script host. Management under Tools uses reviewed released Settings/Form/Alert; original feature windows/controllers/native budgets remain.

## Acceptance
AC-W6-1: Synthetic valid JSON previews/commits/reopens/enables/disables and leaves unrelated temp-config sections/manual commands intact.
AC-W6-2: Invalid/newer required versions, duplicate/case-colliding keys, traversal/reparse/oversized payload and failed updates retain old registry/state and spawn nothing.
AC-W6-3: Real Finder/GPUI entry and typed native command queue verify website words encoded once, separate argv/cwd, fixed-list filter/stable IDs and disabled packages absent.
AC-W6-4: Actual minimum/default manager UI shows source, warnings and conflict with reachable controls and pointer/keyboard once-only/deferred callbacks; not installed visual certification.

## Evidence / compatibility
Current Finder custom_command.rs keyword matching and prepare_custom_command separates executable/argv/cwd; Core section updater owns atomic config. WindowsPlatformAdapter.LaunchProcess is ordinary action acceptance, not script supervision. WinShun Manifest.h/ZeroCode.cpp are read-only behavior references; own Rust parser/interpretation uses no copied implementation. Existing packages/default empty config preserve current behavior; W6 is not proof of cross-feature script APIs already working.

## Out of scope / dependency
No scripts, runtime/ZIP/network downloads, marketplace, installed package acceptance or general state rewrite. W7–W9 depends on this child's verified identity/import/config management. Parent design.md is canonical for shared interfaces and first-release feature operations; this child's design.md/implement.md scope W6 only. No task start until latest complete parent plan is approved.
