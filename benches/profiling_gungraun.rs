use std::hint::black_box;

use gungraun::{Dhat, LibraryBenchmarkConfig, library_benchmark, library_benchmark_group, main};
use serde_json::Value;

mod support;
mod upstream;

fn setup(workload: &'static str, engine: &'static str) -> (&'static str, Vec<Value>) {
    (engine, support::corpus(workload, 128, 32))
}

#[library_benchmark(setup = setup)]
#[bench::published_homogeneous_minimal("homogeneous", "minimal")]
#[bench::published_homogeneous_default("homogeneous", "default")]
#[bench::published_homogeneous_counts("homogeneous", "counts")]
#[bench::homogeneous_profiler("homogeneous", "profiler")]
#[bench::published_sparse_minimal("sparse", "minimal")]
#[bench::published_sparse_default("sparse", "default")]
#[bench::published_sparse_counts("sparse", "counts")]
#[bench::sparse_profiler("sparse", "profiler")]
#[bench::published_arrays_counts("arrays", "counts")]
#[bench::arrays_profiler("arrays", "profiler")]
#[bench::published_wide_counts("wide", "counts")]
#[bench::wide_profiler("wide", "profiler")]
fn compare((engine, corpus): (&str, Vec<Value>)) -> Vec<Value> {
    upstream::run(engine, black_box(&corpus));
    corpus
}

library_benchmark_group!(name = comparisons; benchmarks = compare);
main!(config = LibraryBenchmarkConfig::default().tool(Dhat::with_args(["--num-callers=256"])); library_benchmark_groups = comparisons);
