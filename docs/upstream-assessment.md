# Upstream assessment

Reviewed 2026-09-24 through public GitHub metadata, source checkouts, and small
local Rust probes. The recommendation is to investigate reuse and targeted
contributions to `schema_analysis`, while treating `genson-rs` primarily as prior
art. Neither currently supplies the whole proposed product.

## Activity and usefulness

| Project | Reviewed revision | Maintenance evidence | Assessment |
| --- | --- | --- | --- |
| `schema_analysis` 0.7.0 | `0648d4dacd899029942f9c28ab4e9b2e8d94c51a` | Latest default-branch commit 2026-04-11; 0.7.0 release preparation merged 2026-04-10; repository not archived | Recently maintained, though the observed work is concentrated in a small project; worth discussing focused extensions |
| `genson-rs` 0.2.0 | `c330bb6ee1cc1941bea9808b11da53a538d684c4` | Latest default-branch commit and push 2024-05-26; an August 2025 PR remains open; repository not archived | Useful schema-generation reference, but no recent code maintenance observed; avoid making delivery depend on upstream response |

Sources: [schema_analysis commit history](https://github.com/QuartzLibrary/schema_analysis/commits/master/),
[0.7.0 preparation](https://github.com/QuartzLibrary/schema_analysis/pull/28),
[genson-rs commit history](https://github.com/junyu-w/genson-rs/commits/master/), and
[open ordering PR](https://github.com/junyu-w/genson-rs/pull/4).

These dates describe observed activity, not guarantees about maintainer
availability. Recent `schema_analysis` merges inspected were maintainer-authored;
they do not establish turnaround times for external contributions. GitHub's
general repository `updated_at` timestamp was not used as a code-activity proxy.

## How much already exists?

| Need | schema_analysis | genson-rs |
| --- | --- | --- |
| Merge shapes across documents | Present, with public structural data and `Coalesce` | Present, focused on `SchemaBuilder` and emitted schemas |
| Objects, arrays, mixed types | Present | Present in the builder; bulk-input helper has limitations |
| Type occurrence counts | Available through default/custom contexts | No corresponding profiling counters found in the reviewed strategies |
| Exact missing and null frequencies | Default field status has booleans; needs custom aggregation or field extensions | Presence represented mainly as the required-key intersection; needs new observation state |
| Deterministic structural output | Explicit field/variant sorting APIs | Some sorting exists; alternative ordering needs a separate audit |
| Draft 2020-12 generation | Explicit Schemars integration | Configurable schema URI, which alone is not a dialect implementation guarantee |
| Strict/expansive policy and explanations | New policy/rendering layer | New policy layer, plus the observations needed to explain it |
| Selected subtrees and ancestor semantics | New wrapper/selection layer | New wrapper/selection layer |
| Per-document evidence and coverage | New layer | New layer |
| Configurable whole-profile resource budgets | No suitable fallible traversal budget API found | No suitable budget API found |
| Exact arbitrary-precision JSON numbers | Compatibility gap reproduced | Current numeric strategy distinguishes parser numeric representations; exact-number support needs investigation |
| Supplied-schema evaluation | Separate validator needed | Separate validator needed |

This is a source-based scope comparison, not a percentage-complete estimate.
`schema_analysis` already implements much of the recursive structural machinery.
Most of the intended user-facing workflow still belongs in `json-schema-profiler`.
For `genson-rs`, adding the evidence model would change the underlying engine as
well as add the surrounding workflow.

## schema_analysis: promising extension points and real gaps

The useful extension is `Context`: callers choose the aggregators associated
with each kind. A custom context can collect counts without retaining scalar
samples, and object aggregators receive the encountered key list. The built-in
context records samples, so it should not be our default. See the reviewed
[context interface](https://github.com/QuartzLibrary/schema_analysis/blob/0648d4dacd899029942f9c28ab4e9b2e8d94c51a/schema_analysis/src/context/mod.rs).

There are several distinct categories of work:

1. **Can live in our crate:** presets, candidate assessments, path queries,
   report types, policy-aware schema rendering, caller IDs, and corpus validation.
   These are our product behavior, not features upstream must adopt.
2. **Potentially implementable through existing extensions:** counters-only
   contexts and per-object property-occurrence counts. For ordinary JSON objects,
   missing counts can be derived from applicable object count minus property
   count; null counts can be derived from property count minus non-null counts.
   Arrays need their own element denominators. Verify these derivations and merge
   behavior in a focused adapter experiment before proposing upstream changes.
3. **Worth proposing upstream:** field-level observation callbacks or counts,
   fallible traversal/budget hooks, and exact JSON number integration. These could
   benefit other consumers and reduce duplicated traversal in our crate.

The default `FieldStatus` remembers whether missing/null occurred, not how often.
Nested nulls bypass the context's null aggregator and set a flag directly. Thus
changing only `Context::Null` does not count nested nulls. Sources:
[field status](https://github.com/QuartzLibrary/schema_analysis/blob/0648d4dacd899029942f9c28ab4e9b2e8d94c51a/schema_analysis/src/schema.rs#L91),
[null visitation](https://github.com/QuartzLibrary/schema_analysis/blob/0648d4dacd899029942f9c28ab4e9b2e8d94c51a/schema_analysis/src/analysis/field.rs#L132).

Similarly, `Aggregate` is infallible and has no path or document-ID argument.
Budgets and evidence cannot simply be bolted onto it with an ordinary `Result`.
A wrapper can select input subtrees or preflight documents, but enforcing a
global allocation budget inside traversal is a larger integration question. See
[Aggregate](https://github.com/QuartzLibrary/schema_analysis/blob/0648d4dacd899029942f9c28ab4e9b2e8d94c51a/schema_analysis/src/traits.rs#L46).

### Numeric compatibility probe

The inspected context interface receives `i128` and `f64`, not exact JSON decimal
tokens. A local executable tested both ordinary `serde_json` and its
`arbitrary_precision` feature, through direct deserialization into `InferredSchema`
and through an already-parsed borrowed `serde_json::Value`.

| Input/path | Observed result |
| --- | --- |
| `{"x":1.0}`, ordinary parsing | Classified as float; schema permits `number` |
| `{"x":1.0}`, arbitrary precision, direct JSON deserialization | Classified as an object with a `$serde_json::private::Number` property |
| `{"x":1.0}`, arbitrary precision, borrowed `Value` | Classified as float; schema permits `number` |
| A 50-digit integer, ordinary parsing | Parsed/aggregated through floating point; schema permits `number` |
| A 50-digit integer, arbitrary precision, either input path | Classified as an object with a `$serde_json::private::Number` property |

The float classification is broader than our intended mathematical-integer
classification; it is not itself a rejection of those inputs. Classifying a
number as an object is a real compatibility problem: the generated type cannot
accept the original number. This matters for consumers enabling arbitrary
precision, including the current Hubuum workspace.

A targeted upstream improvement should address exact-number integration or
clearly reject unsupported representations. Disabling arbitrary precision would
trade this failure for possible precision loss and would not satisfy our proposed
contract. Cargo feature unification also makes that an unreliable consumer fix.

### Other observed semantics

For `[{"x":null},{"x":null},{},{"x":1}]`, the default profile records four object
occurrences, one integer occurrence, and missing/null flags. It does not expose
the exact two-null/one-missing split in that profile.

For `{"x":null}`, the current Schemars exporter emits `"x": true`, retaining the
required property but allowing any value. That is a broad choice for insufficient
non-null evidence, not the null-only policy proposed here. Our renderer can make
this choice explicit. The source is the
[field exporter](https://github.com/QuartzLibrary/schema_analysis/blob/0648d4dacd899029942f9c28ab4e9b2e8d94c51a/schema_analysis/src/targets/schemars.rs#L151).

## genson-rs: more adaptation than our needs justify

`SchemaBuilder` accepts `simd_json::BorrowedValue`; its internal observation tree
is not exposed as a queryable profile. Object strategies retain properties and
the intersection of required keys, and numeric strategies retain a resulting
type. They do not retain the frequencies or document evidence our reports need.
See [builder](https://github.com/junyu-w/genson-rs/blob/c330bb6ee1cc1941bea9808b11da53a538d684c4/src/builder.rs),
[object strategy](https://github.com/junyu-w/genson-rs/blob/c330bb6ee1cc1941bea9808b11da53a538d684c4/src/strategy/object.rs),
and [numeric strategy](https://github.com/junyu-w/genson-rs/blob/c330bb6ee1cc1941bea9808b11da53a538d684c4/src/strategy/scalar.rs).

There are also embedding concerns. The library declares a global MiMalloc
allocator, exposes parsing helpers that unwrap errors, and incorporates Rayon
parallelism. Making it a controlled component in another library would require
reviewing those choices. See the
[library entrypoint](https://github.com/junyu-w/genson-rs/blob/c330bb6ee1cc1941bea9808b11da53a538d684c4/src/lib.rs).

A local probe of `build_json_schema` with `delimiter: None` and
`ignore_outer_array: false` observed:

| Input | Result |
| --- | --- |
| `{"x":1}` | Expected object schema |
| Empty buffer | Panic |
| `{"x":"}"}` | Panic despite valid JSON |
| `["a","b"]` | `{"items":{},"type":"array"}`; element type information lost |

These findings apply to that bulk-input helper. They do not establish that the
lower-level builder fails on the same values; feeding already-parsed values can
bypass the scanner. The open
[string-array issue](https://github.com/junyu-w/genson-rs/issues/2) reports the same
area of functionality.

Fixing these helper bugs and moving allocator ownership to the CLI would be
reasonable independent contributions if there is upstream interest. Building
the entire profiler into this project would be a substantial change in scope,
with less evidence of current maintenance than the alternative.

## Recommended next decision (initial assessment)

Keep `json-schema-profiler` as the independent public API and policy/reporting
layer. Use `jsonschema` for validation. Give `schema_analysis` one focused
integration experiment before selecting its observation engine:

1. Build a custom context collecting counts without scalar samples.
2. Verify missing/null derivation, nested arrays, document-level evidence, and
   deterministic results across reordered input.
3. Resolve exact-number handling and prove budget enforcement can happen before
   uncontrolled profile growth.
4. Compare adapter complexity against a small purpose-built `Value` traversal.

If the adapter preserves our invariants without a second full observation engine,
reuse it and propose focused generic improvements upstream. If most ingestion
must be replaced, use our own accumulator and still contribute independently
useful correctness fixes. Do not make v0.0.1 depend on an unaccepted large upstream
redesign.

This assessment includes source review and small probes only, not a complete
upstream test-suite run, performance comparison, or finished adapter. No issues,
pull requests, or maintainer messages were posted as part of the research.

The subsequent [integration experiment](experiment.md) implements the proposed
counts, selection, numeric and admission adapters. Its [benchmark report](benchmarks.md)
records the measured costs and updates the recommendation. Public replay validation
and document-level evidence were added in the subsequent v0.0.1 implementation.
