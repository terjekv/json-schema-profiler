use json_schema_profiler::{
    DiscoveryFinding, DiscoveryOptions, EnumCollection, EnumDetectionOptions, EnumDetectionStatus,
    Finding, Frequency, InferenceError, InferencePolicy, JsonKind, Limits, OutputLimit,
    OutputLimits, Profile, ProfileError, ProfilePath, Profiler, ProfilerOptions, Scope, Suggestion,
    SuggestionOptions,
};
use rstest::rstest;
use serde_json::{Value, json};

fn options(percent: u8) -> EnumDetectionOptions {
    EnumDetectionOptions::new(Frequency::percent(percent).unwrap())
}

fn profile(values: impl IntoIterator<Item = Value>, collection: EnumCollection) -> Profile {
    let mut profiler = Profiler::new(ProfilerOptions::default().with_enum_collection(collection));
    for value in values {
        profiler.observe(&value).unwrap();
    }
    profiler.finish().unwrap()
}

fn root_profile(documents: usize, distinct: usize) -> Profile {
    profile(
        (0..documents).map(|i| json!(format!("value-{}", i % distinct))),
        EnumCollection::default(),
    )
}

#[rstest]
#[case(100, 3, EnumDetectionStatus::Eligible)]
#[case(100, 4, EnumDetectionStatus::RatioExceeded)]
#[case(1_000, 3, EnumDetectionStatus::Eligible)]
#[case(1_000, 30, EnumDetectionStatus::Eligible)]
#[case(99, 3, EnumDetectionStatus::InsufficientDocuments)]
fn distinct_ratio_and_minimum_documents_are_independent(
    #[case] documents: usize,
    #[case] distinct: usize,
    #[case] expected: EnumDetectionStatus,
) {
    let report = root_profile(documents, distinct);
    let detection = report
        .field(&ProfilePath::root())
        .unwrap()
        .detect_enum(options(3));
    assert_eq!(detection.status(), expected);
    assert_eq!(detection.documents(), documents as u64);
    assert_eq!(detection.distinct_values(), Some(distinct));
    assert_eq!(
        detection.distinct_ratio(),
        Some(distinct as f64 / documents as f64)
    );
}

#[rstest]
#[case(99, EnumDetectionStatus::RatioExceeded)]
#[case(100, EnumDetectionStatus::Eligible)]
fn ratio_boundary_is_not_rounded(#[case] documents: usize, #[case] expected: EnumDetectionStatus) {
    let report = root_profile(documents, 3);
    let detection = report
        .field(&ProfilePath::root())
        .unwrap()
        .detect_enum(options(3).with_minimum_documents(1).unwrap());
    assert_eq!(detection.status(), expected);
}

#[test]
fn thresholds_can_be_retuned_without_reprofiling() {
    let report = root_profile(100, 4);
    let field = report.field(&ProfilePath::root()).unwrap();
    assert_eq!(
        field.detect_enum(options(3)).status(),
        EnumDetectionStatus::RatioExceeded
    );
    assert_eq!(
        field.detect_enum(options(5)).status(),
        EnumDetectionStatus::Eligible
    );
    let capped = options(5).with_max_distinct_values(Some(3)).unwrap();
    assert_eq!(
        field.detect_enum(capped).status(),
        EnumDetectionStatus::TooManyValues
    );
    assert_eq!(
        field
            .detect_enum(capped.with_max_distinct_values(None).unwrap())
            .status(),
        EnumDetectionStatus::Eligible
    );
}

#[test]
fn a_single_outlier_is_part_of_the_complete_set() {
    let report = profile(
        (0..99)
            .map(|i| json!(format!("value-{}", i % 3)))
            .chain([json!("outlier")]),
        EnumCollection::default(),
    );
    let field = report.field(&ProfilePath::root()).unwrap();
    assert_eq!(
        field.detect_enum(options(3)).status(),
        EnumDetectionStatus::RatioExceeded
    );
    let outlier = field
        .enum_evidence()
        .unwrap()
        .values()
        .iter()
        .find(|v| v.value() == "outlier")
        .unwrap();
    assert_eq!(outlier.occurrences(), 1);
    assert_eq!(outlier.documents(), 1);
}

