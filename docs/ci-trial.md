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
description. The original trial did not prepare a release; current publication
status is recorded in [release readiness](release.md).

## Current benchmark setup

The workflow uses the supported composite action pinned to the same v1.3.0 SHA.
This allows `scripts/install-valgrind.sh` to run before action setup. An existing
Valgrind is reused; otherwise apt update/install each have at most three attempts,
240-second attempt timeouts and a 900-second total timeout (plus a 15-second kill
grace). Apt transport retries are bounded too. Exhaustion/timeout fails the job
before measurements, and the workflow's explicit Bash pipefail preserves failure
through log capture. Setup attempt logs are uploaded even on failure.

The three Criterion and three Gungraun targets and measurement settings are
preserved. New discovery/policy/replay cases have no historical baseline; raw
upstream cases now have `published_` identities because they use registry 0.7.0
rather than the former patched dependency. Treat these as explicitly changed
baseline definitions, never as unchanged or improved measurements. All unchanged
profiler/validation case pairs still require full coverage. The 3% instruction
and 15% timing gates remain unchanged. Review added/removed cases in the hosted
report separately; local mock-installation tests do not replace a hosted run.

## Pre-release comparison (2026-10-25)

[PR #12](https://github.com/terjekv/json-schema-profiler/pull/12) passed
[stable/MSRV CI](https://github.com/terjekv/json-schema-profiler/actions/runs/40185961081)
and the [six-target comparison](https://github.com/terjekv/json-schema-profiler/actions/runs/40185961056)
at `39947fa`, against base `417c311`. All 78 unchanged timing and 29 unchanged
instruction cases have both measurements. The 44 timing and 13 instruction
baseline renames are exactly accounted for, and all 14 new timing cases appear.
The report's zero/unknown cells belong to these documented one-sided cases;
they do not mean unchanged performance. No expected measurement is missing.

Totals are 122 base/136 head timing cases and 42 instruction cases on each side.
The largest paired increases are 10.18% timing and 0.28% instructions, below the
unchanged 15%/3% thresholds. No regression exceptions or reruns were used.
Valgrind update/install succeeded on attempt one. Artifact
`benchmark-evidence-40185961056-1` contains setup logs, the complete report and
build/cache data. The [PR report](https://github.com/terjekv/json-schema-profiler/pull/12#issuecomment-6396879148)
records all case values; this result resolves the hosted acceptance requirement
for benchmark installation issue #11.
