# Hosted CI trial

The first PR exercises GitHub Actions and `rust-pr-bench` v1.3.0 against the
initial implementation on `main`. Its documentation changes leave library code,
dependencies, benchmark fixtures and measurement settings unchanged. This makes
the comparison a control for integration failures and runner variability.

## Expected checks

- CI runs tests and documentation examples on Rust 1.90.0, including the test
  that enables the validator dependency's file-resolution feature.
- Stable Rust checks formatting, Clippy, tests, examples and all benchmark
  targets. Markdown lint checks the documentation.
- Benchmark discovery finds three Criterion targets and three Gungraun targets.
  Base and head each contain 106 timing cases and 35 instruction-count cases.
- The PR receives benchmark reports and downloadable measurement artifacts.
  Confirm that every case has both base and head results; a missing measurement
  is not evidence of unchanged performance.
- Every pushed commit is signed. Verify signatures locally with
  `git verify-commit HEAD` and inspect GitHub's commit verification status.

## Reviewing the comparison

Gungraun gates instruction-count increases above 3%; Criterion gates median
timing increases above 15%. The workflow fails on unaccepted regressions. An
unchanged implementation should have stable instruction counts, while hosted
timings can vary. Investigate any failure before changing thresholds or accepting
a regression. Preserve the original result when a rerun tests a noise hypothesis.

Gungraun's DHAT allocation totals remain supplementary evidence: v1.3.0 does not
gate allocation regressions. Local measurements and their source fingerprints
remain in [the v0.0.1 benchmark report](benchmarks-v001.md); hosted timings come
from different hardware and should be compared within the hosted base/head pair.

Record actual run links, case coverage, failures and fixes in the trial PR
description. The trial does not prepare a release; `publish = false` remains set.
