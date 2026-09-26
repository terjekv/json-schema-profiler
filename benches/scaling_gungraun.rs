use std::hint::black_box;

use gungraun::{Dhat, LibraryBenchmarkConfig, library_benchmark, library_benchmark_group, main};
use json_schema_profiler::{ProfilePath, Profiler, ProfilerOptions, Scope};
use serde_json::Value;

mod support;
mod upstream;

fn setup(
    workload: &'static str,
    documents: usize,
    width: usize,
    selected: bool,
) -> (Vec<Value>, ProfilerOptions) {
    let scope = if selected {
        Scope::selected([ProfilePath::root().property("hardware")]).unwrap()
    } else {
        Scope::default()
    };
    (
        support::corpus(workload, documents, width),
        ProfilerOptions::default().with_scope(scope),
    )
}

#[library_benchmark(setup = setup)]
#[bench::documents_64("homogeneous", 64, 8, false)]
#[bench::documents_1024("homogeneous", 1024, 8, false)]
#[bench::width_16("wide", 64, 16, false)]
#[bench::width_64("wide", 64, 64, false)]
#[bench::width_256("wide", 64, 256, false)]
#[bench::width_1024("wide", 64, 1024, false)]
#[bench::sparse_256("sparse", 64, 256, false)]
#[bench::depth_8("deep", 64, 8, false)]
#[bench::depth_32("deep", 64, 32, false)]
#[bench::dynamic_128("dynamic", 128, 1, false)]
#[bench::dynamic_1024("dynamic", 1024, 1, false)]
#[bench::scope_whole("selection", 64, 32, false)]
#[bench::scope_selected("selection", 64, 32, true)]
fn scaling((corpus, options): (Vec<Value>, ProfilerOptions)) -> Vec<Value> {
    let mut profiler = Profiler::new(options);
    for value in black_box(&corpus) {
        profiler.observe(value).unwrap();
    }
    black_box(profiler.finish().unwrap());
    corpus
}

fn setup_counts(workload: &str, width: usize) -> Vec<Value> {
    support::corpus(workload, 64, width)
}

#[library_benchmark(setup = setup_counts)]
#[bench::width_16("wide", 16)]
#[bench::width_64("wide", 64)]
#[bench::width_256("wide", 256)]
#[bench::width_1024("wide", 1024)]
#[bench::sparse_256("sparse", 256)]
fn upstream_counts(corpus: Vec<Value>) -> Vec<Value> {
    upstream::run("counts", black_box(&corpus));
    corpus
}

library_benchmark_group!(name = scales; benchmarks = scaling, upstream_counts);
main!(config = LibraryBenchmarkConfig::default().tool(Dhat::with_args(["--num-callers=256"])); library_benchmark_groups = scales);
