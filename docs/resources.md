# Resource boundaries

These bounds control admitted structure and retained/encoded output. They are
not a hard process-memory ceiling, cancellation mechanism or execution deadline.
The caller owns parsed input, and third-party validation may allocate temporary
state independently of report retention.

| Stage | Enforced boundary | Failure behavior |
| --- | --- | --- |
| Profile ingestion | Documents, visited nodes, depth, observed paths and logical path payload | Terminal profiler failure; no completed profile |
| Profile finalization | Compact serialized profile bytes | Explicit `ReportBytes` error |
| Witnesses | Entries per path/kind, global entries, global ID payload; IDs at most 256 bytes | Mark truncation; exact counts remain complete |
| Discovery | Findings and compact report bytes | Explicit error; no partial discovery |
| Suggestion | At most 128 disjoint overrides; findings, schema bytes and suggestion bytes | Explicit error; no partial candidate |
| Compilation | Schema nodes/depth/bytes/numbers, supported references, expansion depth and regex limits | Compilation/configuration error |
| Replay input | Documents and per-document nodes/depth/bytes/numbers | Incomplete evaluation; offending document uncounted |
| Replay source | First caller error, including document-limit lookahead | Incomplete evaluation; error returned separately; no further polling |
| Replay diagnostics | Global/per-document entries and complete encoded report bytes | Truncate diagnostics while continuing coverage |

Compact JSON checks use a bounded counting sink, avoiding a second serialized
buffer. They account for escaping, not just raw UTF-8. Profile/suggestion checks
occur after bounded structural output is assembled; they do not bound that
assembly's peak allocation.

The path budget sums full structural path payloads. Repeated paths do not consume
another slot, while ancestor names repeated in distinct paths do consume payload.
The index/upstream tree add collection overhead. Empty-array item placeholders
have no observed-path slot and can add report nodes. Witness bytes count every
retained ID copy rather than assuming shared storage.

The upstream object visitor tracks presence by retained field index. The first
64 flags occupy 64 inline bytes; wider objects allocate additional packed
words, including growth when new fields appear. This scratch storage follows
the already admitted object structure and is released after the object visit.
It adds neither retained document values nor new persistent per-field state.

Complete values are preflighted before validation, including literal schema
annotation payloads during compilation. Number tokens are capped at 1,024 bytes
and exponent magnitude 4,096 to prevent unbounded decimal expansion. Profiling
only classifies numbers without expanding exponents, so it can profile numbers
that replay rejects as exceeding its resource policy.

One upstream error may internally own a large instance or nested branch errors
before conversion into a small diagnostic. The diagnostic budget bounds wrapper
output, not upstream intermediate allocations. Combinators, uniqueness checks
and large numbers may still require substantial work within admitted sizes.
Callers requiring a hard deadline or heap ceiling need external isolation.

No instance scalar samples are retained in profiles or wrapper diagnostics.
Compiled schemas and verified evidence do retain the schema, including its
`const`, `enum`, examples and other literals. IDs and property names may also
contain sensitive metadata.

Fallible replay reports retain only the source failure's input index. The
nonserializable `ReplayError<E>` wrapper owns the original caller error without
cloning or formatting it; its size and contents are caller-controlled and outside
the report-byte budget. Wrapper `Debug`/`Display` output omits the error payload,
while explicit accessors and the standard error source chain expose it.

Tests cover cumulative admission, repeated array paths, failed finalization,
witness truncation, escaped output budgets, schema limits, reference cycles/depth,
large exponents, diagnostic caps and partial coverage. Benchmarks record
allocation volume, not a universal maximum live-memory figure.

Owned replay retains one current parsed record and the existing bounded report.
It drops that record before polling the next (including the final lookahead).
The iterator may itself retain arbitrary storage; this is outside library bounds.
Parsing and allocation happen before `ValueLimits` preflight. The example reader
uses immutable source bytes as a synthetic snapshot, not a parsing-memory limit.
Discovery retains structural paths/counts and options, without scalar values or IDs.