#[test]
fn missing_null_and_other_kinds_do_not_inflate_the_denominator() {
    let values = (0..10)
        .map(|i| json!({"x": format!("v{}", i % 2)}))
        .chain((0..100).flat_map(|_| [json!({}), json!({"x": null}), json!({"x": 1})]));
    let report = profile(values, EnumCollection::default());
    let field = report.field(&ProfilePath::root().property("x")).unwrap();
    let detection = field.detect_enum(options(3).with_minimum_documents(1).unwrap());
    assert_eq!(detection.documents(), 10);
    assert_eq!(detection.distinct_values(), Some(2));
    assert_eq!(detection.status(), EnumDetectionStatus::RatioExceeded);
    assert_eq!(field.missing(), 100);
    assert_eq!(field.types().get(JsonKind::Null), 100);
}

#[test]
fn arrays_count_each_contributing_document_once() {
    let report = profile(
        [
            json!(["a", "a", "b", "b"]),
            json!(["a"]),
            json!([]),
            json!([null]),
        ],
        EnumCollection::default(),
    );
    let field = report.field(&ProfilePath::root().each_item()).unwrap();
    let detection = field.detect_enum(options(100).with_minimum_documents(2).unwrap());
    assert_eq!(detection.documents(), 2);
    assert_eq!(detection.distinct_values(), Some(2));
    assert_eq!(detection.status(), EnumDetectionStatus::Eligible);
    let counts: Vec<_> = field
        .enum_evidence()
        .unwrap()
        .values()
        .iter()
        .map(|entry| (entry.value(), entry.occurrences(), entry.documents()))
        .collect();
    assert_eq!(counts, [("a", 3, 2), ("b", 2, 1)]);
    assert_eq!(
        field
            .detect_enum(options(100).with_minimum_documents(3).unwrap())
            .status(),
        EnumDetectionStatus::InsufficientDocuments
    );
}

#[test]
fn multiple_array_values_can_have_a_ratio_above_one() {
    let report = profile([json!(["a", "b"])], EnumCollection::default());
    let detection = report
        .field(&ProfilePath::root().each_item())
        .unwrap()
        .detect_enum(options(100).with_minimum_documents(1).unwrap());
    assert_eq!(detection.distinct_ratio(), Some(2.0));
    assert_eq!(detection.status(), EnumDetectionStatus::RatioExceeded);
}

#[test]
fn collection_scope_preserves_literal_escaped_names() {
    let path = ProfilePath::root().property("/~*");
    let report = profile(
        [json!({"/~*":"kept", "other":"secret"})],
        EnumCollection::default().with_scope(Scope::selected([path.clone()]).unwrap()),
    );
    assert_eq!(
        report
            .field(&path)
            .unwrap()
            .enum_evidence()
            .unwrap()
            .values()[0]
            .value(),
        "kept"
    );
    assert_eq!(
        report
            .field(&ProfilePath::root().property("other"))
            .unwrap()
            .detect_enum(options(100))
            .status(),
        EnumDetectionStatus::NotCollected
    );
    assert!(!serde_json::to_string(&report).unwrap().contains("secret"));
}

#[test]
fn enum_collection_does_not_expand_the_profiling_scope() {
    let path = ProfilePath::root().property("selected");
    let mut profiler = Profiler::new(
        ProfilerOptions::default()
            .with_scope(Scope::selected([path.clone()]).unwrap())
            .with_enum_collection(EnumCollection::default()),
    );
    profiler
        .observe(&json!({"selected": "kept", "ignored":"secret"}))
        .unwrap();
    let report = profiler.finish().unwrap();
    assert!(report.field(&path).unwrap().enum_evidence().is_some());
    assert!(!serde_json::to_string(&report).unwrap().contains("secret"));
}

