# Wide-object presence tracking

Measured 2026-09-26 against `main` commit
`63348f59abd8f28994560946091ab93b6525b51b`. The investigation confirms that repeated
linear key-membership scans in `schema_analysis` 0.7.0 dominate wide-object
aggregation. A focused local patch removes those scans without changing public
APIs, occurrence/document counts, policies, or serialized reports.

## Cause and correction

The upstream seeded object visitor calls `keys.contains` while detecting
duplicates and again for each retained field when detecting absence. Repeated
wide objects therefore perform quadratic string-membership work. The
[published source](https://github.com/QuartzLibrary/schema_analysis/blob/d1a1b167ebd6a9fee8e52903481fbac8c0315d4c/schema_analysis/src/analysis/schema_seed.rs)
contains both scans.

The correction uses the field index returned by the existing `OrderMap` lookup.
Indices remain stable during a visit because fields are only appended. A
temporary presence map records duplicate keys; the first 64 byte flags are
inline, with packed 64-bit spill words for wider objects. New fields are marked
at their append index. The missing-field pass traverses the map in that same index order.
Presence tracking itself is linear in incoming keys plus retained fields;
hash lookups, counters, admission, and reporting retain their existing costs.

The ordered key vector still reaches the context unchanged, including duplicate
entries. Missing/null/duplicate flags remain cumulative. No scalar retention,
schema policy, selection behavior, numeric adapter, or validation code changes.
Root-null merging and broader upstream API changes are outside this patch.

An initial per-object boolean-vector implementation improved wide cases but
increased homogeneous counters/profiler and array instruction counts by roughly
5–6%, with additional allocations even for small objects. It was rejected.
An inline-bit variant removed those allocations but retained a small-object
timing cost. Inline byte flags simplify that hot path to indexed loads/stores;
the compact spill words keep wider-object storage small. The complete results
preserve the earlier measurements and longer small-object comparisons too.

## Measurements

Both revisions used identical expanded benchmark entrypoints, synthetic parsed
inputs, default profiler limits/options, Rust 1.98.0, Valgrind 3.26.0, and the same
shared Intel Xeon Silver 4216 host with 32 logical CPUs. Fixture construction is
outside the measured loop. Targets ran sequentially, without concurrent builds
or Valgrind runs initiated during timing; other host activity was uncontrolled.
Instructions/allocations and final timing were measured baseline first. This is
a paired comparison, not a randomized trial or throughput promise.

Criterion used 30 samples, 0.3 seconds of warmup, and a requested one second per
case, extended automatically for slow cases. Values below are median milliseconds
and millions of instructions for 64 dense objects.

| Fields | Baseline ms | Patched ms | Timing change | Baseline M instructions | Patched M instructions | Instruction change |
| --- | --- | --- | --- | --- | --- | --- |
| 16 | 0.345 | 0.274 | -20.53% | 2.640 | 2.118 | -19.80% |
| 64 | 2.200 | 1.271 | -42.23% | 17.950 | 9.335 | -48.00% |
| 256 | 20.168 | 5.948 | -70.51% | 178.470 | 40.701 | -77.19% |
| 1,024 | 253.052 | 26.864 | -89.38% | 2,361.230 | 186.958 | -92.08% |

Increasing width from 256 to 1,024 multiplies baseline profiler instructions by
13.2, versus 4.6 after the patch. Direct upstream counters at width 1,024 fall
from 2,286.508 to 114.142 million instructions (-95.01%) and from 239.761 to
15.708 ms (-93.45%). This isolates the large gain to upstream aggregation. The
counters baseline has no selection, admission, exact-number adapter, document
evidence, or public report, so it is not an equivalent replacement for the library.

Sparse width-1,024 profiling falls from 170.619 to 19.889 ms (-88.34%). At width
256, sparse profiling instructions fall by 75.37%. The full comparison includes
81 timing cases and 30 instruction/allocation cases across homogeneous, sparse,
mixed, array, wide, deep, dynamic-key, selection, generation, and parsing work.
All case identities match between revisions; no missing measurement is counted
as an improvement.

The largest instruction increase is 1.12% for depth eight; depth 32 rises 0.73%
and selected-subtree profiling 0.42%. The largest initial timing increase is
13.71% for deep strict schema generation. Both are below the unchanged
3% instruction and 15% timing thresholds.

Longer homogeneous-object follow-ups used 40 samples, one second of warmup and
three requested measurement seconds. Profiling time rose 3.33–3.56%, compared
with 7.03–8.22% for the intermediate inline-bit variant. Minimal upstream time
rose 6.19–8.98%. These measured small-object costs remain an explicit trade-off;
the raw comparisons, including the rejected variants, are retained.

The same longer settings repeated deep generation at +14.36% for strict and
+8.82% for expansive policy. Profile construction is outside that timed loop,
so the modified object visitor is not executed during generation measurement.
The source of this timing difference has not been isolated. It remains below
the timing threshold, and the hosted comparison still gates all generation cases.

DHAT allocations are cumulative volume, not retained size or a heap ceiling.
The presence map adds no allocations in the homogeneous, array, and width-at-most-64
fixtures. At width 1,024 it adds 7,560 bytes and 63 allocation blocks over 64
documents (8,587,220 versus 8,579,660 bytes total). Dynamic-key growth to 1,024
fields adds 63,048 bytes and 974 blocks, while instructions fall 30.73%.
These wider-object allocation costs are an explicit trade-off. The 64 inline
flags also use 64 stack bytes per active object visit, instead of eight bytes in
the intermediate inline-bit variant. Report/evidence limits and retention
semantics remain unchanged.

[Complete paired results](benchmark-results-wide-objects.json) include median
confidence intervals, instructions, allocated bytes/blocks, source hashes, and
the rejected prototype. Compilation and validation behavior are covered by the
regular tests and the six-target hosted benchmark workflow; the local timing
table above concerns profiling and policy workloads.

## Regression coverage and review

`tests/upstream_objects.rs` exercises reordering, missing-field persistence,
existing/new duplicate keys, ordered context inputs, and growth across 64-bit
word boundaries. Differential cases compare full aggregation output with the
published registry dependency across sparse, mixed, escaped-key and duplicate-key
inputs up to width 1,025. Separate profiler regressions distinguish repeated
occurrences from document contributions and independently validate wide schemas,
including negative examples and deterministic output.

The patch was reviewed for the stable-index invariant, new-field growth, sticky
flags, shift/offset bounds, and unchanged context inputs. The runtime source was
compared with the checksum-verified crates.io archive: only
`src/analysis/schema_seed.rs` differs. The dependency configuration also retains
the explicit denying validator retriever and its feature-unification test.
A separate consumer's Cargo metadata resolves the patched local dependency,
without the registry test oracle.

## Reproduce

Create an isolated baseline and give it the same expanded harness before
compiling either revision:

```sh
git worktree add --detach /tmp/profiler-wide-before 63348f59abd8f28994560946091ab93b6525b51b
cp benches/profiling_criterion.rs benches/scaling_gungraun.rs /tmp/profiler-wide-before/benches/
cargo bench --no-run --locked
(cd /tmp/profiler-wide-before && cargo bench --no-run --locked)
```

Run these commands sequentially in each checkout, keeping their `target`
directories separate:

```sh
cargo bench --bench profiling_gungraun --locked -- --save-summary=json
cargo bench --bench scaling_gungraun --locked -- --save-summary=json
cargo bench --bench profiling_criterion --locked -- --noplot --sample-size 30 --warm-up-time 0.3 --measurement-time 1
cargo bench --bench policies_criterion --locked -- --noplot --sample-size 30 --warm-up-time 0.3 --measurement-time 1
```

There should be 12 profiling and 18 scaling Gungraun cases, and 64 profiling plus
17 policy Criterion cases per revision. Collect the current release Gungraun
summary totals and Criterion `new/estimates.json` medians, not stale debug runs or
duplicate `base` files. The hosted workflow also exercises the validation targets
and retains its original regression thresholds.

## Upstream and adoption

On 2026-09-26, upstream still published 0.7.0 and its master branch was
`0648d4dacd899029942f9c28ab4e9b2e8d94c51a`; its PR history contained no equivalent
fix. The [standalone source patch](../patches/schema-analysis-object-presence.patch)
applies at the upstream repository root. Forward-port the regression cases and
submit a focused upstream change with these measurements when contributing it.
No upstream issue or PR was posted by this work.

The library currently adopts the correction through a direct path dependency on
the [vendored runtime source](https://github.com/terjekv/json-schema-profiler/tree/main/vendor/schema_analysis), preserving its
MIT/Apache licenses and original runtime feature/dependency declarations. The
registry copy is a development-only differential-test oracle. This does not
expose upstream implementation types through the profiler API.

Once a fixed upstream release is available, replace the path dependency and
remove the vendor/patch artifacts in a separate change. Re-run the differential
contract, full stable/MSRV suite, reference-retrieval feature check, and benchmark
comparison before adopting that version. Until then, publication remains
disabled: Cargo strips path dependencies when packaging for a registry, so merely
publishing this manifest would select unpatched upstream 0.7.0. The release audit
must settle that dependency first.
