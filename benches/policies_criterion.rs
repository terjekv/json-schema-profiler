use std::{hint::black_box, time::Duration};

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use json_schema_profiler::{
    DiscoveryOptions, InferencePolicy, Profile, ProfilePath, Profiler, ProfilerOptions, Scope,
};
use serde_json::Value;

mod support;

fn profile(corpus: &[Value], options: ProfilerOptions) -> Profile {
    let mut profiler = Profiler::new(options);
    for value in corpus {
        profiler.observe(value).unwrap();
    }
    profiler.finish().unwrap()
}

fn policies(c: &mut Criterion) {
    let mut scopes = c.benchmark_group("scope");
    for documents in [64, 1024] {
        let corpus = support::corpus("selection", documents, 32);
        let selected = Scope::selected([ProfilePath::root().property("hardware")]).unwrap();
        scopes.throughput(Throughput::Elements(documents as u64));
        for (name, options) in [
            ("whole", ProfilerOptions::default()),
            ("selected", ProfilerOptions::default().with_scope(selected)),
        ] {
            scopes.bench_with_input(BenchmarkId::new(name, documents), &corpus, |b, corpus| {
                b.iter(|| black_box(profile(black_box(corpus), options.clone())));
            });
        }
    }
    scopes.finish();

    let mut generation = c.benchmark_group("generation");
    for (workload, width) in [
        ("homogeneous", 8),
        ("sparse", 32),
        ("mixed", 8),
        ("wide", 256),
        ("deep", 32),
        ("dynamic", 8),
    ] {
        let report = profile(
            &support::corpus(workload, 1024, width),
            ProfilerOptions::default(),
        );
        for (name, policy) in [
            ("strict", InferencePolicy::strict()),
            ("expansive", InferencePolicy::expansive()),
            ("balanced", InferencePolicy::balanced()),
        ] {
            generation.bench_function(BenchmarkId::new(workload, name), |b| {
                b.iter(|| black_box(report.suggest(policy).unwrap()))
            });
        }
    }
    generation.finish();
    let mut discovery = c.benchmark_group("discovery");
    for workload in ["sparse", "mixed", "arrays"] {
        let report = profile(
            &support::corpus(workload, 1024, 32),
            ProfilerOptions::default(),
        );
        discovery.bench_function(workload, |b| {
            b.iter(|| black_box(report.discover(DiscoveryOptions::default()).unwrap()))
        });
    }
    discovery.finish();

    let corpus = support::corpus("homogeneous", 1024, 8);
    let source = serde_json::to_vec(&corpus).unwrap();
    c.bench_function("parse_and_profile/1024", |b| {
        b.iter(|| {
            let parsed: Vec<Value> = serde_json::from_slice(black_box(&source)).unwrap();
            black_box(profile(&parsed, ProfilerOptions::default()));
        })
    });
}

criterion_group! { name = benches; config = Criterion::default().sample_size(40).warm_up_time(Duration::from_secs(1)).measurement_time(Duration::from_secs(3)); targets = policies }
criterion_main!(benches);
