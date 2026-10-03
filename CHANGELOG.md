# Changelog

## [Unreleased]

### Added

- Owned-record evaluation and verification, including fallible on-demand parsing,
  with the same coverage, source-error and resource-limit contracts as borrowed replay.
- Bounded structural discovery for sparse properties, rare kinds, nullability,
  mixed types, unobserved paths and low document evidence, with explicit thresholds.
- Balanced inference preset, exact required-property frequency thresholds and
  minimum contributing-document requirements. Frequency-based requirements expose
  their counts and can reject observed documents; replay remains required to prove coverage.
- Offline regression tests for benchmark-tool installation retries and a complete
  JSON Lines replay example; parsing/replay and discovery benchmarks.

- Release-readiness audit and a packaged-consumer workflow example covering
  accepted, rejected, and source-failed replay, with compiling API/verification
  boundary documentation and explicit publication blockers.
- Generated API documentation checks on stable and the MSRV, with README links
  that also work when rendered outside the repository checkout.
- Standalone synthetic inventory consumer and CI coverage demonstrating snapshot
  identity/revisions, reviewed policies, fallible replay, bounded diagnostics and
  rejection of stale schema application. The library API is unchanged.
- Fallible replay evaluation and verification through `try_evaluate` and
  `try_verify`, preserving caller errors separately from bounded reports and
  preventing source failures from producing verified corpus evidence.
- Wide/sparse object scaling benchmarks and differential aggregation regressions
  against the published `schema_analysis` 0.7.0 release.
- Experimental incremental corpus profiling backed by `schema_analysis` 0.7.0,
  with exact missing/null/type occurrence counts and arbitrary-precision number
  classification.
- Typed subtree selection, deterministic path queries and serializable reports,
  configurable admission limits, and explicit failed/incomplete outcomes.
- Strict and expansive Draft 2020-12 structural schema candidates with findings
  and independent mixed-type, presence, and additional-property policy controls.
- Synthetic correctness fixtures, independent schema validation tests, Criterion
  timing benchmarks, and Gungraun instruction/allocation benchmarks.
- PR benchmark integration pinned to `terjekv/rust-pr-bench` v1.3.0, plus CI,
  reproduction commands, recorded experiment results, and an upstream reuse
  assessment.

- Public candidate/supplied-schema replay evaluation with exact numeric assertions,
  explicit format policy, bounded redacted diagnostics and verified replay evidence.
- Per-path/per-kind distinct-document counts, validated optional document IDs and
  bounded witness retention independent of occurrence counting.
- Null rejection, nonoverlapping subtree policy overrides, compact-JSON output
  limits and explicit incomplete evaluation outcomes for resource limits.
- Tested keyword/format support, restricted local references, an explicit denying
  retriever, resource audit and dependency license inventory.
- Validation/compilation/evidence benchmarks and Rust 1.90 MSRV CI, including a
  dependency-feature-unification test for disabled file retrieval.

### Changed

- Bundle the maintained aggregation core inside the library instead of depending
  on an unpublished nested package. Cargo archives retain the object-presence fix;
  the original registry release remains a development-only differential oracle.
- Use the pinned benchmark composite action after a bounded, retrying Valgrind
  installation. Preserve setup attempts and retain 3% instruction/15% timing gates.
- Raw upstream benchmark baselines now explicitly use the published 0.7.0 release
  and have new `published_` identities. Previous raw cases used the local patch;
  treat these as different baselines, not comparable performance regressions.
- **Breaking:** `Presence` adds `AtLeast(Frequency)`; `Finding` adds `LowEvidence`
  and `RequiredFromFrequency`. Update exhaustive matches and serialized-report readers.
  Candidate `options` adds `minimum_documents` (default 1). Existing presets and
  borrowed replay signatures retain their behavior.

- Refresh both development lockfiles from yanked `yoke-derive` 0.8.3 to 0.8.4;
  Rust 1.90 remains the supported minimum. Public APIs are unchanged.
- **Breaking:** `EvaluationStop` now includes `InputError { document_index }`.
  Update exhaustive matches and serialized-report consumers to handle the
  `input_error` reason. Existing `evaluate`/`verify` signatures and infallible
  report output are unchanged; use the fallible methods for sources that can fail.
- Replace repeated linear object-key scans in a local `schema_analysis` 0.7.0
  copy with field-indexed presence flags. Small objects require no new allocations;
  wider objects use temporary storage proportional to their fields. Public APIs and report
  semantics are unchanged. Adopt a released upstream fix or an explicit packaging
  alternative before publishing; the local dependency remains unpublished.
- **Breaking:** `Profile::suggest` now returns `Result<Suggestion, InferenceError>`;
  callers must handle output-budget errors before examining a suggestion. Use
  `suggest_with` for per-path policies and custom output limits.
- **Breaking:** serialized candidate provenance now stores `options` (global
  policy, overrides and output limits) instead of a single `policy` field;
  profile field reports now include document evidence. Update report consumers.

No version has been released; `publish = false` remains set.
