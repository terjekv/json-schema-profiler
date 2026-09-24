use json_schema_profiler::{
    DocumentId, EvidenceLimits, JsonKind, LimitKind, Limits, ProfileError, ProfilePath, Profiler,
    ProfilerOptions,
};
use rstest::rstest;
use serde_json::json;

#[test]
fn document_counts_deduplicate_repeated_array_occurrences() {
    let mut profiler = Profiler::default();
    profiler
        .observe(&json!([{"x":1},{"x":2},{"x":null},{"x":"s"}]))
        .unwrap();
    profiler.observe(&json!([{"x":3}])).unwrap();
    let profile = profiler.finish().unwrap();
    let field = profile
        .field(&ProfilePath::root().each_item().property("x"))
        .unwrap();
    assert_eq!(
        (
            field.present(),
            field.evidence().documents(),
            field.evidence().types().get(JsonKind::Integer)
        ),
        (5, 2, 2)
    );
}

#[test]
fn duplicate_labels_do_not_merge_input_records() {
    let id = DocumentId::new("same-label").unwrap();
    let mut profiler = Profiler::default();
    for _ in 0..2 {
        profiler.observe_with_id(&id, &json!(1)).unwrap();
    }
    assert_eq!(
        profiler
            .finish()
            .unwrap()
            .field(&ProfilePath::root())
            .unwrap()
            .evidence()
            .documents(),
        2
    );
}

#[test]
fn witnesses_identify_each_observed_kind_without_retaining_values() {
    let mut profiler = Profiler::default();
    profiler
        .observe_with_id(&DocumentId::new("first").unwrap(), &json!({"x":42}))
        .unwrap();
    profiler
        .observe_with_id(
            &DocumentId::new("second").unwrap(),
            &json!({"x":"private-value"}),
        )
        .unwrap();
    let profile = profiler.finish().unwrap();
    let witnesses = profile
        .field(&ProfilePath::root().property("x"))
        .unwrap()
        .evidence()
        .witnesses();
    assert_eq!(
        witnesses
            .iter()
            .map(|w| (w.kind(), w.document_id().as_str()))
            .collect::<Vec<_>>(),
        [(JsonKind::Integer, "first"), (JsonKind::String, "second")]
    );
    assert!(
        !serde_json::to_string(&profile)
            .unwrap()
            .contains("private-value")
    );
}

#[rstest]
#[case(EvidenceLimits::new(1, 100, 1000), 1)]
#[case(EvidenceLimits::new(100, 1, 1000), 1)]
#[case(EvidenceLimits::new(100, 100, 3), 1)]
#[case(EvidenceLimits::disabled(), 0)]
fn witness_truncation_does_not_truncate_counts(
    #[case] limits: EvidenceLimits,
    #[case] retained: usize,
) {
    let mut profiler = Profiler::new(ProfilerOptions::default().with_evidence_limits(limits));
    for id in ["one", "two", "three"] {
        profiler
            .observe_with_id(&DocumentId::new(id).unwrap(), &json!(42))
            .unwrap();
    }
    let profile = profiler.finish().unwrap();
    let evidence = profile.field(&ProfilePath::root()).unwrap().evidence();
    assert_eq!(
        (
            evidence.documents(),
            evidence.witnesses().len(),
            evidence.witnesses_truncated()
        ),
        (3, retained, true)
    );
}

#[test]
fn witness_budget_is_shared_across_paths() {
    let mut profiler = Profiler::new(
        ProfilerOptions::default().with_evidence_limits(EvidenceLimits::new(10, 3, 100)),
    );
    profiler
        .observe_with_id(&DocumentId::new("id").unwrap(), &json!({"a":1,"b":2,"c":3}))
        .unwrap();
    let profile = profiler.finish().unwrap();
    assert_eq!(
        profile
            .fields()
            .map(|field| field.evidence().witnesses().len())
            .sum::<usize>(),
        3
    );
}

#[test]
fn an_empty_array_has_no_contributing_item_documents() {
    let mut profiler = Profiler::default();
    profiler.observe(&json!([])).unwrap();
    assert_eq!(
        profiler
            .finish()
            .unwrap()
            .field(&ProfilePath::root().each_item())
            .unwrap()
            .evidence()
            .documents(),
        0
    );
}

#[test]
fn oversized_profile_is_not_returned_as_complete() {
    let mut profiler = Profiler::new(
        ProfilerOptions::default().with_limits(Limits::builder().report_bytes(10).build().unwrap()),
    );
    profiler.observe(&json!({"x":1})).unwrap();
    assert!(matches!(
        profiler.finish(),
        Err(ProfileError::LimitExceeded(LimitKind::ReportBytes))
    ));
}

#[rstest]
#[case("")]
#[case(&"x".repeat(257))]
fn invalid_document_ids_are_rejected(#[case] value: &str) {
    assert!(DocumentId::new(value).is_err());
}
