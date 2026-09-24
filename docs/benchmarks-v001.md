# v0.0.1 benchmark results

Measured 2026-09-24 after adding document evidence, output limits, per-path
policies and replay validation. All 106 Criterion and 35 Gungraun cases ran.
Use the [initial report](benchmarks.md) for fixture/baseline definitions and
the [complete results](benchmark-results-v001.json) for medians, confidence
intervals, instruction/allocation counts and source hashes.

The shared host and measurement settings match the initial experiment: Rust
1.98.0, Intel Xeon Silver 4216, Criterion 30 samples / 0.3 s warmup / requested
1 s measurement, Valgrind 3.26.0 and Gungraun 0.19.4. Timing suites ran sequentially
without concurrent builds or Valgrind measurements initiated by this work.
Other host activity was not controlled.

## Profiling with document counts and output checks

Medians below are milliseconds per 1,024 documents. The counters-only upstream
baseline still lacks selection, budgets, document evidence and public reports.

| Corpus | Counts upstream | Current profiler | Profiler / counts |
| --- | --- | --- | --- |
| homogeneous | 0.821 | 1.123 | 1.37× |
| sparse | 6.998 | 8.886 | 1.27× |
| mixed | 0.492 | 0.623 | 1.27× |
| arrays | 6.969 | 9.543 | 1.37× |
| wide | 84.364 | 101.200 | 1.20× |
| deep | 4.325 | 5.622 | 1.30× |

Selecting `hardware` took 0.413 ms versus 17.540 ms for
the whole fixture (42.5× faster). Width remains an upstream performance concern.

## Replay and end-to-end behavior

The structural validation fixture has only `id` and `name`; it is smaller than
the homogeneous profiling fixture. Raw flags use upstream `is_valid` without
wrapper preflight, coverage reports or diagnostics. Their work is not equivalent.
Medians below are milliseconds per 1,024 documents.

| Fixture | Raw validity flags | Coverage only | Bounded diagnostics |
| --- | --- | --- | --- |
| structural | 0.086 | 0.224 | 0.406 |
| invalid | 0.046 | 0.251 | 0.570 |
| references | 2.033 | 3.109 | 6.096 |
| conditional | 0.067 | 0.207 | 0.299 |
| decimal | 1.997 | 2.046 | 2.139 |

Profiling, strict generation, compilation and verification together took 0.858 ms
for 1,024 structural documents. This does not include JSON parsing. Compile once
and reuse the compiled schema when evaluating several corpora.

Warmed schema compilation medians:

| Schema | Microseconds |
| --- | --- |
| structural | 17.43 |
| references | 20.78 |
| conditional | 22.27 |
| decimal | 10.09 |

For 1,024 structural documents, anonymous profiling took
0.409 ms; labeled profiling with bounded witnesses took
0.437 ms. Passing labels with retention disabled took
0.438 ms. Distinct-document counts remain enabled in all three.

## Benchmark-driven coverage optimization

The first replay implementation always requested error iteration. Once diagnostic
retention is disabled or exhausted, it now uses first-error validation while still
recognizing engine failures. This preserves exact document coverage.

| 1,024 invalid documents | Before (ms) | After (ms) |
| --- | --- | --- |
| coverage | 0.856 | 0.251 |
| diagnostics | 1.114 | 0.570 |

The affected validation targets were rerun after this change. Other profiling
targets use the unchanged profiling logic measured in the same experiment.
Before/after runs were sequential, not randomized paired trials.

## Instruction and allocation evidence

DHAT bytes/blocks are cumulative allocations attributed to the measured function,
not retained report size or peak heap. Fixture setup and teardown are excluded.
Compilation cases below include cold process-local meta-schema initialization;
Criterion compilation runs above are warmed and should not be equated with them.

| Validation case | Instructions | Allocated bytes | Allocation blocks |
| --- | --- | --- | --- |
| compilation: conditional | 5,929,078 | 998,616 | 6,780 |
| compilation: decimal | 5,869,169 | 992,940 | 6,693 |
| compilation: references | 5,920,624 | 997,585 | 6,763 |
| compilation: structural | 5,908,090 | 996,579 | 6,753 |
| evaluation: conditional_128 | 291,762 | 10,240 | 384 |
| evaluation: decimal_128 | 1,760,248 | 7,332 | 768 |
| evaluation: invalid_1024_coverage | 2,030,388 | 258,048 | 2,048 |
| evaluation: invalid_1024_diagnostics | 4,267,733 | 391,132 | 3,770 |
| evaluation: invalid_64_diagnostics | 1,859,277 | 128,736 | 1,415 |
| evaluation: references_128 | 6,009,679 | 313,344 | 8,576 |
| evaluation: structural_1024 | 3,243,263 | 114,688 | 4,096 |
| evaluation: structural_64 | 204,918 | 7,168 | 256 |

## Reproduce and PR integration

Run the four original targets using the [initial commands](benchmarks.md#reproduce),
then run the new targets sequentially:

```sh
cargo bench --bench validation_criterion --locked -- --noplot --sample-size 30 --warm-up-time 0.3 --measurement-time 1
cargo bench --bench validation_gungraun --locked -- --save-summary=json
```

The two targets add 41 timing cases and 12 instruction/allocation cases. The PR
workflow remains pinned to `rust-pr-bench` v1.3.0, discovers six targets and uses
longer Criterion measurements. Its matrix and metric readers were checked against
all 106 timing and 35 instruction results. Hosted GitHub execution still requires
a configured remote. DHAT allocation totals are evidence, not an automatic
v1.3.0 regression gate. Intentional feature costs and shared-runner timing noise
must be assessed when reviewing a performance regression.
