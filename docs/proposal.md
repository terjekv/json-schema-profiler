# Feasibility and v0.0.1 proposal

Drafted 2026-09-24. This describes the intended release target, not a promise that
every API is implemented. The [integration experiment](experiment.md) now implements
the core profiling and replay workflow. The README and support matrix document
the implemented subset and explicit restrictions.

## Recommendation

Build a generic Rust library that profiles a corpus, explains structural
variation, generates policy-controlled candidates, and evaluates schemas against
the supplied documents. This is feasible without machine learning or a new
implementation of JSON Schema validation.

The useful product is the explanation: which paths prevent a uniform contract,
what broadening would cover the corpus, and which documents a proposed constraint
rejects. A plain schema generator omits much of that information.

There is always a schema accepting every input: the boolean schema `true`.
Consequently, viability must mean usefulness under an explicit policy, rather
than mere existence. The specification defines these boolean schemas in
[JSON Schema Core](https://json-schema.org/draft/2020-12/json-schema-core#section-4.3.2).

Even a complete snapshot cannot establish the intended future contract. For
example, observing only `"active"` does not prove that no other status is valid.
Likewise, enumerating complete input documents would memorize the corpus without
discovering a useful model. Avoid both extremes.

## Proposed workflow and boundaries

```text
Caller-owned documents
        |
        v
Bounded profiler --> Queryable observations
                            |
                   Inference policy + scope
                            |
                            v
                    Schema candidate + reasons
                            |
                  Replay documents through validator
                            |
                            v
                 Coverage + bounded diagnostics
```

Keep three operations distinct:

1. **Profile:** discover paths, presence, types, and array structure.
2. **Suggest:** synthesize a schema using observations and chosen policies.
3. **Evaluate:** validate documents against an inferred or caller-supplied schema.

Querying a profile should support questions such as “which fields have mixed
types?” and “is this property always present when its parent is an object?”.
Evaluating a schema should support “which documents fail this `required` or
`type` constraint?”. A general query language is unnecessary for v0.0.1.

Profile summaries cannot answer arbitrary schema queries exactly. Cross-field
conditions, patterns, uniqueness, and many other assertions require original
values. Keep documents with the caller and request a replay for exact validation;
when a schema is supplied up front, evaluation can run alongside profiling.

## Define strictness as separate policies

“Strict” mixes several independent choices. Offer named presets backed by
explicit options, with the selected settings included in every report.

| Policy | Strict preset | Expansive preset |
| --- | --- | --- |
| Unrelated observed types | Return a conflict instead of choosing one | Preserve observed alternatives in a union |
| Integer and fractional numbers | Widen to `number` | Widen to `number` |
| Observed null | Permit explicit nullability and flag it | Permit explicit nullability and flag it |
| Property presence | Require properties present in every applicable parent | Leave inferred properties optional |
| Extra properties inside selected object subtrees | Disallow | Allow |
| Fields outside selected subtrees | Allow | Allow |
| Enums, bounds, patterns, formats | Do not infer constraints | Do not infer constraints |

Expose overrides for extra properties, observed versus optional presence, mixed
types, and nullable versus non-null values. Requiring non-null values when nulls
were observed is a policy conflict, not permission to discard observations.
Integer widening and nullability must be reported even when the preset permits
them. “Strict” therefore means a consistent structural contract under these
settings, not the mathematically narrowest possible schema.

For `{ "size": 4 }` and `{ "size": "4" }`, strict inference reports a conflict
at `size`. Expansive inference can produce `"type": ["integer", "string"]`.
Neither operation coerces data. Numeric strings remain strings.

Both presets promise to cover all successfully profiled documents if they
return a corpus-covering candidate. If the selected policy cannot do so, return
the reasons. An intentionally narrower schema supplied by the caller can still
be evaluated and its failures counted. Do not implement automatic majority-type
selection or data cleanup in this release.

JSON Schema supports type unions, and `integer` includes mathematically integral
values regardless of their JSON spelling. These are specification semantics,
not Rust numeric representation choices. See the
[validation specification](https://json-schema.org/draft/2020-12/json-schema-validation#section-6.1.1).

## Selected subtrees

Constrain a selected subtree with nested `properties` and leave surrounding
objects open. For example, selecting `/hardware` with a closed-object policy
could produce the following candidate:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "hardware": {
      "type": "object",
      "properties": {
        "cores": { "type": "integer" },
        "vendor": { "type": "string" }
      },
      "required": ["cores"],
      "additionalProperties": false
    }
  },
  "additionalProperties": true
}
```

Here `hardware` is optional at the root. When present, it must be an object with
`cores`; `vendor` is optional. Unknown fields inside it fail, while unrelated
root fields can contain any JSON value. Selecting only `/hardware/cores` instead
would leave other `hardware` fields open. Selecting a subtree includes all its
descendants. Adding root `required: ["hardware"]` is a separate presence choice.
These behaviors follow the documented
[object keywords](https://json-schema.org/understanding-json-schema/reference/object).

Proposed selection rules:

- Use typed segments `Property(name)` and `EachItem`, for example
  `Property("interfaces"), EachItem, Property("address")`.
- Offer an escaped object-property path convenience for `/hardware/cores`.
  Do not label wildcard selectors as JSON Pointers. Concrete instance locations
  in validation failures use actual
  [RFC 6901 JSON Pointers](https://datatracker.ietf.org/doc/html/rfc6901).
- A literal property named `*` stays distinct from `EachItem`. Numerical property
  names remain property names; fixed array-index selection is deferred.
- Scope affects profiling, not just rendering: inspect selected descendants and
  the ancestor structure needed to reach them; skip unselected sibling values.
- Ancestors must have the container types implied by the selector when present.
  A scalar `hardware` is a structural conflict when selecting `hardware/cores`.
  Do not silently count it as a missing `cores` field.
- Presence policy applies at each relevant level. `IfPresent` imposes no existence
  requirement; `Observed` requires a property only when it appeared in every
  applicable parent. No observations means insufficient evidence.
- An `EachItem` selection checks every element. Empty arrays supply no item-type
  evidence. Presence inside items does not imply a nonempty array.
- Deduplicate identical selections and collapse redundant descendants with the
  same policy. Reject overlapping selections with conflicting policies in
  v0.0.1 instead of introducing implicit precedence.

Emit one merged `properties` tree for this release. Future composition through
`allOf` will need care: `additionalProperties` only sees declarations in its own
subschema; `unevaluatedProperties` can account for evaluated properties across
subschemas. See the specification's
[applicator and unevaluated vocabularies](https://json-schema.org/draft/2020-12/json-schema-core#section-10).

## What to record and report

Build a structural tree with separate observations for each encountered kind.
Preserve the object and array portions when a node also contains scalars.
Otherwise later generation can lose constraints or create false conflicts.

Each path should expose:

- Occurrence counts by kind, using disjoint integer and non-integral-number bins.
- Present, missing, and null counts with explicit denominators.
- Counts of applicable object parents, array elements, and contributing documents.
- A bounded set of caller-provided document identifiers illustrating each issue.
- Findings such as mixed types, numeric widening, null-only evidence, no array
  items, and insufficient observations.

For `[{"a": {"b": 1}}, {}]`, `a` is present in one of two root objects;
`b` is present in one of one applicable `a` objects. Requiring `b` inside optional
`a` accepts both documents. Missing ancestors, wrong-type ancestors, absent
properties, and explicit null must remain distinguishable.

For arrays, keep element counts separate from document counts: a document with
1,000 items must not count as 1,000 documents. Deduplicate document contributions
per path while processing that document. A later-discovered property must account
for earlier applicable parents when calculating absence.

Use exact decimal semantics for number classification; never route arbitrary
precision JSON numbers through `f64` to decide whether they are integers.
Include `1`, `1.0`, `1e0`, fractional values, negative zero, and very large numbers
in compatibility tests against the chosen validator. Precision already lost by
the caller's parser cannot be recovered by this library.

Do not store scalar examples by default. Retain counts and bounded opaque caller
IDs; property paths can themselves contain sensitive information and must be
documented as retained metadata. Validator messages need structured conversion
to avoid copying offending values into reports.

## Assess viability without a misleading score

Return an assessment with independent, inspectable dimensions:

- **Evidence:** supplied document count, selected scope, zero-evidence paths,
  and whether traversal completed. A caller's claim of a complete dataset is
  metadata, not something the library can verify.
- **Policy compatibility:** satisfied, blocked by listed conflicts, or
  insufficient evidence. Empty corpora receive no positive recommendation.
- **Flexibility introduced:** union types, nullable fields, optional fields,
  open objects, and unconstrained item schemas, each tied to a path.
- **Validation:** not run, complete with passing/failing document counts, or
  incomplete. Diagnostic truncation is reported independently of scan completion.

An example report could say: “1,000 documents profiled; strict policy blocked by
two mixed-type paths. Expansive candidate covers all 1,000 documents after replay;
14 optional fields and two unions were introduced.” It must not say that future
documents have a particular probability of passing.

Do not sum per-path failure counts to estimate rejected documents: one document
may fail several constraints. Count document outcomes during validation.
Provide transparent observations rather than a universal 0–100 quality score.

## v0.0.1 delivery target

Implement one synchronous library crate with these capabilities:

1. Incremental ingestion of borrowed `serde_json::Value` documents with optional
   caller IDs, supporting any JSON root type for whole-document profiling.
2. Whole-document or typed subtree selection, including all array elements.
3. Queryable path observations and deterministic, serializable reports.
4. Strict and expansive policies with the explicit overrides described above.
5. JSON Schema Draft 2020-12 output using `$schema`, `type`, `properties`,
   `required`, `additionalProperties`, `items`, and `anyOf` where needed.
6. Candidate and supplied-schema evaluation over a caller-provided document
   iterator, returning document coverage and bounded structured diagnostics.
7. Configurable limits and explicit incomplete or failed outcomes.
8. Examples and regression tests demonstrating the complete workflow.

Treat arrays as lists with one merged item schema. Use type unions for simple
alternatives and `anyOf` for alternatives needing separate structures. Do not
infer `oneOf`: its exclusive-match semantics require more evidence. An array
observed only as empty can use `items: true`, accompanied by an explicit finding.
A selected path never reached at all produces insufficient evidence instead of
invented constraints.

Flattening object variants loses correlations: observing `{ "kind": "a",
"x": 1 }` and `{ "kind": "b", "y": 2 }` does not justify a general correlated
union automatically. The first release merges object properties and documents
that it may accept unseen combinations. This is a limitation even in strict mode.

Supplied-schema evaluation should pin Draft 2020-12, validate schema syntax, and
publish a tested keyword-support matrix for the selected `jsonschema` version.
Delegate validation keywords to that engine, including caller-supplied assertions
which this crate does not infer. Reject unsupported required vocabularies and
explicitly unsupported features; do not claim full specification coverage merely
because generated schemas use a small supported subset.

Start with self-contained schemas and fragment-only references; reject nested
resource identifiers and external reference requirements. Make format assertion
an explicit option with the supported formats documented. Prevent automatic
network or filesystem reference retrieval through both dependency settings and
an explicit denying retriever, including when another dependency unifies Cargo
features. The validator documents configuration, draft support, and retrieval in
its [Rust API guide](https://docs.rs/jsonschema/latest/jsonschema/).

Defer CLI commands, JSONPath, fixed-index tuples, automatic discriminators,
pattern discovery, enum/range inference, dictionary detection, repair execution,
schema-to-schema containment proofs, remote references, persistence, parallel
workers, and incremental deletion. Leave profile merging for a later release
unless reuse makes its exact counting semantics straightforward.

## Suggested API shape

These are conceptual signatures, not a committed public API:

```text
Profiler::new(validated_options) -> Profiler
Profiler::observe(optional_id, &Value) -> Result<(), ProfileError>
Profiler::finish(self) -> Result<Profile, ProfileError>

