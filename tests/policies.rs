use json_schema_profiler::{
    CompiledSchema, Document, EvaluationOptions, InferenceError, InferencePolicy, Nullability,
    OutputLimit, OutputLimits, Presence, ProfilePath, Profiler, SchemaOptions, Suggestion,
    SuggestionOptions,
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
