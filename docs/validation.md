# Validation support

The evaluator pins `jsonschema` 0.49.9 with `arbitrary-precision` enabled and
default features disabled. It forces Draft 2020-12 and compiles schema syntax
before evaluation. This is a documented subset, not a full-dialect conformance
claim. Positive/negative wrapper tests cover the rows below; the complete
upstream conformance suite was not independently rerun.

## Assertions

| Keywords | Supported behavior and test coverage |
| --- | --- |
| Boolean schemas, `type` | True/false schemas, simple and union types, exact integral/fractional classification |
| `const`, `enum` | Equality/membership; literal objects are data, not nested schemas |
| `minimum`, `maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf` | Numeric assertions, decimal multiples, bounds beyond machine integer precision |
| `minLength`, `maxLength`, `pattern` | String bounds and Rust-regex assertions |
| `minItems`, `maxItems`, `uniqueItems` | Array length and uniqueness |
| `prefixItems`, `items` | Tuple-prefix and remaining-item validation; inference still generates list schemas |
| `contains`, `minContains`, `maxContains` | Matching-item counts |
| `properties`, `patternProperties`, `additionalProperties`, `required` | Property schemas, name patterns, object openness and required keys |
| `propertyNames`, `minProperties`, `maxProperties` | Name schemas and property counts |
| `dependentRequired`, `dependentSchemas` | Presence and schema dependencies |
| `allOf`, `anyOf`, `oneOf`, `not` | Schema combinations, including exclusive matching for supplied `oneOf` |
| `if`, `then`, `else` | Conditional validation |
| `unevaluatedProperties`, `unevaluatedItems` | Unevaluated locations after supported applicators |
| `$defs`, `$ref` | Acyclic local JSON Pointer references, including RFC 6901 escaping and URI percent decoding |

`$schema`, when present, must identify Draft 2020-12. Unknown required
`$vocabulary` entries are rejected; optional unknown vocabularies do not enable
new assertions. Unknown keywords remain annotations under JSON Schema semantics.
Metadata and `contentEncoding`/`contentMediaType`/`contentSchema` do not assert
instance validity. No annotation output tree is exposed.

The wrapper rejects legacy `dependencies`/`additionalItems`, dynamic/recursive
reference keywords, nested `$id`, named-anchor references and reference cycles.
Even unused cyclic definitions are rejected. Noncyclic schema/reference expansion
depth is limited to 128. A root `$id` is allowed, but references must still be
fragment-only JSON Pointers. References into annotation data cause those targets
to be checked as schemas, so annotations cannot bypass retrieval restrictions.

Network and filesystem retrieval are disabled through a denying retriever,
independently of Cargo feature unification. Schema/reference checks precede
upstream compilation. A regression run also enables upstream `resolve-file` and
confirms that forbidden references remain rejected.

## Formats and regular expressions

Formats are annotations by default. `SchemaOptions::with_formats(FormatPolicy::Assert)`
enables assertions and rejects unknown format names. A required format-assertion
vocabulary is rejected unless that policy is enabled. Evaluation reports retain
the policy, including verified replay reports.

These upstream formats each have positive/negative wrapper tests:

| Group | Formats |
| --- | --- |
| Date/time | `date`, `time`, `date-time`, `duration` |
| Addresses/names | `email`, `idn-email`, `hostname`, `idn-hostname`, `ipv4`, `ipv6` |
| Resource identifiers | `uri`, `uri-reference`, `iri`, `iri-reference`, `uri-template`, `uuid` |
| Syntax | `json-pointer`, `relative-json-pointer`, `regex` |

`pattern` and `patternProperties` use the Rust regex engine with 1 MB compilation
and DFA limits. Lookaround and backreferences are rejected during compilation.
This restricts syntax from the wider ECMAScript dialect used by JSON Schema.
If a runtime regex/reference failure prevents establishing validity, evaluation
is incomplete and cannot produce verified evidence.

## Reports

`processed()` counts records actually evaluated; `valid()` and `invalid()`
partition that count. Multiple keyword failures in one document count once as
invalid. Diagnostic limits do not change coverage. Diagnostics expose upstream
top-level failures, not every nested branch error in `anyOf`/`oneOf`.

`Complete` means the supplied iterator was exhausted within limits. Reaching the
document limit consumes at most one lookahead record to distinguish exact
completion from a longer stream. An input-limit failure leaves that document
uncounted. An empty replay can be complete but cannot yield `VerifiedCorpus`.

Violations retain pointers, keywords and optional IDs, without raw scalar values
or upstream formatted messages. Pointers can contain sensitive names. Read the
[resource audit](resources.md) for retained-output and temporary-memory boundaries
and the [upstream API documentation](https://docs.rs/jsonschema/0.49.9/jsonschema/)
for the underlying validator.
