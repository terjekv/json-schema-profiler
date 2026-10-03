# Release readiness and compatibility

The implementation targets v0.0.1 and remains unreleased with `publish = false`.
The source repository is [terjekv/json-schema-profiler](https://github.com/terjekv/json-schema-profiler).
Publication remains intentionally disabled. The 2026-10-03 follow-up resolves
[issue #9](https://github.com/terjekv/json-schema-profiler/issues/9) in the working
implementation by packaging a private maintained aggregation core directly in
`src/engine`. Its object-presence optimization is included in every archive, and
there is only one development dependency on registry `schema_analysis` 0.7.0.
`cargo package --allow-dirty --locked --offline` now verifies successfully.
`--allow-dirty` permits testing the working changes; it does not bypass verification. The
verified archive contains 81 files, including the private engine, tests, examples,
benchmark targets, installer scripts, and original license notices. A fresh
application using only the extracted archive passes accepted, rejected and
source-failed replay on stable and Rust 1.90. Its normal dependency graph contains
no `schema_analysis` dependency or repository path override.

The [engine notices](../licenses/schema_analysis/README.md) describe the derived
source, removed unused interfaces, maintenance responsibility and original licenses.
The original release remains a differential oracle; its types never enter the
public profiler API. No engine fork publication is needed.

Owned-record replay addresses the consumer integration in
[issue #7](https://github.com/terjekv/json-schema-profiler/issues/7), with a reader
example and matched parse/replay benchmarks. Benchmark setup for
[issue #11](https://github.com/terjekv/json-schema-profiler/issues/11) now uses the
supported v1.3.0 composite action after repository-controlled bounded installation.
Offline tests cover recovery, reuse, exhausted retries and timeout failure.
The updated six-target hosted workflow still needs its own run and review before
publication; local verification does not establish hosted performance acceptance.

## Historical audit results (2026-10-01)

The following records the previous path-dependency packaging failure and the
original audit evidence. Its package/engine blockers are resolved by the bundled
core described above; recorded benchmark runs apply to their historical revisions.

| Area | Result |
| --- | --- |
| Public API boundary | Crate-owned paths, options, reports and errors; upstream inference and validator types remain private; JSON values and Serde serialization are intentional integrations |
| Validated facts | Completed profiles, candidates and verified replays have private fields; a compile-fail doctest prevents fabricating `VerifiedCorpus` |
| Generated documentation | Rustdoc builds with warnings denied on stable and Rust 1.90; README links use repository URLs, including the license link that previously failed as an unresolved Rust item |
| Completion and retention | Occurrences, distinct-document evidence, diagnostic truncation and incomplete source/limit stops remain separate; reports omit instance scalars; schema literals, paths and caller IDs can be retained |
| Application trial | Separate synthetic application covers policy review, positive/negative replay, source failures, bounded evidence, dataset identity and stale revision rejection |
| Hosted comparison | PR #2's updated head `39c785a` has all 122 timing and 42 instruction cases paired, with no regressions above the unchanged 15%/3% thresholds |
| Archive consumer | A separate application built from the extracted archive passes accepted, rejected and source-failed replay on stable and Rust 1.90 |
| Package verification | **Blocked:** normalization makes the runtime engine and differential oracle duplicate dependencies with different names |
| Engine equivalence | **Blocked:** the archive resolves registry `schema_analysis` 0.7.0, losing the local object-presence fix |
| Package contents | Sources, examples, tests, benchmark entrypoints/helpers, documentation, root MIT license and upstream patch notices are included; separate nested packages are excluded |
| Dependency licenses | All 180 distinct dependency name/version entries match locked metadata; no missing license declarations; local/registry engine copies share the same dual license |
| MSRV | Rust 1.90.0 remains aligned with the manifest, CI and dependency floor; `ordered-float` and development dependency `bincode-next` declare 1.90 |
| Yanked lock entry | Both lockfiles updated from yanked `yoke-derive` 0.8.3 to non-yanked 0.8.4, with unchanged Unicode-3.0 license and a lower 1.82 declared MSRV |
| Names and metadata | Public repository and intended docs.rs metadata are consistent; both crate-name spellings remain absent from the sparse index; no name is reserved |
| Compatibility | Migration notes cover fallible suggestions, candidate provenance/evidence serialization, and the replay `input_error` stop reason |

The historical audit did not prepare a release, publish a fork, create a tag or
remove `publish = false`. Issue #9 tracked the package/engine decision resolved above. The
[owned-record replay evaluation](https://github.com/terjekv/json-schema-profiler/issues/7)
is a consumer ergonomics follow-up, not a publication prerequisite.

The [PR #2 benchmark run](https://github.com/terjekv/json-schema-profiler/actions/runs/36910579074)
completed after retrying two Valgrind installation timeouts. Those attempts failed
before measurement; their original logs remain in the run history. No regression
threshold or code change was used to make that retry pass. Each subsequent PR
must still pass its own stable/MSRV and benchmark checks.

## Archive verification history and reproduction

The 2026-10-01 audited archive contained 65 files. Cargo includes the root library, both
examples, all five integration test targets, six benchmark entrypoints and their
helpers, documentation, repository automation files, the patch and its notices.
No build output or production fixtures are included. Cargo automatically excludes
the nested `vendor/schema_analysis` and `consumers/inventory-trial` packages.
The consumer trial is therefore linked to its repository sources from the
packaged documentation. Copies of the upstream license texts under `licenses/`
accompany the patch even though the nested vendor package is excluded.

Before bundling the core, `cargo package --locked --offline` assembled the archive but failed:

```text
error: failed to verify package tarball
the crate json-schema-profiler depends on crate schema_analysis v0.7.0 multiple times with different names
```

The normalized `Cargo.toml` contains `schema_analysis = "=0.7.0"` without the
local path, alongside the `schema_analysis_original` development alias for that
same registry package. A normal external consumer does not use this package's
development dependencies, so it can build; this does not make package verification
successful or preserve the patched engine. The external graph was checked to
resolve registry 0.7.0, and it contains no vendored path or differential oracle.

The [package workflow example](../examples/package_workflow.rs) uses only public
APIs and normal dependencies. Its three focused tests run under
`cargo test --all-targets`. The same source was copied into a fresh application
depending on the extracted archive, with no registry patches, vendor files or
repository paths added. Stable and Rust 1.90 both printed:

```text
Package workflow passed: accepted replay, rejected data, preserved source failure.
```

To reproduce the archive probe from a clean checkout:

```sh
cargo package --list --locked
cargo package --locked
audit_dir=$(mktemp -d)
tar -xzf target/package/json-schema-profiler-0.0.1.crate -C "$audit_dir"
mkdir -p "$audit_dir/consumer/src"
cp "$audit_dir/json-schema-profiler-0.0.1/examples/package_workflow.rs" "$audit_dir/consumer/src/main.rs"
cat > "$audit_dir/consumer/Cargo.toml" <<'TOML'
[package]
name = "packaged-profiler-audit"
version = "0.0.0"
edition = "2024"
rust-version = "1.90"
publish = false
[dependencies]
json-schema-profiler = { path = "../json-schema-profiler-0.0.1" }
serde_json = { version = "1", features = ["arbitrary_precision"] }
[workspace]
TOML
cargo run --manifest-path "$audit_dir/consumer/Cargo.toml"
cargo +1.90.0 run --manifest-path "$audit_dir/consumer/Cargo.toml" --locked
cargo tree --manifest-path "$audit_dir/consumer/Cargo.toml" --locked
```

Require package verification to pass. Inspect the extracted `src/engine` files
and confirm the normal dependency graph has no runtime `schema_analysis` package.
Recheck the external consumer on stable and the MSRV for future engine updates.

## Rust and dependencies

The MSRV is Rust 1.90.0, matching the current dependency floor (`ordered-float`
and the Gungraun development dependency tree require 1.90). CI tests behavior and
documentation there and runs all checks/bench targets on stable. `Cargo.lock`
is kept for reproducible development and CI.

The private engine is derived from `schema_analysis` 0.7.0; `jsonschema` 0.49.9
remains pinned. Updates require numeric, reference, evidence, generated-schema
and differential regressions plus benchmark review. Upstream implementation types
are private; `serde_json::Value` is the intentional integration boundary.
The normal dependency graph now needs `ordermap` directly, while the published
engine and its sampling dependencies remain development-only.

The [license inventory](dependency-licenses.md) records locked dependency metadata,
including development tooling. Retain required notices when distributing those
dependencies. This library uses MIT.

## Naming and publication

On 2026-10-01 the public Cargo sparse-index entries for `json-schema-profiler`
and `json_schema_profiler` both returned HTTP 404 (`NoSuchKey`); the intended
docs.rs page also returned 404. Neither spelling appeared indexed;
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
Field reports also include document evidence. `EvaluationStop` now includes
`InputError { document_index }`; update exhaustive Rust matches and serialized
report readers for the `input_error` reason. Existing `evaluate`/`verify` signatures
remain available; fallible sources should use `try_evaluate`/`try_verify`.

For the first published 0.0.1, patch updates should preserve documented behavior
and report meaning. Incompatible counting, policy or representation changes need
an explicit versioned migration even while pre-1.0. Counts/schema output are
deterministic; bounded first-seen witnesses depend on input order and limits.

The discovery/tuning additions introduce `Presence::AtLeast`, `Finding::LowEvidence`
and `Finding::RequiredFromFrequency`. Update exhaustive matches and report readers.
Candidate options also retain `minimum_documents`, defaulting to one. Frequency
policies explicitly may reject the observed corpus; a candidate is still unverified.
Owned replay methods are additive and share the borrowed methods' semantics.