#[test]
fn default_profiles_retain_no_values_or_new_serialized_field() {
    let mut profiler = Profiler::default();
    profiler.observe(&json!("secret")).unwrap();
    let report = profiler.finish().unwrap();
    let field = report.field(&ProfilePath::root()).unwrap();
    assert_eq!(
        field.detect_enum(options(3)).status(),
        EnumDetectionStatus::NotCollected
    );
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("secret"));
    assert!(!serialized.contains("enum_evidence"));
}

#[test]
fn per_path_overflow_discards_partial_values_but_keeps_other_paths() {
    let report = profile(
        (0..100).map(|i| json!({"wide":format!("v{}", i % 3), "stable":"a"})),
        EnumCollection::default()
            .with_limits(2, 100, 1_000)
            .unwrap(),
    );
    let wide = report.field(&ProfilePath::root().property("wide")).unwrap();
    assert_eq!(
        wide.detect_enum(options(100)).status(),
        EnumDetectionStatus::Incomplete
    );
    assert_eq!(wide.detect_enum(options(100)).distinct_values(), None);
    assert!(wide.enum_evidence().unwrap().values().is_empty());
    assert_eq!(wide.present(), 100);
    assert_eq!(wide.evidence().documents(), 100);
    let stable = report
        .field(&ProfilePath::root().property("stable"))
        .unwrap();
    assert_eq!(
        stable.detect_enum(options(3)).status(),
        EnumDetectionStatus::Eligible
    );
}

#[rstest]
#[case(1, 1_000)]
#[case(100, 1)]
fn global_overflow_invalidates_all_enum_evidence(#[case] values: usize, #[case] bytes: usize) {
    let report = profile(
        [json!({"a":"x", "b":"y"}), json!({"a":"x", "c":"z"})],
        EnumCollection::default()
            .with_limits(100, values, bytes)
            .unwrap(),
    );
    for name in ["a", "b", "c"] {
        let field = report.field(&ProfilePath::root().property(name)).unwrap();
        assert_eq!(
            field.detect_enum(options(100)).status(),
            EnumDetectionStatus::Incomplete
        );
        assert!(field.enum_evidence().unwrap().values().is_empty());
    }
}

#[test]
fn evidence_preserves_exact_strings_and_is_order_independent() {
    let values = [
        json!(""),
        json!("a"),
        json!("A"),
        json!("a "),
        json!("é"),
        json!("e\u{301}"),
        json!("a"),
    ];
    let forward = profile(values.clone(), EnumCollection::default());
    let reverse = profile(values.into_iter().rev(), EnumCollection::default());
    assert_eq!(forward, reverse);
    assert_eq!(
        forward
            .field(&ProfilePath::root())
            .unwrap()
            .enum_evidence()
            .unwrap()
            .distinct_values(),
        Some(6)
    );
}

#[test]
fn no_string_observations_are_explicit() {
    let report = profile([json!(null), json!(1)], EnumCollection::default());
    let detection = report
        .field(&ProfilePath::root())
        .unwrap()
        .detect_enum(options(3));
    assert_eq!(detection.status(), EnumDetectionStatus::NoStrings);
    assert_eq!(detection.distinct_ratio(), None);
}

#[test]
fn discovery_reports_eligibility_without_exposing_values() {
    let report = profile((0..100).map(|_| json!("secret")), EnumCollection::default());
    let findings = report
        .discover(DiscoveryOptions::default().with_enum_detection(options(3)))
        .unwrap();
    assert!(findings.findings().iter().any(|finding| matches!(finding,
        DiscoveryFinding::StringEnum { detection, .. } if detection.status() == EnumDetectionStatus::Eligible)));
    assert!(!serde_json::to_string(&findings).unwrap().contains("secret"));
}

