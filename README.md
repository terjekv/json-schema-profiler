# json-schema-profiler

An experimental Rust library that profiles JSON document corpora and proposes
JSON Schemas with explicit policies. Its private structural aggregation core is
derived from `schema_analysis` 0.7.0, with a counters-only context and a JSON input adapter.
The bundled core includes an [object-presence optimization](https://github.com/terjekv/json-schema-profiler/blob/main/docs/wide-objects.md)
and ships inside the Cargo archive. Attribution and maintained-source provenance
are recorded in the [engine notices](https://github.com/terjekv/json-schema-profiler/blob/main/licenses/schema_analysis/README.md).

**Status: v0.0.1 implementation, unreleased.** Profiling, document evidence,
subtree selection, per-path policies, candidate generation and bounded replay
validation are implemented. Rust 1.90 or newer is required.
Structural discovery, graduated inference policies and owned-record replay are available.
The crate is generic and has no database, web framework, or runtime dependency.

## Use

```rust
use json_schema_profiler::{InferencePolicy, JsonKind, ProfilePath, Profiler, Suggestion};
use serde_json::json;

let mut profiler = Profiler::default();
profiler.observe(&json!({"size": 4}))?;
profiler.observe(&json!({"size": "4"}))?;
profiler.observe(&json!({}))?;
let profile = profiler.finish()?;

let size = profile.field(&ProfilePath::root().property("size")).unwrap();
assert_eq!(size.present(), 2);
assert_eq!(size.missing(), 1);
assert_eq!(size.types().get(JsonKind::Integer), 1);
assert!(matches!(profile.suggest(InferencePolicy::strict())?, Suggestion::Blocked(_)));

if let Suggestion::Candidate(candidate) = profile.suggest(InferencePolicy::expansive())? {
    println!("{}", candidate.schema());
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

Run `cargo run --example explore` for a report and both policy outcomes.
Run `cargo run --example package_workflow` for successful, rejected, and
source-failed replay through the public API.
The [standalone inventory trial](https://github.com/terjekv/json-schema-profiler/blob/main/docs/consumer-trial.md) exercises the complete
workflow with snapshot revisions, fallible replay, and deliberate failures.

Strict mode rejects unrelated mixed types, requires properties present in every
applicable object, and closes selected object subtrees. Expansive mode permits
unions, makes properties optional, and permits additional properties. The three
axes have independent overrides. Both presets preserve observed nulls and widen
mixed integer/fractional observations to `number`. `Nullability::Reject` adds an
explicit conflict when nulls were observed.

Numbers use exact decimal classification: `1.0` is integral; an arbitrarily long
fraction is not rounded to decide its type. Keep `serde_json`'s
`arbitrary_precision` feature enabled when parsing inputs. Already-rounded
caller values cannot recover their original precision.

## Select subtrees

```rust
use json_schema_profiler::{ProfilePath, Profiler, ProfilerOptions, Scope};

let scope = Scope::selected([
    ProfilePath::root().property("hardware").property("cores"),
    ProfilePath::root().property("interfaces").each_item().property("address"),
])?;
let mut profiler = Profiler::new(ProfilerOptions::default().with_scope(scope));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Selecting a path includes its whole subtree. Unselected sibling values are
skipped, and their containing objects remain open in generated schemas. Ancestors
must have the container types implied by the selection when present; a mismatch
is an error. Missing selected paths are permitted during ingestion, but a
selection never observed produces `Suggestion::Insufficient`.

`ProfilePath::from_property_pointer` decodes RFC 6901 escaping for object-property
paths. Numerical names and `*` are literal properties. Use `each_item()` for
array traversal; fixed array indices are not supported.

## Document evidence and replay verification

```rust
use json_schema_profiler::{Document, DocumentId, EvaluationOptions, InferencePolicy,
    Profiler, SchemaOptions, Suggestion};
use serde_json::json;

let documents = [json!({"cores":4}), json!({"cores":8})];
let ids = [DocumentId::new("node-a")?, DocumentId::new("node-b")?];
let mut profiler = Profiler::default();
for (id, value) in ids.iter().zip(&documents) {
    profiler.observe_with_id(id, value)?;
}
let profile = profiler.finish()?;
let Suggestion::Candidate(candidate) = profile.suggest(InferencePolicy::strict())? else {
    panic!("inspect blocked or insufficient findings");
};
let compiled = candidate.compile(SchemaOptions::default())?;
let replay = documents.iter().zip(&ids).map(|(value, id)| Document::new(value).with_id(id));
let verified = compiled.verify(replay, EvaluationOptions::default())
    .expect("the complete supplied corpus should pass");
assert_eq!(verified.evaluation().valid(), 2);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Use `CompiledSchema::new(&schema, options)` for a caller-supplied schema and
`compiled.evaluate(replay, options)` to inspect coverage and violations. A
violation includes a zero-based document index, optional caller ID, instance
pointer, schema pointer and keyword; it does not copy the offending scalar or an
upstream error message. Counts remain exact when diagnostic retention is capped.

`field.evidence()` exposes distinct contributing-document counts, per-kind
document counts and bounded witnesses, separate from occurrence counts. A
document contributes at most once per path and kind, even with repeated array
items. IDs are labels, not deduplication keys: repeated IDs still count as
separate inputs. Witnesses are first-seen and therefore depend on input order.
Missing-property witnesses can be obtained by replaying a `required` constraint;
the profiler does not retrospectively retain IDs of absent properties.

`verify` returns `VerifiedCorpus` only for a nonempty, completely evaluated replay
with no rejected documents. It proves acceptance of that replay under the reported
format policy; it does not prove identity with an earlier profiling input or
predict future data. Evaluation can be complete while diagnostics are truncated;
an input/document resource limit instead marks evaluation incomplete.

When the replay source can fail, use `try_evaluate` or `try_verify` with an
iterator of `Result<Document<'_>, E>`. Do not filter read/parse errors out of the
iterator or convert them into end-of-input: that would hide incomplete coverage.
The first source error stops evaluation and produces
`EvaluationStop::InputError { document_index }`, leaving the failed record
uncounted. `ReplayError<E>` returns the incomplete report and the original error
separately. The error payload is excluded from serialized reports and wrapper
`Debug`/`Display` output, but remains accessible to the caller.

`try_evaluate` returning `Ok` means no source error was encountered; check the
report for invalid documents or incomplete evaluation. `try_verify` distinguishes
`VerificationError::Input` from `VerificationError::Evaluation` and returns
`VerifiedCorpus` only for complete, nonempty, passing coverage. At the document
limit, one lookahead checks for exhaustion: a source error there is an input
failure; an available document instead marks the document limit as exceeded.
Neither outcome polls further input.

## Per-path policies

```rust
use json_schema_profiler::{InferencePolicy, Nullability, ProfilePath, SuggestionOptions};

let options = SuggestionOptions::new(InferencePolicy::strict())
    .with_path_policy(ProfilePath::root().property("metadata"),
        InferencePolicy::expansive())?
    .with_path_policy(ProfilePath::root().property("hardware"),
        InferencePolicy::strict().with_nullability(Nullability::Reject))?;
// Pass options to profile.suggest_with(options).
# Ok::<(), Box<dyn std::error::Error>>(())
```

An override applies to that path and its descendants; its presence option applies
to the property itself. Overlapping overrides and paths absent from the profile
are errors. There are at most 128 overrides. The global policy supplies defaults
elsewhere, and selected-subtree ancestor objects remain open.

## Discovery and strictness

`profile.discover(DiscoveryOptions::default())` reports sparse properties,
unrelated mixed types, null-only/nullable fields, unobserved selections/items,
rare kinds and limited document evidence. Findings retain paths and exact counts,
with the chosen thresholds recorded separately; they do not change a schema.
Rare kinds use distinct contributing documents, while property presence uses
applicable object occurrences. Kind contributions can overlap within one document.
The defaults flag fewer than five contributing documents, kinds present in at most
5% of contributing documents, and properties present in at most 50% of applicable
objects. These are review thresholds, not confidence estimates. Options customize
them and bound findings/report bytes; exceeded output limits return an error.

| Preset | Mixed types | Required properties | Additional properties |
| --- | --- | --- | --- |
| `strict()` | Reject unrelated kinds | Present in every applicable object | Deny in fully selected objects |
| `balanced()` | Preserve observed unions | Present in every applicable object | Allow |
| `expansive()` | Preserve observed unions | All optional | Allow |

All presets preserve observed nulls and widen integer/fractional mixtures to
`number`. Each policy axis can be overridden globally or for a selected subtree.

```rust
use json_schema_profiler::{Frequency, InferencePolicy, Presence, SuggestionOptions};

let policy = InferencePolicy::balanced()
    .with_presence(Presence::AtLeast(Frequency::percent(95)?));
let options = SuggestionOptions::new(policy).with_minimum_documents(5)?;
// Pass options to profile.suggest_with(options).
# Ok::<(), Box<dyn std::error::Error>>(())
```

`AtLeast(95%)` explicitly requires properties present in at least 95% of their
applicable object occurrences. It can reject observed documents with a missing
property; `RequiredFromFrequency` reports the exact counts and threshold. Use
replay to measure acceptance before adopting that constraint. No rare observed
type is silently discarded. `with_minimum_documents` requires sufficient distinct
contributing records at each reported path, including array item paths; repeated
items cannot substitute for more documents. Insufficient evidence produces
`Suggestion::Insufficient`. The default remains one record and preserves the
existing empty-array behavior.

## Replay records parsed on demand

Use `OwnedDocument` with `evaluate_owned`/`verify_owned` or their `try_` variants
when an iterator parses each record as it is read. The owned and borrowed APIs
share the same evaluator, counts, limits, source-error handling and single-record
lookahead. Each owned record is dropped before the next is requested; the library
does not accumulate the parsed corpus. Callers still own parsing, reader buffering
and snapshot consistency. A parser's allocation happens before admission checks.

```rust
use json_schema_profiler::{CompiledSchema, EvaluationOptions, OwnedDocument, SchemaOptions};
use serde_json::{Value, json};

let compiled = CompiledSchema::new(&json!({"type":"integer"}), SchemaOptions::default())?;
let records = ["1", "2"].into_iter()
    .map(|line| serde_json::from_str::<Value>(line).map(OwnedDocument::new));
let verified = compiled.try_verify_owned(records, EvaluationOptions::default())?;
assert_eq!(verified.evaluation().valid(), 2);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Run `cargo run --example on_demand` for a complete two-pass JSON Lines example.

## Semantics and limits

- Occurrence counts describe applicable parents; document evidence counts distinct
  input records. Missing and explicit null are separate observations.
- Profiles retain paths, counts and optional caller IDs, but no scalar values or
  document copies. Paths and IDs may themselves contain sensitive information.
- Candidates use Draft 2020-12 structural keywords. They are intended to accept
  the supplied corpus under the chosen policy. Use replay validation to verify
  coverage; suggestion alone is not a verification result.
- Object merging loses correlations between fields and may accept combinations
  never observed. Neither preset predicts the intended future contract.
- Empty arrays provide no item-type evidence and generate `items: true` with a
  finding. With null rejection, they instead exclude null items without inventing
  other item types. Null-only observations generate `type: "null"` when allowed.
- Default admission limits are one million documents, one million visited nodes
  per document, depth 64, 10,000 observed paths, and 4 MB of logical path payload.
  `Limits::builder()` configures these and the 8 MB compact-JSON profile limit.
  Empty-array item placeholders are report nodes without observed paths.
- Witness defaults are two IDs per path/kind, 256 retained entries globally and
  16 KB of ID payload. IDs contain 1–256 UTF-8 bytes. `EvidenceLimits` can disable
  or change retention without changing counts.
- `OutputLimits` defaults to 1 MB schema output, 4 MB suggestion output and 50,000
  findings. `suggest` and `suggest_with` return a `Result`; limits never silently
  discard policy conflicts or return an incomplete candidate.
- Evaluation defaults to one million documents, 256 diagnostics globally, eight
  per document and 1 MB report output. `ValueLimits` checks nodes, depth and compact
  JSON bytes before validation; number tokens are capped at 1,024 bytes and
  exponent magnitude 4,096 to bound arbitrary-precision expansion.
- These are logical and encoded-output bounds, not exact heap or execution-time
  guarantees. See the [resource audit](https://github.com/terjekv/json-schema-profiler/blob/main/docs/resources.md).
- Any ingestion error makes the profiler terminally incomplete. `finish()` rejects
  incomplete and empty corpora instead of returning a partial success.

Read the [integration findings](https://github.com/terjekv/json-schema-profiler/blob/main/docs/experiment.md) for measured overhead,
upstream gaps, and the recommendation. The [original proposal](https://github.com/terjekv/json-schema-profiler/blob/main/docs/proposal.md)
describes the larger v0.0.1 target; the [upstream assessment](https://github.com/terjekv/json-schema-profiler/blob/main/docs/upstream-assessment.md)
records source review of `schema_analysis` and `genson-rs`.

The evaluator supports a documented Draft 2020-12 subset with local JSON Pointer
references. It rejects external references, recursive schemas, dynamic references,
named-anchor references and nested resource IDs. Formats are annotations unless
`FormatPolicy::Assert` is selected. Regex assertions use the Rust regex engine;
lookaround and backreferences are rejected. See the
[tested keyword and format matrix](https://github.com/terjekv/json-schema-profiler/blob/main/docs/validation.md).

## Development

Gungraun benchmark targets also run under `cargo test --all-targets`; install
Valgrind and the matching runner first. CI tests stable and the Rust 1.90.0 MSRV.

```sh
cargo install gungraun-runner --version 0.19.4 --locked
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo test --doc --locked
cargo clippy --all-targets --locked -- -D warnings
npx --yes markdownlint-cli2@0.23.3 --config .markdownlint.json "**/*.md" "!target"
```

See [benchmark methodology and commands](https://github.com/terjekv/json-schema-profiler/blob/main/docs/benchmarks.md). PR comparison uses
`terjekv/rust-pr-bench` v1.3.0, pinned to its release commit, with both Criterion
and Gungraun. CI checks all six benchmark targets. See the
[hosted CI trial](https://github.com/terjekv/json-schema-profiler/blob/main/docs/ci-trial.md) for the comparison contract and review steps.

## License

MIT; see [LICENSE](https://github.com/terjekv/json-schema-profiler/blob/main/LICENSE).

See [release readiness and compatibility](https://github.com/terjekv/json-schema-profiler/blob/main/docs/release.md) and the
[dependency license inventory](https://github.com/terjekv/json-schema-profiler/blob/main/docs/dependency-licenses.md).
