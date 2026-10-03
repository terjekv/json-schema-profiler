use json_schema_profiler::{
    CompiledSchema, Document, EvaluationOptions, Finding, Frequency, InferenceError,
    InferencePolicy, Nullability, OutputLimit, OutputLimits, Presence, ProfilePath, Profiler,
    SchemaOptions, Suggestion, SuggestionOptions,
};
use rstest::rstest;
use serde_json::{Value, json};

fn report(documents: &[Value]) -> json_schema_profiler::Profile {
    let mut profiler = Profiler::default();
    for value in documents {
        profiler.observe(value).unwrap();
    }
    profiler.finish().unwrap()
}

#[rstest]
#[case(66, true)]
#[case(67, false)]
#[case(100, false)]
fn presence_threshold_uses_exact_applicable_parent_frequency(
    #[case] percent: u8,
    #[case] required: bool,
) {
    let values = [
        json!({"a":{"x":1}}),
        json!({"a":{"x":2}}),
        json!({"a":{}}),
        json!({}),
    ];
    let policy = InferencePolicy::balanced()
        .with_presence(Presence::AtLeast(Frequency::percent(percent).unwrap()));
    let Suggestion::Candidate(candidate) = report(&values).suggest(policy).unwrap() else {
        panic!()
    };
    let validator = jsonschema::draft202012::new(candidate.schema()).unwrap();
    assert!(validator.is_valid(&json!({"a":{"x":3}})));
    assert!(!validator.is_valid(&json!({"a":{"x":true}})));
    assert_eq!(validator.is_valid(&json!({"a":{}})), !required);
    assert_eq!(
        candidate.findings().iter().any(|f| matches!(f,
        Finding::RequiredFromFrequency { path, present: 2, applicable_parents: 3, .. }
            if path == &ProfilePath::root().property("a").property("x"))),
        required
    );
}

#[test]
fn frequency_based_requirement_does_not_claim_verified_corpus_coverage() {
    let values = [json!({"x":1}), json!({})];
    let Suggestion::Candidate(candidate) = report(&values)
        .suggest(
            InferencePolicy::strict()
                .with_presence(Presence::AtLeast(Frequency::percent(50).unwrap())),
        )
        .unwrap()
    else {
        panic!()
    };
    let evaluation = candidate
        .compile(SchemaOptions::default())
        .unwrap()
        .verify(
            values.iter().map(Document::new),
            EvaluationOptions::default(),
        )
        .unwrap_err();
    assert_eq!(evaluation.invalid(), 1);
}

#[test]
fn local_frequency_override_preserves_other_property_policies() {
    let values = [json!({"x":1,"y":1}), json!({})];
    let options = SuggestionOptions::new(InferencePolicy::expansive())
        .with_path_policy(
            ProfilePath::root().property("x"),
            InferencePolicy::balanced()
                .with_presence(Presence::AtLeast(Frequency::percent(50).unwrap())),
        )
        .unwrap();
    let Suggestion::Candidate(candidate) = report(&values).suggest_with(options).unwrap() else {
        panic!()
    };
    assert_eq!(candidate.schema()["required"], json!(["x"]));
}

#[test]
fn balanced_policy_allows_unions_and_extra_keys_but_requires_consistent_fields() {
    let Suggestion::Candidate(candidate) = report(&[json!({"x":1}), json!({"x":"a"})])
        .suggest(InferencePolicy::balanced())
        .unwrap()
    else {
        panic!()
    };
    let validator = jsonschema::draft202012::new(candidate.schema()).unwrap();
    assert!(validator.is_valid(&json!({"x":"b","new":true})));
    assert!(!validator.is_valid(&json!({})));
    assert!(!validator.is_valid(&json!({"x":false})));
}

#[test]
fn minimum_evidence_counts_documents_not_repeated_array_items() {
    let options = SuggestionOptions::new(InferencePolicy::balanced())
        .with_minimum_documents(2)
        .unwrap();
    let Suggestion::Insufficient(findings) = report(&[json!([1, 1, 1]), json!([])])
        .suggest_with(options)
        .unwrap()
    else {
        panic!()
    };
    assert!(findings.contains(&Finding::LowEvidence {
        path: ProfilePath::root().each_item(),
        documents: 1,
        minimum: 2,
    }));
}

#[test]
fn minimum_evidence_accepts_enough_distinct_records() {
    let options = SuggestionOptions::new(InferencePolicy::balanced())
        .with_minimum_documents(2)
        .unwrap();
    assert!(matches!(
        report(&[json!([1]), json!([2])])
            .suggest_with(options)
            .unwrap(),
        Suggestion::Candidate(_)
    ));
}

#[test]
fn zero_minimum_evidence_is_rejected() {
    assert!(
        SuggestionOptions::new(InferencePolicy::strict())
            .with_minimum_documents(0)
            .is_err()
    );
}

#[rstest]
#[case(InferencePolicy::strict())]
#[case(InferencePolicy::expansive())]
fn null_rejection_is_a_conflict_under_both_presets(#[case] policy: InferencePolicy) {
    let profile = report(&[json!({"x":null}), json!({"x":1})]);
    assert!(matches!(
        profile
            .suggest(policy.with_nullability(Nullability::Reject))
            .unwrap(),
        Suggestion::Blocked(_)
    ));
}