#[test]
fn collecting_and_discovering_values_does_not_enable_constraints() {
    let report = root_profile(100, 3);
    report
        .discover(DiscoveryOptions::default().with_enum_detection(options(3)))
        .unwrap();
    let Suggestion::Candidate(candidate) = report.suggest(InferencePolicy::strict()).unwrap()
    else {
        panic!()
    };
    assert!(
        jsonschema::draft202012::new(candidate.schema())
            .unwrap()
            .is_valid(&json!("unseen"))
    );
    assert!(candidate.schema().get("enum").is_none());
}

#[rstest]
#[case(false)]
#[case(true)]
fn generated_enums_accept_every_observed_string_and_reject_new_ones(#[case] mixed: bool) {
    let mut values: Vec<_> = (0..100)
        .map(|i| json!({"x": format!("v{}", i % 3)}))
        .collect();
    if mixed {
        values.extend([json!({"x":null}), json!({"x":42}), json!({})]);
    }
    let report = profile(values.clone(), EnumCollection::default());
    let policy = InferencePolicy::balanced().with_enum_detection(options(3));
    let Suggestion::Candidate(candidate) = report.suggest(policy).unwrap() else {
        panic!()
    };
    let validator = jsonschema::draft202012::new(candidate.schema()).unwrap();
    for value in &values {
        assert!(validator.is_valid(value));
    }
    assert!(!validator.is_valid(&json!({"x":"new"})));
    assert!(!validator.is_valid(&json!({"x":false})));
    assert_eq!(validator.is_valid(&json!({"x":null})), mixed);
    assert_eq!(validator.is_valid(&json!({})), mixed);
}

#[test]
fn enum_policy_can_be_enabled_per_path() {
    let report = profile(
        (0..100).map(|_| json!({"a":"x", "b":"y"})),
        EnumCollection::default(),
    );
    let options = SuggestionOptions::new(InferencePolicy::strict())
        .with_path_policy(
            ProfilePath::root().property("a"),
            InferencePolicy::strict().with_enum_detection(options(3)),
        )
        .unwrap();
    let Suggestion::Candidate(candidate) = report.suggest_with(options).unwrap() else {
        panic!()
    };
    let validator = jsonschema::draft202012::new(candidate.schema()).unwrap();
    assert!(validator.is_valid(&json!({"a":"x", "b":"new"})));
    assert!(!validator.is_valid(&json!({"a":"new", "b":"y"})));
}

#[test]
fn incomplete_values_cannot_become_a_truncated_enum() {
    let report = profile(
        (0..100).map(|i| json!(format!("v{}", i % 3))),
        EnumCollection::default()
            .with_limits(2, 100, 1_000)
            .unwrap(),
    );
    let Suggestion::Candidate(candidate) = report
        .suggest(InferencePolicy::strict().with_enum_detection(options(3)))
        .unwrap()
    else {
        panic!()
    };
    assert!(candidate.schema().get("enum").is_none());
    assert!(candidate.findings().iter().any(|finding| matches!(finding,
        Finding::StringEnum { detection, .. } if detection.status() == EnumDetectionStatus::Incomplete)));
    let validator = jsonschema::draft202012::new(candidate.schema()).unwrap();
    for value in ["v0", "v1", "v2"] {
        assert!(validator.is_valid(&json!(value)));
    }
}

#[test]
fn retained_values_are_subject_to_profile_output_limits() {
    let mut profiler = Profiler::new(
        ProfilerOptions::default()
            .with_enum_collection(EnumCollection::default())
            .with_limits(Limits::builder().report_bytes(1_000).build().unwrap()),
    );
    profiler.observe(&json!("x".repeat(2_000))).unwrap();
    assert!(matches!(
        profiler.finish(),
        Err(ProfileError::LimitExceeded(_))
    ));
}

