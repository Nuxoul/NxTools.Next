# Boundaries

Children edit disjoint production files. Parent owns shared C# adapter/tests, installer/architecture scripts and integration verification. No recursive delegation. Reference migration requires note/source invariants and red boundary test before code. Rust owns state/layout; C# Windows API; GPUI presentation.

Window fixes first; independent Rust engine, clipboard and indexer remediation may run concurrently. All Cargo commands coordinated by parent with jobs=1/incremental=0/codegen-units=256; delegates do not run builds. Indexer authorization separates read and privileged mutation and validates actual client process, not caller-controlled payload. Policy/design uncertainty is reported before widening.
