# Local schema_analysis patch

This directory carries the runtime source of `schema_analysis` 0.7.0 from the
crates.io package with checksum
`04dd8374f226542344bb23631f626fc40b99526ef431148270d89b74611d6a49`.
The original MIT and Apache-2.0 licenses and README are retained. Upstream's
runtime source is unchanged except for `src/analysis/schema_seed.rs`, where
object presence tracking replaces repeated linear key scans.

The original manifest's runtime dependencies and optional features are retained.
Its development dependencies/tests are omitted, its readme points to
`README.upstream.md`, and publication is disabled. The parent crate exercises the
modified visitor directly in `tests/upstream_objects.rs`, alongside its full
profiling, policy, evidence and validation suites, including differential
comparisons with the original registry dependency. Those tests run in stable and
MSRV CI. The original README is excluded from this repository's Markdown lint;
this maintenance note is linted normally.

The parent uses a direct path dependency so a consumer of its Git checkout uses
this fix too. A root-only Cargo patch would not propagate to such consumers.
The library exposes no upstream types and this patch adds no public API.

Before registry publication, replace this temporary path dependency with a
released upstream version containing an equivalent fix, or explicitly settle a
maintained packaging alternative. Cargo strips path dependencies when preparing
a registry package, so the current version constraint alone would restore the
unpatched release. Both packages remain `publish = false`.

See the wide-object investigation in `docs/wide-objects.md` at the repository
root for measurements, the source diff, and the upstream adoption plan.