#[test]
fn generated_enum_values_are_subject_to_schema_output_limits() {
    let report = profile(
        (0..100).map(|_| json!("x".repeat(2_000))),
        EnumCollection::default(),
    );
    let options = SuggestionOptions::new(InferencePolicy::strict().with_enum_detection(options(3)))
        .with_output_limits(OutputLimits::new(1_000, 10_000, 100).unwrap());
    assert!(matches!(
        report.suggest_with(options),
        Err(InferenceError::LimitExceeded(OutputLimit::SchemaBytes))
    ));
}

#[test]
fn zero_minimum_enum_documents_is_rejected() {
    assert!(options(3).with_minimum_documents(0).is_err());
}

#[test]
fn zero_distinct_value_ceiling_is_rejected() {
    assert!(options(3).with_max_distinct_values(Some(0)).is_err());
}

#[rstest]
#[case(0, 10, 10)]
#[case(10, 0, 10)]
#[case(10, 10, 0)]
fn zero_collection_limits_are_rejected(
    #[case] per_path: usize,
    #[case] total: usize,
    #[case] bytes: usize,
) {
    assert!(
        EnumCollection::default()
            .with_limits(per_path, total, bytes)
            .is_err()
    );
}

#[rstest]
#[case("a", 1, EnumDetectionStatus::Eligible)]
#[case("é", 1, EnumDetectionStatus::Incomplete)]
#[case("é", 2, EnumDetectionStatus::Eligible)]
fn byte_budget_counts_utf8_payload(
    #[case] value: &str,
    #[case] bytes: usize,
    #[case] expected: EnumDetectionStatus,
) {
    let report = profile(
        (0..100).map(|_| json!(value)),
        EnumCollection::default()
            .with_limits(10, 10, bytes)
            .unwrap(),
    );
    assert_eq!(
        report
            .field(&ProfilePath::root())
            .unwrap()
            .detect_enum(options(3))
            .status(),
        expected
    );
}

#[test]
fn empty_strings_consume_a_value_entry() {
    let report = profile(
        [json!(""), json!("a")],
        EnumCollection::default().with_limits(10, 1, 10).unwrap(),
    );
    assert_eq!(
        report
            .field(&ProfilePath::root())
            .unwrap()
            .detect_enum(options(100))
            .status(),
        EnumDetectionStatus::Incomplete
    );
}

#[test]
fn disabled_collection_is_reported_when_generation_is_requested() {
    let mut profiler = Profiler::default();
    for _ in 0..100 {
        profiler.observe(&json!("a")).unwrap();
    }
    let Suggestion::Candidate(candidate) = profiler
        .finish()
        .unwrap()
        .suggest(InferencePolicy::strict().with_enum_detection(options(3)))
        .unwrap()
    else {
        panic!()
    };
    assert!(candidate.schema().get("enum").is_none());
    assert!(candidate.findings().iter().any(|finding| matches!(finding,
        Finding::StringEnum { detection, .. } if detection.status() == EnumDetectionStatus::NotCollected)));
}

#[test]
fn null_rejection_remains_a_policy_conflict_with_enum_inference() {
    let report = profile(
        (0..100).map(|_| json!("a")).chain([json!(null)]),
        EnumCollection::default(),
    );
    let policy = InferencePolicy::strict()
        .with_enum_detection(options(3))
        .with_nullability(json_schema_profiler::Nullability::Reject);
    assert!(matches!(
        report.suggest(policy).unwrap(),
        Suggestion::Blocked(_)
    ));
}

#[test]
fn enum_evidence_does_not_mask_incomplete_ingestion() {
    let mut profiler = Profiler::new(
        ProfilerOptions::default()
            .with_enum_collection(EnumCollection::default())
            .with_limits(Limits::builder().documents(1).build().unwrap()),
    );
    profiler.observe(&json!("a")).unwrap();
    assert!(profiler.observe(&json!("b")).is_err());
    assert_eq!(profiler.finish().unwrap_err(), ProfileError::Incomplete);
}
