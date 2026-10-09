# CLI quality

Evidence: `repo:rust/apps/nxtools-cli/src/lib.rs#tests`, `repo:rust/apps/nxtools-cli/src/lib.rs#parse_invocation`.

Test actual parse_invocation/parse_args combinations, optional download prefix, --connect-only placement, invalid schedule/IDs/headers, exit mapping and JSON projection. Watch uses authoritative generation/sequence and bounded caller policy; do not fake completion from launch success. Fixtures use synthetic tasks/temp directories, no product autostart. `cargo test -p nxtools-cli --lib` and strict scoped checks follow [verification](../../shared/verification.md).
