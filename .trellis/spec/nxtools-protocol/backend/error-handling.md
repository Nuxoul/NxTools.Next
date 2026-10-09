# Protocol errors

Evidence: `repo:rust/crates/nxtools-protocol/src/lib.rs#ProtocolError`, `repo:rust/crates/nxtools-protocol/src/lib.rs#decode_line`.

Reject oversized encoded/decoded frame and unsupported version before treating payload as a current Message. JSON parse error remains typed; do not silently reinterpret an unknown version/action. Feature native results preserve success/error/native_error_code/data and their accepted-vs-applied meaning; schema decode is not operation success. Error debug can echo malformed data downstream: log fixed category/size/version, never the raw frame. [Shared contracts](../../shared/engineering.md).
