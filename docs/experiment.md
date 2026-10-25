# schema_analysis integration experiment

Performed 2026-09-24 using `schema_analysis` 0.7.0. This describes implemented
behavior, including the subsequent v0.0.1 additions. The [proposal](proposal.md)
records the original target; the README documents the current API.

## Decision

Keep a separate `json-schema-profiler` library. The current implementation
bundles the required `schema_analysis` aggregation core as a private module to
preserve the optimization in distributable Cargo archives. The extension API is useful:
it handles incremental structural merging, object variants, and recursive arrays
without requiring us to maintain a second aggregation engine. The initial
prototype needed no fork or upstream patch; the subsequent
[wide-object optimization](wide-objects.md) uses a focused patch now retained in
the bundled core.

The separate library is warranted when callers need explanations, counts,
selected subtrees and policy-controlled schemas. For callers who only want a
broad schema from ordinary JSON, direct `schema_analysis` remains simpler.

This is a substantial adapter and policy layer, not merely a renamed upstream
API. Benchmarks compare upstream contexts against the complete profiler; they
do not establish that this architecture beats a purpose-built accumulator.
Keep the engine private so that decision can change without a consumer rewrite.

## What was reused and what we added

| Concern | Implementation |
| --- | --- |
| Recursive shape inference and incremental merging | Reused `schema_analysis::InferredSchema` |
| Exact scalar/object/array occurrence counts | Custom upstream `Context` and aggregators |
| Exact missing and null counts | Derive from object key occurrences, array item totals, and non-null counts |
| Root null handling | A virtual one-element document array routes roots through upstream `Field` |
| Exact JSON number kinds | Inspect original decimal token and send kind markers through a custom Serde adapter |
| Input selection and limits | Adapter skips unselected values and admits paths before upstream growth |
| Queries and deterministic reports | Crate-owned path/type/count API and sorted profiles |
| Strictness and compromise explanations | Crate-owned policies, findings, and schema renderer |
| Candidate/supplied-schema replay validation | Internal `jsonschema` 0.49.9 adapter with explicit support and resource policies |
| Document-level evidence | Admission index tracks per-document visitation and bounded witness labels |

The context records object key frequencies because upstream field status stores
only missing/null flags. A property's null count is its presence count minus its
non-null type counts. Array item denominators count all elements; object-property
denominators count only applicable object occurrences. Tests cover later-discovered
properties, absent ancestors, mixed kinds, repeated nulls and reordered inputs.

Only integer/fractional kind markers reach upstream numeric aggregators, and
string/boolean markers carry no original scalar content. This is valid for this
counters-only context. Adding ranges, samples or enum discovery would require a
different adapter contract; the marker values must never be used as observations.

The path admission index stores child IDs and performs full-path allocation only
when a path is first observed. It now also tracks distinct document contributions
and witnesses without duplicating the recursive shape/occurrence engine.
Selection and resource checks occur during the same traversal as inference.
There is no separate full-document preflight pass.

## Findings that warrant upstream work

1. **Root null merging:** the reviewed generic `Schema::coalesce` implementation
   lacks a `(Null, Null)` merge arm. Repeated directly merged root nulls can create
   duplicate union alternatives. The wrapper avoids this with a virtual envelope;
   a focused fix and regression test would benefit upstream directly.
2. **Exact JSON numbers:** upstream accepts `i128`/`f64`, while Serde JSON arbitrary
   precision can surface a private number map. A normal inference path can then
   infer an object for a JSON number. Our shape adapter avoids that, but exact
   numeric aggregation deserves an upstream design discussion.
3. **Wide objects:** the object visitor repeatedly tests membership in a `Vec`
   of keys for previously known fields. Inspection suggests quadratic work with
   width; the wide-object benchmarks show a steep cost in both raw upstream and
   the wrapper. The [wide-object investigation](wide-objects.md) confirms this
   cause and records the local patch and upstream adoption plan.
4. **Observation and budget hooks:** field-level counts and fallible traversal
   hooks would simplify our integration. These are broader API changes, less
   urgent than correctness fixes and measured hot paths.

Source reviewed: [schema merging](https://github.com/QuartzLibrary/schema_analysis/blob/0648d4dacd899029942f9c28ab4e9b2e8d94c51a/schema_analysis/src/schema.rs),
[analysis implementation](https://github.com/QuartzLibrary/schema_analysis/tree/0648d4dacd899029942f9c28ab4e9b2e8d94c51a/schema_analysis/src/analysis),
and [context interface](https://github.com/QuartzLibrary/schema_analysis/blob/0648d4dacd899029942f9c28ab4e9b2e8d94c51a/schema_analysis/src/context/mod.rs).
The [earlier assessment](upstream-assessment.md) has reproduction results and
maintenance evidence. No upstream issues, PRs, or messages have been sent.

## v0.0.1 follow-through

| Capability | Current implementation |
| --- | --- |
| Public replay validation | Compiled schema, exact coverage, bounded diagnostics, verified replay evidence |
| Caller IDs and witnesses | Validated labels, bounded per-kind evidence and labeled violations |
| Distinct document counts | Per-path and per-kind contributions deduplicated within each input record |
| Null and per-path policies | Null rejection plus nonoverlapping subtree overrides |
| Output/diagnostic budgets | Compact-JSON limits, finding/diagnostic caps and documented resource boundaries |
| Supported vocabulary | Tested assertion/format matrix, reference restrictions and denying retriever |
| Release readiness | Rust 1.90 MSRV, dependency license inventory and compatibility policy; publishing disabled |

The profile alone cannot evaluate `pattern`, `enum`, range, uniqueness or
conditional constraints because it discarded those values. The new replay API
performs those checks. `VerifiedCorpus` describes only a nonempty, fully passing
supplied replay, including its effective format policy.

Profiling input is incremental borrowed `serde_json::Value`, not a streaming JSON parser.
Replay also accepts owned records parsed on demand by the consumer.
Parsing memory belongs to the caller, and selection still examines object keys
to find the selected ones. We do not retain document scalar values, discover
cross-field correlations, merge independent profiles, remove documents, or infer
enums, ranges, formats, dictionaries or tuple schemas.

## Measured costs

See the [benchmark report](benchmarks.md) for timing, instruction and allocation
results, full case data, baseline definitions and reproduction commands. Those
results quantify the extra work, not an apples-to-apples implementation race:
raw upstream does not offer the wrapper's limits, selections, exact-number
semantics or canonical queryable report.

The engine remains reusable after adding document witnesses: they fit in the
existing admission index. Keep the upstream correctness/performance proposals
focused; broader reference support or streaming parsing can be separate future
work. The repository is public; the crate remains unreleased with publication
disabled. The [consumer trial](consumer-trial.md) exercises application ownership
and replay, while the [release audit](release.md) records packaging resolution and publication gates.
