use json_schema_profiler::{
    DiscoveryFinding, DiscoveryOptions, EvidenceLimits, Frequency, InferenceError, JsonKind,
    OutputLimit, Profile, ProfilePath, Profiler, ProfilerOptions, Scope,
};
use rstest::rstest;
use serde_json::{Value, json};

fn profile(values: &[Value]) -> Profile {
    let mut profiler = Profiler::default();
    for value in values {
        profiler.observe(value).unwrap();
    }
    profiler.finish().unwrap()
}

#[test]
fn discovery_separates_null_missing_mixed_and_sparse_fields() {
    let profile = profile(&[
        json!({"x": 1, "owner": null}),
        json!({"x": "private"}),
        json!({}),
    ]);
    let result = profile.discover(DiscoveryOptions::default()).unwrap();
    assert!(result.findings().iter().any(|f| matches!(f, DiscoveryFinding::MixedTypes { path, .. } if path == &ProfilePath::root().property("x"))));
    assert!(
        result
            .findings()
            .iter()
            .any(|f| matches!(f, DiscoveryFinding::NullOnly { occurrences: 1, .. }))
    );
    assert!(result.findings().iter().any(|f| matches!(
        f,
        DiscoveryFinding::SparseProperty {
            present: 1,
            applicable_parents: 3,
            ..
        }
    )));
    assert!(!serde_json::to_string(&result).unwrap().contains("private"));
}

#[test]
fn rare_type_uses_documents_instead_of_repeated_array_items() {
    let result = profile(&[json!([1, 1, 1, 1, 1, 1, 1, 1, 1, 1, "private"]), json!([2])])
        .discover(
            DiscoveryOptions::default().with_rare_type_threshold(Frequency::percent(50).unwrap()),
        )
        .unwrap();
    let rare: Vec<_> = result
        .findings()
        .iter()
        .filter(|f| matches!(f, DiscoveryFinding::RareType { .. }))
        .collect();
    assert_eq!(
        rare,
        vec![&DiscoveryFinding::RareType {
            path: ProfilePath::root().each_item(),
            value_type: JsonKind::String,
            documents: 1,
            contributing_documents: 2,
        }]
    );
}

#[test]
fn sparse_property_denominator_counts_only_applicable_objects() {
    let result = profile(&[json!({"a": {"b": 1}}), json!({"a": {}}), json!({})])
        .discover(DiscoveryOptions::default())
        .unwrap();
    assert!(
        result
            .findings()
            .contains(&DiscoveryFinding::SparseProperty {
                path: ProfilePath::root().property("a").property("b"),
                present: 1,
                applicable_parents: 2,
            })
    );
}

#[test]
fn empty_arrays_and_absent_selections_have_explicit_discovery_findings() {
    let mut profiler = Profiler::new(
        ProfilerOptions::default().with_scope(
            Scope::selected([
                ProfilePath::root().property("absent"),
                ProfilePath::root().property("empty"),
            ])
            .unwrap(),
        ),
    );
    profiler.observe(&json!({"empty": []})).unwrap();
    let result = profiler
        .finish()
        .unwrap()
        .discover(DiscoveryOptions::default())
        .unwrap();
    for path in [
        ProfilePath::root().property("absent"),
        ProfilePath::root().property("empty").each_item(),
    ] {
        assert!(
            result
                .findings()
                .contains(&DiscoveryFinding::Unobserved { path })
        );
    }
}

#[test]
fn discovery_is_independent_of_witness_retention_and_input_order() {
    let values = [json!({"/~*":null}), json!({"/~*":1}), json!({})];
    let expected = profile(&values)
        .discover(DiscoveryOptions::default())
        .unwrap();
    let mut profiler =
        Profiler::new(ProfilerOptions::default().with_evidence_limits(EvidenceLimits::disabled()));
    for value in values.iter().rev() {
        profiler.observe(value).unwrap();
    }
    let actual = profiler
        .finish()
        .unwrap()
        .discover(DiscoveryOptions::default())
        .unwrap();
    assert_eq!(
        serde_json::to_value(expected).unwrap(),
        serde_json::to_value(actual).unwrap()
    );
}

#[rstest]
#[case(0, 1_000_000, OutputLimit::Findings)]
#[case(10, 1, OutputLimit::ReportBytes)]
fn discovery_limits_never_return_partial_success(
    #[case] findings: usize,
    #[case] bytes: usize,
    #[case] expected: OutputLimit,
) {
    let options = DiscoveryOptions::default()
        .with_output_limits(findings, bytes)
        .unwrap();
    assert!(
        matches!(profile(&[json!(null)]).discover(options), Err(InferenceError::LimitExceeded(limit)) if limit == expected)
    );
}

#[rstest]
#[case(0)]
#[case(101)]
#[case(255)]
fn frequency_rejects_invalid_percentages(#[case] value: u8) {
    assert!(Frequency::percent(value).is_err());
}

#[test]
fn integer_and_fractional_numbers_are_not_unrelated_mixed_types() {
    let report = profile(&[json!(1), json!(1.5)])
        .discover(DiscoveryOptions::default())
        .unwrap();
    assert!(
        !report
            .findings()
            .iter()
            .any(|f| matches!(f, DiscoveryFinding::MixedTypes { .. }))
    );
}
