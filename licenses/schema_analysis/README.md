# schema_analysis notices and maintained core

The private `src/engine` module is derived from `schema_analysis` 0.7.0
(crates.io archive SHA-256
`04dd8374f226542344bb23631f626fc40b99526ef431148270d89b74611d6a49`).
The unmodified MIT and Apache-2.0 notices and upstream README are retained here
and included in the Cargo archive. Upstream author: QuartzLibrary.

The profiler maintains the aggregation core as part of its own package. This
avoids Cargo dropping a nested path dependency and distributing a different
engine. No separate fork package is published. The published original remains a
development-only dependency for differential tests and benchmark baselines.

The retained source comprises `schema.rs`, `analysis/{mod,field,schema,schema_seed}.rs`,
the context interface, and aggregation/merge traits. Changes from upstream:

- Object presence tracking uses field indices and inline/spill flags, as recorded
  in `patches/schema-analysis-object-presence.patch`.
- Imports point into the private engine module; unused sorting, structural
  comparison and integration helpers are removed.
- CLI, export targets, XML helpers and sampling contexts are omitted. Runtime
  profiling uses only the existing counters context. The default internal context
  is `()`; differential tests adapt the published sampling contexts.
- Upstream aggregation and duplicate/missing/null semantics are retained.
  The JSON adapter still wraps root values to avoid the upstream root-null merge issue.

`src/engine/tests.rs` preserves the object-visitor regressions and compares full
serialized aggregation against registry 0.7.0, including duplicate/escaped keys
and widths through 1,025 fields. Public profiling and independent validation tests
cover the adapter contract. Reevaluate a released upstream fix before updating
this maintained core; keep source provenance, notices and differential tests.
