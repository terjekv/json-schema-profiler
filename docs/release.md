# Release readiness and compatibility

The implementation targets v0.0.1 and remains unreleased with `publish = false`.
Repository/docs.rs metadata name intended locations; no remote repository,
release tag or registry publication has been created. Hosted PR benchmarks remain
to be exercised after repository publication.

## Rust and dependencies

The MSRV is Rust 1.90.0, matching the current dependency floor (`ordered-float`
and the Gungraun development dependency tree require 1.90). CI tests behavior and
documentation there and runs all checks/bench targets on stable. `Cargo.lock`
is kept for reproducible development and CI.

`schema_analysis` 0.7.0 and `jsonschema` 0.49.9 are pinned while their adapter
contracts are established. Updates require numeric, reference, evidence and
generated-schema regressions plus benchmark review. Upstream implementation types
are private; `serde_json::Value` is the intentional integration boundary.

The [license inventory](dependency-licenses.md) records locked dependency metadata,
including development tooling. Retain required notices when distributing those
dependencies. This library uses MIT.

## Naming and publication

On 2026-09-24 the public Cargo sparse-index entries for `json-schema-profiler`
and `json_schema_profiler` both returned HTTP 404 (`NoSuchKey`). The crates.io
API returned HTTP 403 from this environment. Neither spelling appeared indexed;
this is not a name reservation or a guarantee of publishing rights. Recheck
availability when intentionally preparing publication.

Before publication, confirm/configure the intended repository, run hosted
CI/benchmark comparison, prepare dated 0.0.1 changelog/README release entries,
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