#[test]
fn local_expansion_preserves_strict_sibling_contracts() {
    let profile = report(&[json!({"a":1,"b":2}), json!({"a":"one","b":3})]);
    let options = SuggestionOptions::new(InferencePolicy::strict())
        .with_path_policy(
            ProfilePath::root().property("a"),
            InferencePolicy::expansive(),
        )
        .unwrap();
    let Suggestion::Candidate(candidate) = profile.suggest_with(options).unwrap() else {
        panic!("candidate expected");
    };
    let compiled = candidate.compile(SchemaOptions::default()).unwrap();
    let invalid = json!({"a":1,"b":"wrong"});
    assert_eq!(
        compiled
            .evaluate([Document::new(&invalid)], EvaluationOptions::default())
            .invalid(),
        1
    );
}

#[test]
fn local_presence_override_applies_to_property_itself() {
    let profile = report(&[json!({"a":1,"b":2})]);
    let options = SuggestionOptions::new(InferencePolicy::strict())
        .with_path_policy(
            ProfilePath::root().property("a"),
            InferencePolicy::strict().with_presence(Presence::Optional),
        )
        .unwrap();
    let Suggestion::Candidate(candidate) = profile.suggest_with(options).unwrap() else {
        panic!();
    };
    let value = json!({"b":2});
    assert!(
        CompiledSchema::new(candidate.schema(), SchemaOptions::default())
            .unwrap()
            .evaluate([Document::new(&value)], EvaluationOptions::default())
            .all_valid()
    );
}

#[test]
fn null_policy_inherits_through_array_subtrees() {
    let options = SuggestionOptions::new(InferencePolicy::expansive())
        .with_path_policy(
            ProfilePath::root().property("a"),
            InferencePolicy::expansive().with_nullability(Nullability::Reject),
        )
        .unwrap();
    assert!(matches!(
        report(&[json!({"a":[{"x":null}],"b":null})])
            .suggest_with(options)
            .unwrap(),
        Suggestion::Blocked(_)
    ));
}

#[rstest]
#[case(ProfilePath::root().property("a"))]
#[case(ProfilePath::root().property("a").property("b"))]
#[case(ProfilePath::root())]
fn overlapping_policies_are_rejected(#[case] path: ProfilePath) {
    let options = SuggestionOptions::new(InferencePolicy::strict())
        .with_path_policy(ProfilePath::root().property("a"), InferencePolicy::strict())
        .unwrap();
    assert!(matches!(
        options.with_path_policy(path, InferencePolicy::expansive()),
        Err(InferenceError::ConflictingPolicies(_))
    ));
}

#[test]
fn an_unprofiled_policy_path_is_not_silently_ignored() {
    let options = SuggestionOptions::new(InferencePolicy::strict())
        .with_path_policy(
            ProfilePath::root().property("unknown"),
            InferencePolicy::expansive(),
        )
        .unwrap();
    assert!(matches!(
        report(&[json!({})]).suggest_with(options),
        Err(InferenceError::UnknownPath(_))
    ));
}

#[rstest]
#[case(OutputLimits::new(1,10000,100).unwrap(), OutputLimit::SchemaBytes)]
#[case(OutputLimits::new(10000,1,100).unwrap(), OutputLimit::ReportBytes)]
#[case(OutputLimits::new(10000,10000,0).unwrap(), OutputLimit::Findings)]
fn candidate_output_limits_are_explicit_errors(
    #[case] limits: OutputLimits,
    #[case] expected: OutputLimit,
) {
    let options = SuggestionOptions::new(InferencePolicy::expansive()).with_output_limits(limits);
    assert!(
        matches!(report(&[json!({"x":1})]).suggest_with(options),Err(InferenceError::LimitExceeded(kind)) if kind == expected)
    );
}

#[test]
fn byte_budget_counts_json_escaping() {
    let profile = report(&[json!({"\"\\\n":1})]);
    let Suggestion::Candidate(candidate) = profile.suggest(InferencePolicy::strict()).unwrap()
    else {
        panic!();
    };
    let exact = serde_json::to_vec(candidate.schema()).unwrap().len();
    let limits = OutputLimits::new(exact - 1, 10000, 100).unwrap();
    assert!(matches!(
        profile.suggest_with(
            SuggestionOptions::new(InferencePolicy::strict()).with_output_limits(limits)
        ),
        Err(InferenceError::LimitExceeded(OutputLimit::SchemaBytes))
    ));
}

#[test]
fn null_rejection_also_constrains_items_when_only_empty_arrays_were_seen() {
    let profile = report(&[json!([])]);
    let Suggestion::Candidate(candidate) = profile
        .suggest(InferencePolicy::strict().with_nullability(Nullability::Reject))
        .unwrap()
    else {
        panic!();
    };
    let compiled = candidate.compile(SchemaOptions::default()).unwrap();
    let documents = [json!([1]), json!([null])];
    let result = compiled.evaluate(
        documents.iter().map(Document::new),
        EvaluationOptions::default(),
    );
    assert_eq!((result.valid(), result.invalid()), (1, 1));
}
