# Standalone inventory consumer trial

Completed 2026-10-01 using the public API after fallible replay landed in
[PR #2](https://github.com/terjekv/json-schema-profiler/pull/2).
The [inventory application](https://github.com/terjekv/json-schema-profiler/blob/main/consumers/inventory-trial/src/main.rs) is a separate,
unpublished Cargo package with its own lockfile. It depends on this checkout
through a path dependency; no library internals or development-only upstream
oracle are used by its runtime. This is the application trial for
[issue #4](https://github.com/terjekv/json-schema-profiler/issues/4), not the
registry-package audit.

## Reproduce

From the repository root, run:

```sh
cargo run --manifest-path consumers/inventory-trial/Cargo.toml --locked
cargo test --manifest-path consumers/inventory-trial/Cargo.toml --locked
cargo +1.90.0 test --manifest-path consumers/inventory-trial/Cargo.toml --locked
cargo clippy --manifest-path consumers/inventory-trial/Cargo.toml --all-targets --locked -- -D warnings
cargo fmt --manifest-path consumers/inventory-trial/Cargo.toml --all -- --check
```

The program prints findings, the reviewed candidate, bounded reports, and elapsed
stage times. It exits unsuccessfully if a demonstrated contract is violated.
CI runs its tests and walkthrough on stable and Rust 1.90, and checks its formatting
and Clippy on stable. The repository commit hook includes the standalone package.

## Observed workflow

The synthetic inventory has three labeled records. Only `hardware` and
`interfaces` are profiled. Unselected sibling values are omitted from the profile
and remain unconstrained by the generated schema.

| Scenario | Observed result |
| --- | --- |
| Missing and null owner | Two occurrences, one missing parent contribution, one explicit null |
| Repeated interface addresses | Three occurrences from two distinct documents |
| One witness per kind | Witness truncation is reported while the counts remain exact |
| Strict core policy | Integer/string observations produce a mixed-type conflict |
| Explicit core override | Allowing a union only at `/hardware/cores` preserves required presence and closed hardware objects |
| Successful replay | All three records pass; the consumer can apply the approved schema |
| Read failure at record 1 | One record passes, then input failure prevents approval |
| Invalid core type | A labeled `/hardware/cores` diagnostic identifies the rejection without retaining the scalar |
| Three rejected records, one diagnostic | Complete coverage counts three rejections and reports diagnostic truncation |
| Malformed JSON at record 1 | The parse error remains recoverable; prior coverage is incomplete |
| Document or value limit | Incomplete evaluation cannot produce an application approval |
| Changed revision | Rejected both before replay and between verification and application |
| Another inventory at the same revision number | Rejected using consumer-owned dataset identity |

Fourteen focused cases in [the consumer tests](https://github.com/terjekv/json-schema-profiler/blob/main/consumers/inventory-trial/src/tests.rs)
cover these contracts, including negative inputs with missing required cores,
boolean cores, and an unexpected hardware property.

## Snapshot and ownership boundaries

The [application model](https://github.com/terjekv/json-schema-profiler/blob/main/consumers/inventory-trial/src/inventory.rs) owns dataset
identity, a monotonic revision, source strings, parsed records, and the applied
schema. Private fields carry the identity/revision from analysis through proposal,
compilation, and successful verification. Every row replacement advances the
revision. Applying a schema requires matching both identity and revision.
The example's exclusive mutable borrow makes the check and in-memory write one
operation. A persistent consumer needs a durable dataset identifier and an atomic
revision comparison and schema write, plus its own authorization checks.

The snapshot owns `Result<Record, io::Error>` entries. Its replay iterator borrows
values, labels, and original errors. Parsing/read failures are retained as errors,
never filtered out. Profiling aborts on source failure without finalizing a partial
profile. The library does not claim that a replay matches earlier profiling;
the consumer supplies that guarantee through its immutable snapshot and revision.

This fixture intentionally retains three parsed records and their source strings.
Profiling has explicit admission and witness limits; replay uses the library's
document/value/report defaults except in deliberate limit cases. Those library
limits do not bound the application's parsing or snapshot memory. No production
documents, database, service, network client, or runtime is involved.

## Integration costs and follow-up

One debug-build walkthrough on the shared x86_64 development host, Rust 1.98.0,
measured these stage costs. Cargo compilation is excluded; schema compilation
includes first-use validator initialization. These three-record measurements are
illustrative integration costs, not a throughput comparison or resource guarantee.

| Stage | Elapsed |
| --- | --- |
| Snapshot parse | 61.8 µs |
| Profile and finalize | 522.5 µs |
| Suggest after explicit policy review | 187.9 µs |
| Compile candidate | 17.4 ms |
| Verify replay | 86.0 µs |

Compact serialized profile and candidate outputs were 2,864 and 1,248 bytes.
These are encoded payload sizes, not live heap measurements. The program reports
fresh timings and sizes on each run; timings vary by build and machine.

The borrowed API fits an application with stable parsed-record storage. The
follow-up for [issue #7](https://github.com/terjekv/json-schema-profiler/issues/7)
adds `OwnedDocument` and owned replay methods for a caller parsing records on demand.
`examples/on_demand.rs` demonstrates two passes over a synthetic immutable reader,
without retaining all parsed records. The additive API is justified by Rust's
borrowing constraints: a standard iterator cannot yield a reference to a temporary
parsed value. It shares the existing corpus evaluator, preserving global limits,
indices, lookahead, source failures and verification evidence. Existing borrowed
callers require no migration.

The `parse_and_replay/{borrowed,owned}/1024` Criterion cases include parsing the
same synthetic source strings in both variants. Borrowed replay retains 1,024
parsed values; owned replay holds one at a time. Source storage is outside that
comparison. Timings are integration costs, not peak-memory guarantees.

No library API change was required for this trial. The root crate remains generic,
and publication remains disabled pending the separate release audit.

### Owned replay follow-up measurement (2026-10-03)

A local optimized build on the shared Intel Xeon Silver 4216 host, Rust 1.98.0,
measured the paired 1,024-record parsing/replay cases. Criterion used 30 samples,
0.3 seconds of warmup and one requested second of measurement. Fixtures were
prepared before timing; both variants parsed the same strings and verified the
same schema with default evaluation limits. No other builds or benchmarks were
started by this task during measurement; unrelated host activity was uncontrolled.

| Replay | Median milliseconds | Parsed records retained by replay input |
| --- | --- | --- |
| Borrowed | 0.964 | 1,024 |
| Owned on demand | 0.832 | 1 |

These are one-run integration measurements, not a throughput or heap guarantee.
Reproduce with `cargo bench --bench validation_criterion -- parse_and_replay
--noplot --sample-size 30 --warm-up-time 0.3 --measurement-time 1` (one command).
