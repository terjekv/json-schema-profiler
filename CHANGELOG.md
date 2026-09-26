# Changelog

## [Unreleased]

### Added

- Fallible replay evaluation and verification through `try_evaluate` and
  `try_verify`, preserving caller errors separately from bounded reports and
  preventing source failures from producing verified corpus evidence.
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

- **Breaking:** `EvaluationStop` now includes `InputError { document_index }`.
  Update exhaustive matches and serialized-report consumers to handle the
  `input_error` reason. Existing `evaluate`/`verify` signatures and infallible
  report output are unchanged; use the fallible methods for sources that can fail.
- **Breaking:** `Profile::suggest` now returns `Result<Suggestion, InferenceError>`;
  callers must handle output-budget errors before examining a suggestion. Use
  `suggest_with` for per-path policies and custom output limits.
- **Breaking:** serialized candidate provenance now stores `options` (global
  policy, overrides and output limits) instead of a single `policy` field;
  profile field reports now include document evidence. Update report consumers.

No version has been released; `publish = false` remains set.
