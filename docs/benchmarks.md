# Benchmark experiment

These are the initial adapter measurements, before document evidence, output
budget checks and public replay validation were added. See the
[v0.0.1 measurements](benchmarks-v001.md) for the current implementation.

Measured 2026-09-24 on a shared Linux x86_64 host, Intel Xeon Silver 4216 at
2.10 GHz, 32 logical CPUs, Rust 1.98.0 and Valgrind 3.26.0. Results are exploratory
local measurements, not deployment throughput promises. Wall-clock suites ran
sequentially without concurrent compilation or Valgrind runs initiated by this
experiment; other host activity was not controlled.

## Coverage and baselines

The initial suite had 65 Criterion timing cases and 23 Gungraun cases. Each Gungraun case runs
Callgrind for instructions and DHAT for allocation volume. All fixtures are
deterministic synthetic data generated before the measured ingestion loop.

| Target | Cases | Coverage |
| --- | --- | --- |
| `profiling_criterion` | 48 | Six corpus shapes, 64/1,024 documents, four engines |
| `policies_criterion` | 17 | Whole/subtree selection, strict/expansive generation, parsing plus profiling |
| `profiling_gungraun` | 12 | Upstream/wrapper instructions and allocations across homogeneous, sparse, array and wide documents |
| `scaling_gungraun` | 11 | Document count, width 16/64/256, depth 8/32, dynamic keys, whole/subtree selection |

The four engines share incremental borrowed `Value` input and a virtual
one-document array envelope:

- `minimal`: upstream `Context = ()`, structural inference without counters.
- `default`: upstream `DefaultContext`, including its default aggregation and
  scalar sampling behavior.
- `counts`: an independent minimal custom upstream context with scalar counts,
  object key frequencies and array item totals.
- `profiler`: the actual library, including selection, exact-number handling,
  admission checks, incremental inference, and conversion into its public report.

The `counts` baseline is the most useful comparison for wrapper overhead.
Baselines do not implement equivalent policies, report APIs or resource limits,
and do not solve the arbitrary-precision numeric compatibility problem. The
performance corpora use ordinary small numbers, so those semantic differences
are covered by correctness tests rather than disguised as a speed comparison.

Profiling timings include construction, ingestion, finalization and destruction
of inferred state; caller-owned input construction and destruction are excluded.
The separate parse-and-profile case includes parsing and destroying its input.
Generation timings operate on an already-completed profile. A strict result for
the mixed corpus is a blocked finding, not a generated schema, so its cost should
not be compared as equivalent output to expansive generation.

## Timing results

Criterion used 30 samples, 0.3 seconds of warmup and a requested one second of
measurement per case. Longer cases may require longer measurement windows.
Values below are medians in milliseconds for 1,024 documents.

| Corpus | Minimal upstream | Default upstream | Counts upstream | Profiler | Profiler / counts |
| --- | --- | --- | --- | --- | --- |
| Homogeneous objects | 0.558 | 1.002 | 0.867 | 1.081 | 1.25× |
| Sparse/null properties, width 32 | 5.166 | 5.439 | 7.054 | 8.676 | 1.23× |
| Mixed types | 0.348 | 0.594 | 0.474 | 0.579 | 1.22× |
| Arrays of objects, 16 elements | 4.758 | 8.349 | 7.248 | 9.119 | 1.26× |
| Wide objects, 128 properties | 72.790 | 74.024 | 84.440 | 99.236 | 1.18× |
| Nested objects, depth 24 | 3.116 | 3.177 | 4.199 | 5.325 | 1.27× |

Whole-document profiling of the selection fixture took 15.969 ms; selecting just
`hardware` took 0.376 ms, about 42× faster. That fixture intentionally contains
32 irrelevant nested array elements in each document. This is evidence that
selection avoids work, not a general expected speedup for arbitrary corpora.
Parse-and-profile for 1,024 homogeneous documents took about 2.77 ms.

The first implementation stored full paths in a set and repeatedly allocated
path segments during traversal. Replacing that with an admitted-child index
reduced homogeneous profiling from 1.518 to 1.081 ms, arrays from 14.833 to
9.119 ms, and deep objects from 17.734 to 5.325 ms. Before/after runs were
sequential, not randomized paired trials; instruction/allocation evidence
supports the optimization in addition to timings.

## Instructions and allocation volume

