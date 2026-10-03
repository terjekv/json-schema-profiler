use std::{hint::black_box, time::Duration};

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use json_schema_profiler::{
    CompiledSchema, Document, DocumentId, EvaluationOptions, EvidenceLimits, InferencePolicy,
    OwnedDocument, Profiler, ProfilerOptions, SchemaOptions, Suggestion,
};

mod validation_support;

fn benchmarks(c: &mut Criterion) {
    let mut compilation = c.benchmark_group("compilation");
    for workload in ["structural", "references", "conditional", "decimal"] {
        let schema = validation_support::schema(workload);
        compilation.bench_function(workload, |b| {
            b.iter(|| {
                black_box(
                    CompiledSchema::new(black_box(&schema), SchemaOptions::default()).unwrap(),
                )
            })
        });
    }
    compilation.finish();
    let mut evaluation = c.benchmark_group("evaluation");
    for workload in [
        "structural",
        "invalid",
        "references",
        "conditional",
        "decimal",
    ] {
        let schema = validation_support::schema(workload);
        let compiled = CompiledSchema::new(&schema, SchemaOptions::default()).unwrap();
        let raw = jsonschema::draft202012::new(&schema).unwrap();
        for count in [64, 1024] {
            let corpus = validation_support::corpus(workload, count);
            for (name, options) in [
                ("diagnostics", EvaluationOptions::default()),
                (
                    "coverage",
                    EvaluationOptions::default().with_diagnostic_limits(0, 0),
                ),
            ] {
                evaluation.bench_with_input(
                    BenchmarkId::new(format!("{workload}/{name}"), count),
                    &corpus,
                    |b, corpus| {
                        b.iter(|| {
                            black_box(compiled.evaluate(
                                black_box(corpus).iter().map(Document::new),
                                options.clone(),
                            ))
                        })
                    },
                );
            }
            evaluation.bench_with_input(
                BenchmarkId::new(format!("{workload}/raw_flag"), count),
                &corpus,
                |b, corpus| {
                    b.iter(|| {
                        black_box(
                            black_box(corpus)
                                .iter()
                                .filter(|value| raw.is_valid(value))
                                .count(),
                        )
                    })
                },
            );
        }
    }
    evaluation.finish();
    let values = validation_support::corpus("structural", 1024);
    let lines: Vec<_> = values
        .iter()
        .map(|value| serde_json::to_string(value).unwrap())
        .collect();
    let compiled = CompiledSchema::new(
        &validation_support::schema("structural"),
        SchemaOptions::default(),
    )
    .unwrap();
    let mut parsing = c.benchmark_group("parse_and_replay");
    // Both include parsing identical source strings. The borrowed variant retains
    // the entire parsed corpus; the owned variant holds one parsed record at a time.
    parsing.bench_function("borrowed/1024", |b| {
        b.iter(|| {
            let records: Vec<serde_json::Value> = black_box(&lines)
                .iter()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            black_box(
                compiled
                    .verify(
                        records.iter().map(Document::new),
                        EvaluationOptions::default(),
                    )
                    .unwrap(),
            );
        })
    });
    parsing.bench_function("owned/1024", |b| {
        b.iter(|| {
            let records = black_box(&lines)
                .iter()
                .map(|line| serde_json::from_str(line).map(OwnedDocument::new));
            black_box(
                compiled
                    .try_verify_owned(records, EvaluationOptions::default())
                    .unwrap(),
            );
        })
    });
    parsing.finish();
    let mut evidence = c.benchmark_group("evidence");
    for count in [64, 1024] {
        let corpus = validation_support::corpus("structural", count);
        let ids: Vec<_> = (0..count)
            .map(|index| DocumentId::new(format!("doc-{index}")).unwrap())
            .collect();
        for mode in ["anonymous", "bounded_ids", "disabled_ids"] {
            evidence.bench_function(BenchmarkId::new(mode, count), |b| {
                b.iter(|| {
                    let options = if mode == "disabled_ids" {
                        ProfilerOptions::default().with_evidence_limits(EvidenceLimits::disabled())
                    } else {
                        ProfilerOptions::default()
                    };
                    let mut profiler = Profiler::new(options);
                    for (id, value) in ids.iter().zip(black_box(&corpus)) {
                        if mode == "anonymous" {
                            profiler.observe(value).unwrap();
                        } else {
                            profiler.observe_with_id(id, value).unwrap();
                        }
                    }
                    black_box(profiler.finish().unwrap());
                })
            });
        }
    }
    evidence.finish();
    let corpus = validation_support::corpus("structural", 1024);
    c.bench_function("profile_generate_verify/1024", |b| {
        b.iter(|| {
            let mut profiler = Profiler::default();
            for value in black_box(&corpus) {
                profiler.observe(value).unwrap();
            }
            let Suggestion::Candidate(candidate) = profiler
                .finish()
                .unwrap()
                .suggest(InferencePolicy::strict())
                .unwrap()
            else {
                panic!();
            };
            let compiled = candidate.compile(SchemaOptions::default()).unwrap();
            black_box(
                compiled
                    .verify(
                        corpus.iter().map(Document::new),
                        EvaluationOptions::default(),
                    )
                    .unwrap(),
            );
        })
    });
}

criterion_group! { name=benches; config=Criterion::default().sample_size(40).warm_up_time(Duration::from_secs(1)).measurement_time(Duration::from_secs(3)); targets=benchmarks }
criterion_main!(benches);
