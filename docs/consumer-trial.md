# Standalone inventory consumer trial

Completed 2026-10-01 using the public API after fallible replay landed in
[PR #2](https://github.com/terjekv/json-schema-profiler/pull/2).
The [inventory application](../consumers/inventory-trial/src/main.rs) is a separate,
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

Fourteen focused cases in [the consumer tests](../consumers/inventory-trial/src/tests.rs)
cover these contracts, including negative inputs with missing required cores,
boolean cores, and an unexpected hardware property.

## Snapshot and ownership boundaries

The [application model](../consumers/inventory-trial/src/inventory.rs) owns dataset
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

The borrowed API fits an application with stable parsed-record storage. A standard
iterator cannot parse a new owned value and yield a reference that outlives that
value. Large on-demand readers would otherwise need caller-owned storage or a
different replay interface. [Issue #7](https://github.com/terjekv/json-schema-profiler/issues/7)
records an evaluation of owned-record/callback replay while preserving global
counts, source errors, limits, and verification semantics. Independently verifying
each row is not a substitute for complete corpus verification.

No library API change was required for this trial. The root crate remains generic,
and publication remains disabled pending the separate release audit.