These comparisons use 128 documents, with width or array length 32 where applicable.
DHAT total bytes are cumulative allocations
attributed to the measured function, not peak live memory or retained profile
size. Input setup allocations are excluded. We use 256 caller frames so deep
Serde stacks remain attributable to the benchmark entrypoint. Profile changes
that exceed that stack depth would need a new attribution check.

| Corpus / engine | Instructions | Allocated bytes | Allocation blocks |
| --- | --- | --- | --- |
| Homogeneous / minimal | 577,803 | 30,334 | 1,043 |
| Homogeneous / default | 6,287,932 | 2,902,300 | 8,524 |
| Homogeneous / counts | 860,393 | 35,486 | 1,813 |
| Homogeneous / profiler | 1,062,011 | 40,187 | 1,861 |
| Sparse / counts | 6,566,139 | 257,898 | 6,042 |
| Sparse / profiler | 8,258,142 | 297,674 | 6,223 |
| Arrays / counts | 13,919,859 | 517,050 | 29,330 |
| Arrays / profiler | 16,955,254 | 522,021 | 29,383 |
| Wide / counts | 9,687,213 | 285,676 | 8,784 |
| Wide / profiler | 11,840,772 | 325,820 | 8,966 |

The wrapper adds about 22–26% instructions in these comparisons. Its default
counters-only context avoids the sample retention and auxiliary work of the
upstream default context. Default-context startup costs are more visible at 128
documents than in the 1,024-document timing table.

For 64 documents, width 16/64/256 required 2.34/16.85/173.57 million instructions.
That growth is substantially worse than linear in width. The upstream visitor's
repeated linear key-membership checks are a plausible contributor based on source
inspection; an upstream patch and paired measurement are needed to establish
the size of that contribution.

Dynamic-key profiles grew from 128 to 1,024 distinct properties and consumed
about 0.248 MB versus 2.022 MB of cumulative allocations. Limits matter even
when scalar values are not retained. These measurements do not establish a
hard upper bound on peak process memory.

All medians, their Criterion confidence intervals, instruction counts and
allocation totals are recorded in [benchmark-results.json](benchmark-results.json),
including the initial path-set implementation for comparison. Raw local profiler
artifacts are under `target/criterion` and `target/gungraun` and are not committed.

## Reproduce

Install Valgrind through the system package manager and the exact runner version:

```sh
cargo install gungraun-runner --version 0.19.4 --locked
cargo bench --no-run --locked
cargo bench --bench profiling_criterion --locked -- --noplot --sample-size 30 --warm-up-time 0.3 --measurement-time 1
cargo bench --bench policies_criterion --locked -- --noplot --sample-size 30 --warm-up-time 0.3 --measurement-time 1
cargo bench --bench profiling_gungraun --locked -- --save-summary=json
cargo bench --bench scaling_gungraun --locked -- --save-summary=json
```

Run the targets sequentially for comparable local timing. Inspect reported case
counts: positional Gungraun arguments are filters. A named saved baseline uses
`--save-baseline=local_baseline`, with the equals sign and an underscore in the
name. Saved baseline files differ from ordinary latest-run output files; do not
accidentally compare an old debug test run against release benchmark data.

## Pull-request integration

[The benchmark workflow](../.github/workflows/benchmarks.yml) uses
[`rust-pr-bench` v1.3.0](https://github.com/terjekv/rust-pr-bench/releases/tag/v1.3.0)
at commit `d0202becabbd115351a879c21f53ab9da6812a44`. It enables both backends,
discovers all six current benchmark targets, and groups their compilation. PR timing
runs use 40 samples, one second of warmup and three seconds of measurement.
Median timing regressions above 15% and instruction regressions above 3% fail
the benchmark job; those initial thresholds may need calibration on CI runners.

DHAT runs provide allocation diagnostics, but v1.3.0's regression comparison
consumes Callgrind instructions and Criterion timing, not DHAT allocation totals.
Memory regressions therefore still require inspecting the DHAT evidence.

Initial checks passed for workflow syntax, v1.3.0 matrix discovery (four correctly
routed targets), and v1.3.0 metric collectors against all 65 timing and 23
instruction results. The local collector check used isolated copies of the
latest release artifacts to exclude older runs and saved baselines. There is no
configured remote or hosted PR run yet; this does not claim an end-to-end GitHub
comparison or publication.

The v0.0.1 run extends discovery to six targets and adds 41 Criterion and 12
Gungraun cases for validation, compilation, evidence and the complete workflow.
