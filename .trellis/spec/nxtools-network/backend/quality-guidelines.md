# Network quality

Evidence: `repo:rust/crates/nxtools-network/src/lib.rs#operation_snapshot_is_immutable_across_policy_updates`, `repo:rust/crates/nxtools-network/src/lib.rs#manual_snapshot_routes_a_real_http_request_through_the_proxy`.

Use synthetic local proxy, bounded accepted-socket IO and exact route assertion; don't inspect live system proxy secrets. Existing tests prove snapshot update isolation and real request routing. Changes to generation/client reuse additionally run download cache-policy tests. Validate direct/system/manual branches and preserve one-operation one-route. Do not infer certificate policy or external system-proxy instant invalidation from snapshot alone. Run scoped tests/strict lint/check/fmt per [verification](../../shared/verification.md).

Direct/System injected-resolver regression asserts origin-form versus absolute-form actual local requests; this is not OS registry/PAC discovery evidence. Fixture accept/read/write are bounded and accepted sockets restore blocking mode. `repo:rust/crates/nxtools-network/src/lib.rs#direct_and_system_routes_use_real_requests_with_an_injected_resolver` preserves production route branches unchanged.