Profile::field(&ProfilePath) -> Option<&FieldProfile>
Profile::fields() -> iterator of field profiles
Profile::assess(&InferencePolicy) -> Assessment
Profile::suggest(&InferencePolicy) -> Suggestion

Suggestion = Candidate(SchemaCandidate) | Blocked(Findings) | Insufficient(Findings)

CompiledSchema::new(&Value, validated_options) -> Result<CompiledSchema, SchemaError>
CompiledSchema::evaluate(document_iterator, report_limits) -> Evaluation
```

`ProfilePath`, `InferencePolicy`, `Limits`, `Profile`, `SchemaCandidate`, and
`CompiledSchema` should preserve validated state behind private fields. Prefer
owned public errors and report types to exposing the validator's internals.
Using `serde_json::Value` for inputs and generated schema output is intentional.
Separate a successful scan result from bounded diagnostic details and from the
candidate's inference provenance. Do not let callers manufacture a “verified”
candidate just by setting a boolean.

The profiler need not retain documents. Memory scales with the observed structure
and bounded evidence; traversal is approximately linear in visited input size,
plus map operations. Distinct dynamic keys can still make the profile grow as
large as the input, so depth, nodes, property-name bytes, witnesses, and schema
output size need limits. Fail rather than silently dropping observations. An
ingestion error must either leave state unchanged or put the profiler in a
terminal failed state so `finish` cannot return misleading success.

Validation reporting limits do not guarantee a hard execution deadline inside a
third-party validator. Keep that limitation explicit; cancellation between
documents and process isolation by a consuming application are separate concerns.

## Existing tools and reuse choices

These are observations from the linked project documentation, followed by
proposed uses. They are not benchmarks or claims that alternatives lack every
feature we need.

| Project | Relevant documented capability | Proposed role |
| --- | --- | --- |
| [schema_analysis](https://docs.rs/schema_analysis/latest/schema_analysis/) | Rust/Serde structural analysis, distinct missing/null observations, multi-document aggregation, merging, and schema export integration | First candidate to evaluate for reusing the observation engine |
| [genson-rs](https://github.com/junyu-w/genson-rs) | Rust implementation generating schemas from one or more JSON inputs, with a focus on throughput | Evaluate inference reuse and compare output on shared fixtures |
| [GenSON](https://github.com/wolverdude/GenSON) | Python schema builder with sample/schema merging and explicit guiding rules | Learn merge rules and edge cases; avoid introducing a Python runtime dependency |
| [quicktype](https://github.com/glideapps/quicktype) | Infers types and JSON Schema from sample data; documents a review-and-edit workflow | Compare user workflow and treatment of optional fields and variants |
| [Schemars](https://graham.cool/schemars/generating/) | Generates schemas from Rust types, with a less precise example-value mode | Possible report-schema generation later; assess separately from corpus profiling |
| [jsonschema](https://docs.rs/jsonschema/latest/jsonschema/) | Rust schema validation, reusable compiled validators, schema checks, structured errors, configurable reference retrieval | Recommended validation dependency behind an adapter |
| [JSON Schema Test Suite](https://github.com/json-schema-org/JSON-Schema-Test-Suite) | Shared language-independent schema validation fixtures | Source of relevant semantics and regression cases |

Start with a bounded reuse spike. Test `schema_analysis` and `genson-rs` against
the intended statistics, precision, selection, deterministic ordering, evidence,
and resource limits. Verify dependency licenses and maintenance state before
committing to either. Schema output alone cannot recover the frequency and
provenance information needed for explanations.

The initial [upstream assessment](upstream-assessment.md) now favors a focused
`schema_analysis` adapter experiment with targeted upstream improvements.
`genson-rs` remains useful as prior art, but its current interfaces and maintenance
state make it a weaker foundation. Neither is ready for unconditional adoption.

If an adapter can preserve those facts, reuse the engine. If it would require
walking every document again and duplicating the state anyway, write a small
structural accumulator and retain those tools as comparison references. Either
way, own the policy and assessment layer and reuse an established validator.

## Acceptance criteria and implementation order

Implement in four reviewable steps:

1. Compare reuse candidates against a small synthetic fixture corpus and record
   the decision; settle numeric semantics, supported draft, and public types.
2. Implement profiling, selection, exact counts, typed limits, and failure states.
3. Implement generation policies, candidate explanations, and schema validation.
4. Implement corpus evaluation, examples, support documentation, and packaging.

Release gates:

- Every generated candidate passes Draft 2020-12 schema validation.
- Every corpus-covering candidate accepts every document used to infer it.
- Negative examples demonstrate that intended constraints actually reject data.
- Missing/null distinctions and nested required denominators are correct.
- Mixed-type, mixed-root, and nested-array corpora have defined results.
- Empty corpora, empty arrays, null-only fields, and never-matched selections are
  reported honestly.
- Selected subtrees reject local violations while unrelated values remain free.
- Literal `*`, empty property names, `/`, `~`, and numerical property names work.
- Reordering documents leaves schema and aggregate statistics unchanged; bounded
  illustrative witnesses have an explicitly documented ordering policy.
- Limits, caller iterator errors, and diagnostic truncation cannot produce a
  false all-documents-passed result.
- Numeric classification and validation agree on exact-value edge cases.
- Reference retrieval stays disabled under dependency feature unification.
- Report payloads omit document scalar values by default.
- Rust tests, doctests, formatting, clippy, and Markdown lint pass.
- Dependency licenses, MSRV, package contents, metadata, and crate-name
  availability are checked before publication.

The repository scaffold intentionally has no implementation dependencies yet.
Choosing versions belongs to the reuse spike; package publication is disabled.

## A later consuming-application integration

A consumer such as Hubuum can stream the JSON data of objects in one class,
providing opaque IDs for diagnostics. The library returns observations and a
candidate. The application owns authorization, snapshot consistency, task
lifecycle, presenting the suggestion, and applying an approved schema.

For exact two-pass coverage, replay the same dataset snapshot or attach a dataset
revision and reject stale results before applying a schema. The library must not
promise that a database remained unchanged while analysis ran.

Hubuum already has a separate `hubuum-schema-diagnostics` crate which converts
validator errors into bounded, value-redacted reports. Its treatment of evidence
limits and paths is useful prior art. Keep this library independent; evaluate
extraction or a generic shared diagnostic dependency separately if useful.
