use std::{hint::black_box, time::Duration};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

mod support;
mod upstream;

fn compare(c: &mut Criterion) {
    for (workload, width) in [
        ("homogeneous", 8),
        ("sparse", 32),
        ("mixed", 8),
        ("arrays", 16),
        ("wide", 128),
        ("deep", 24),
    ] {
        let mut group = c.benchmark_group(workload);
        for documents in [64, 1024] {
            let corpus = support::corpus(workload, documents, width);
            group.throughput(Throughput::Elements(documents as u64));
            for engine in ["minimal", "default", "counts", "profiler"] {
                group.bench_with_input(
                    BenchmarkId::new(engine, documents),
                    &corpus,
                    |b, corpus| {
                        b.iter(|| upstream::run(engine, black_box(corpus)));
                    },
                );
            }
        }
        group.finish();
    }
}

criterion_group! { name = benches; config = Criterion::default().sample_size(40).warm_up_time(Duration::from_secs(1)).measurement_time(Duration::from_secs(3)); targets = compare }
criterion_main!(benches);
