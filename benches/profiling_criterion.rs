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
                    BenchmarkId::new(
                        if engine == "profiler" {
                            engine.to_owned()
                        } else {
                            format!("published_{engine}")
                        },
                        documents,
                    ),
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

fn width_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("width_scaling");
    for workload in ["wide", "sparse"] {
        for width in [16, 64, 256, 1024] {
            let corpus = support::corpus(workload, 64, width);
            for engine in ["counts", "profiler"] {
                group.bench_with_input(
                    BenchmarkId::new(
                        format!(
                            "{workload}/{}",
                            if engine == "profiler" {
                                engine.to_owned()
                            } else {
                                format!("published_{engine}")
                            }
                        ),
                        width,
                    ),
                    &corpus,
                    |b, corpus| b.iter(|| upstream::run(engine, black_box(corpus))),
                );
            }
        }
    }
    group.finish();
}

criterion_group! { name = benches; config = Criterion::default().sample_size(40).warm_up_time(Duration::from_secs(1)).measurement_time(Duration::from_secs(3)); targets = compare, width_scaling }
criterion_main!(benches);
