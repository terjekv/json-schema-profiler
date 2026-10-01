# Release readiness and compatibility

The implementation targets v0.0.1 and remains unreleased with `publish = false`.
The source repository is [terjekv/json-schema-profiler](https://github.com/terjekv/json-schema-profiler).
No release tag or registry publication has been created; the docs.rs URL remains
an intended publication location. The [hosted CI trial](ci-trial.md) describes
verification of the first PR comparison.

## Rust and dependencies

The MSRV is Rust 1.90.0, matching the current dependency floor (`ordered-float`
and the Gungraun development dependency tree require 1.90). CI tests behavior and
documentation there and runs all checks/bench targets on stable. `Cargo.lock`
is kept for reproducible development and CI.

`schema_analysis` 0.7.0 and `jsonschema` 0.49.9 are pinned while their adapter
contracts are established. Updates require numeric, reference, evidence and
generated-schema regressions plus benchmark review. Upstream implementation types
are private; `serde_json::Value` is the intentional integration boundary.

The current `schema_analysis` dependency is a local copy with a focused
[object-presence patch](wide-objects.md). Before registry publication, adopt a
released upstream fix or settle an explicitly maintained packaging alternative:
Cargo removes path dependencies from published manifests, which would otherwise
restore unpatched 0.7.0. A registry copy of 0.7.0 remains a development-only oracle
for differential tests. Both local packages keep `publish = false`.

The [license inventory](dependency-licenses.md) records locked dependency metadata,
including development tooling. Retain required notices when distributing those
dependencies. This library uses MIT.

## Naming and publication

On 2026-09-24 the public Cargo sparse-index entries for `json-schema-profiler`
and `json_schema_profiler` both returned HTTP 404 (`NoSuchKey`). The crates.io
API returned HTTP 403 from this environment. Neither spelling appeared indexed;
this is not a name reservation or a guarantee of publishing rights. Recheck
availability when intentionally preparing publication.

Before publication, review hosted CI/benchmark comparison results,
prepare dated 0.0.1 changelog/README release entries,
and intentionally remove `publish = false`.

## Compatibility

Rust APIs and serialized reports are consumer interfaces. Changes require
changelog and migration notes. The experimental `suggest` API became fallible:
callers must now handle `Result` before matching `Suggestion`. Candidate reports
store full suggestion options rather than just a global policy.

For the first published 0.0.1, patch updates should preserve documented behavior
and report meaning. Incompatible counting, policy or representation changes need
an explicit versioned migration even while pre-1.0. Counts/schema output are
deterministic; bounded first-seen witnesses depend on input order and limits.
