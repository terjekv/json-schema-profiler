use std::hint::black_box;

use gungraun::{Dhat, LibraryBenchmarkConfig, library_benchmark, library_benchmark_group, main};
use json_schema_profiler::{CompiledSchema, Document, EvaluationOptions, SchemaOptions};
use serde_json::Value;

mod validation_support;

type Input = (CompiledSchema, Vec<Value>, EvaluationOptions);

fn setup(workload: &str, count: usize, diagnostics: bool) -> Input {
    let schema = CompiledSchema::new(
        &validation_support::schema(workload),
        SchemaOptions::default(),
    )
    .unwrap();
    let options = if diagnostics {
        EvaluationOptions::default()
    } else {
        EvaluationOptions::default().with_diagnostic_limits(0, 0)
    };
    (schema, validation_support::corpus(workload, count), options)
}

#[library_benchmark(setup=setup)]
#[bench::structural_64("structural", 64, true)]
#[bench::structural_1024("structural", 1024, true)]
#[bench::invalid_64_diagnostics("invalid", 64, true)]
#[bench::invalid_1024_diagnostics("invalid", 1024, true)]
#[bench::invalid_1024_coverage("invalid", 1024, false)]
#[bench::references_128("references", 128, true)]
#[bench::conditional_128("conditional", 128, true)]
#[bench::decimal_128("decimal", 128, true)]
fn evaluation(input: Input) -> Input {
    black_box(input.0.evaluate(
        black_box(&input.1).iter().map(Document::new),
        input.2.clone(),
    ));
    input
}

#[library_benchmark(setup=validation_support::schema)]
#[bench::structural("structural")]
#[bench::references("references")]
#[bench::conditional("conditional")]
#[bench::decimal("decimal")]
fn compilation(schema: Value) -> Value {
    black_box(CompiledSchema::new(black_box(&schema), SchemaOptions::default()).unwrap());
    schema
}

library_benchmark_group!(name=replay; benchmarks=evaluation,compilation);
main!(config=LibraryBenchmarkConfig::default().tool(Dhat::with_args(["--num-callers=256"])); library_benchmark_groups=replay);
