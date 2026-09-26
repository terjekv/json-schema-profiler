use json_schema_profiler::{
    InferencePolicy, JsonKind, LimitKind, Limits, Presence, Profile, ProfileError, ProfilePath,
    Profiler, ProfilerOptions, Scope, Suggestion,
};
use rstest::rstest;
use serde_json::{Value, json};

fn profile(documents: &[Value]) -> Profile {
    let mut profiler = Profiler::default();
    for document in documents {
        profiler.observe(document).unwrap();
    }
    profiler.finish().unwrap()
}

fn schema(profile: &Profile, policy: InferencePolicy) -> Value {
    let Suggestion::Candidate(candidate) = profile.suggest(policy).unwrap() else {
        panic!("expected candidate");
    };
    candidate.schema().clone()
}

#[rstest]
#[case(65)]
#[case(1025)]
fn wide_object_occurrences_and_document_contributions_remain_distinct(#[case] width: usize) {
    let object = |value: Value| {
        Value::Object(
            (0..width)
                .map(|index| (format!("field/~{index}"), value.clone()))
                .collect(),
        )
    };
    let documents = [
        json!([object(json!(1)), object(json!(1)), {}]),
        json!([object(Value::Null)]),
    ];
    let report = profile(&documents);
    for index in 0..width {
        let field = report
            .field(
                &ProfilePath::root()
                    .each_item()
                    .property(format!("field/~{index}")),
            )
            .unwrap();
        assert_eq!(
            (field.applicable_parents(), field.present(), field.missing()),
            (4, 3, 1)
        );
        assert_eq!(field.types().get(JsonKind::Integer), 2);
        assert_eq!(field.types().get(JsonKind::Null), 1);
        assert_eq!(field.evidence().documents(), 2);
        assert_eq!(field.evidence().types().get(JsonKind::Integer), 1);
        assert_eq!(field.evidence().types().get(JsonKind::Null), 1);
    }
}

#[rstest]
#[case(65)]
#[case(1025)]
fn wide_object_candidate_is_deterministic_and_rejects_unobserved_types(#[case] width: usize) {
    let object = Value::Object(
        (0..width)
            .map(|index| (format!("field_{index}"), json!(index)))
            .collect(),
    );
    let documents = [object, json!({})];
    let candidate = schema(&profile(&documents), InferencePolicy::strict());
    let reversed = [documents[1].clone(), documents[0].clone()];
    assert_eq!(
        candidate,
        schema(&profile(&reversed), InferencePolicy::strict())
    );
    let validator = jsonschema::draft202012::new(&candidate).unwrap();
    assert!(documents.iter().all(|value| validator.is_valid(value)));
    assert!(!validator.is_valid(&json!({"field_0":"unexpected"})));
    assert!(!validator.is_valid(&json!({"additional":1})));
}

#[rstest]
#[case("1", JsonKind::Integer)]
#[case("1.0", JsonKind::Integer)]
#[case("1e0", JsonKind::Integer)]
#[case("10e-1", JsonKind::Integer)]
#[case("1.100e1", JsonKind::Integer)]
#[case("-0.000e-100000", JsonKind::Integer)]
#[case(
    "12345678901234567890123456789012345678901234567890",
    JsonKind::Integer
)]
#[case("1e100000000000000000000000", JsonKind::Integer)]
#[case("1e-100000000000000000000000", JsonKind::FractionalNumber)]
#[case("1.000000000000000000000000000000000001", JsonKind::FractionalNumber)]
#[case("10.01", JsonKind::FractionalNumber)]
#[case("0.1e0", JsonKind::FractionalNumber)]
fn exact_number_kind(#[case] token: &str, #[case] expected: JsonKind) {
    let report = profile(&[serde_json::from_str(token).unwrap()]);
    assert_eq!(
        report
            .field(&ProfilePath::root())
            .unwrap()
            .types()
            .get(expected),
        1
    );
}

#[rstest]
#[case(&[r#"{"x":null}"#, r#"{"x":null}"#, "{}", r#"{"x":1}"#], 4, 3, 2)]
#[case(&["{}", r#"{"x":1}"#], 2, 1, 0)]
#[case(&[r#"{"x":null}"#, "{}"], 2, 1, 1)]
fn exact_presence_counts(
    #[case] documents: &[&str],
    #[case] parents: u64,
    #[case] present: u64,
    #[case] nulls: u64,
) {
    let report = profile(
        &documents
            .iter()
            .map(|value| serde_json::from_str(value).unwrap())
            .collect::<Vec<_>>(),
    );
    let field = report.field(&ProfilePath::root().property("x")).unwrap();
    assert_eq!(
        (
            field.applicable_parents(),
            field.present(),
            field.types().get(JsonKind::Null)
        ),
        (parents, present, nulls)
    );
}

#[test]
fn nested_presence_uses_object_parent_denominator() {
    let report = profile(&[json!({"a":{"b":1}}), json!({}), json!({"a":null})]);
    let field = report
        .field(&ProfilePath::root().property("a").property("b"))
        .unwrap();
    assert_eq!((field.applicable_parents(), field.present()), (1, 1));
}

#[test]
fn array_counts_use_element_denominators() {
    let report = profile(&[json!([{"x":1}, {}, null, {"x":null}]), json!([{"x":2}])]);
    let field = report
        .field(&ProfilePath::root().each_item().property("x"))
        .unwrap();
    assert_eq!(
        (
            field.applicable_parents(),
            field.present(),
            field.types().get(JsonKind::Null)
        ),
        (4, 3, 1)
    );
}

#[test]
fn repeated_mixed_root_nulls_have_exact_counts() {
    let report = profile(&[Value::Null, json!(1), Value::Null, json!({}), Value::Null]);
    assert_eq!(
        report
            .field(&ProfilePath::root())
            .unwrap()
            .types()
            .get(JsonKind::Null),
        3
    );
}

#[rstest]
#[case(vec![json!({"a":null}),json!({"a":1}),json!({}),json!({"a":"x"})])]
#[case(vec![json!([1,null,{}]),json!(["s",{"x":1}]),json!([])])]
#[case(vec![Value::Null, json!({"a":1}), json!(true), Value::Null])]
fn document_order_does_not_change_report(#[case] mut documents: Vec<Value>) {
    let forward = profile(&documents);
    documents.reverse();
    assert_eq!(forward, profile(&documents));
}

#[rstest]
#[case(vec![json!({"a":{"b":1}}), json!({})], InferencePolicy::strict())]
#[case(vec![json!({"x":null}),json!({"x":1})], InferencePolicy::strict())]
#[case(vec![json!({"x":1}),json!({"x":1.5})], InferencePolicy::strict())]
#[case(vec![json!([1,"x",null,{},[]]),json!([])], InferencePolicy::expansive())]
#[case(vec![json!({"x":1}),json!({"x":"1"})], InferencePolicy::expansive())]
#[case(vec![Value::Null,json!({}),json!([]),json!(true),json!("x"),json!(3)], InferencePolicy::expansive())]
fn generated_schemas_cover_corpus(#[case] documents: Vec<Value>, #[case] policy: InferencePolicy) {
    let candidate = schema(&profile(&documents), policy);
    assert!(jsonschema::draft202012::meta::is_valid(&candidate));
    let validator = jsonschema::draft202012::new(&candidate).unwrap();
    assert!(
        documents
            .iter()
            .all(|document| validator.is_valid(document))
    );
}

#[rstest]
#[case(json!({"x":"wrong"}))]
#[case(json!({}))]
#[case(json!({"x":1,"unknown":true}))]
fn strict_schema_rejects_violations(#[case] invalid: Value) {
    let candidate = schema(&profile(&[json!({"x":1})]), InferencePolicy::strict());
    assert!(
        !jsonschema::draft202012::new(&candidate)
            .unwrap()
            .is_valid(&invalid)
    );
}

#[test]
fn strict_mixed_types_are_blocked() {
    assert!(matches!(
        profile(&[json!({"x":1}), json!({"x":"1"})])
            .suggest(InferencePolicy::strict())
            .unwrap(),
        Suggestion::Blocked(_)
    ));
}

#[test]
fn null_only_field_is_constrained_to_null() {
    let candidate = schema(&profile(&[json!({"x":null})]), InferencePolicy::strict());
    assert_eq!(candidate["properties"]["x"]["type"], "null");
}

#[rstest]
#[case(json!({"hardware":{"cores":8},"outside":[null,42]}), true)]
#[case(json!({"hardware":{"cores":8,"vendor":"arbitrary"}}), true)]
#[case(json!({"hardware":{"cores":"eight"}}), false)]
fn selecting_a_leaf_leaves_siblings_free(#[case] document: Value, #[case] valid: bool) {
    let scope =
        Scope::selected([ProfilePath::root().property("hardware").property("cores")]).unwrap();
    let mut profiler = Profiler::new(ProfilerOptions::default().with_scope(scope));
    profiler
        .observe(&json!({"hardware":{"cores":4,"vendor":"ignored"}}))
        .unwrap();
    let candidate = schema(&profiler.finish().unwrap(), InferencePolicy::strict());
    assert_eq!(
        jsonschema::draft202012::new(&candidate)
            .unwrap()
            .is_valid(&document),
        valid
    );
}

#[test]
fn selecting_array_items_profiles_only_selected_leaf() {
    let path = ProfilePath::root()
        .property("interfaces")
        .each_item()
        .property("address");
    let mut profiler = Profiler::new(
        ProfilerOptions::default().with_scope(Scope::selected([path.clone()]).unwrap()),
    );
    profiler
        .observe(&json!({"interfaces":[{"address":"a","ignored":42},{"address":"b"}]}))
        .unwrap();
    assert_eq!(
        profiler.finish().unwrap().field(&path).unwrap().present(),
        2
    );
}

#[test]
fn optional_presence_can_override_strict_preset() {
    let candidate = schema(
        &profile(&[json!({"x":1})]),
        InferencePolicy::strict().with_presence(Presence::Optional),
    );
    assert!(
        jsonschema::draft202012::new(&candidate)
            .unwrap()
            .is_valid(&json!({}))
    );
}

#[rstest]
#[case(ProfilePath::root().property("absent"), json!({}))]
#[case(ProfilePath::root().property("a").each_item(), json!({"a":[]}))]
fn never_observed_selection_is_insufficient(#[case] path: ProfilePath, #[case] document: Value) {
    let mut profiler =
        Profiler::new(ProfilerOptions::default().with_scope(Scope::selected([path]).unwrap()));
    profiler.observe(&document).unwrap();
    assert!(matches!(
        profiler
            .finish()
            .unwrap()
            .suggest(InferencePolicy::expansive())
            .unwrap(),
        Suggestion::Insufficient(_)
    ));
}

#[test]
fn wrong_ancestor_type_is_an_explicit_error() {
    let mut profiler =
        Profiler::new(ProfilerOptions::default().with_scope(
            Scope::selected([ProfilePath::root().property("a").property("b")]).unwrap(),
        ));
    assert!(matches!(
        profiler.observe(&json!({"a":42})),
        Err(ProfileError::ScopeMismatch(_))
    ));
}

#[rstest]
#[case(Limits::builder().document_nodes(1).build().unwrap(), json!([1]), LimitKind::DocumentNodes)]
#[case(Limits::builder().depth(1).build().unwrap(), json!([[1]]), LimitKind::Depth)]
#[case(Limits::builder().profile_paths(1).build().unwrap(), json!({"x":1}), LimitKind::ProfilePaths)]
#[case(Limits::builder().profile_path_bytes(2).build().unwrap(), json!({"long":1}), LimitKind::ProfilePathBytes)]
fn admission_limits_are_enforced(
    #[case] limits: Limits,
    #[case] document: Value,
    #[case] kind: LimitKind,
) {
    let mut profiler = Profiler::new(ProfilerOptions::default().with_limits(limits));
    assert_eq!(
        profiler.observe(&document),
        Err(ProfileError::LimitExceeded(kind))
    );
}

#[rstest]
#[case(Limits::builder().profile_paths(2).build().unwrap(), LimitKind::ProfilePaths)]
#[case(Limits::builder().profile_path_bytes(3).build().unwrap(), LimitKind::ProfilePathBytes)]
fn profile_limits_accumulate_across_documents(#[case] limits: Limits, #[case] kind: LimitKind) {
    let mut profiler = Profiler::new(ProfilerOptions::default().with_limits(limits));
    profiler.observe(&json!({"x":1})).unwrap();
    profiler.observe(&json!({"x":2})).unwrap();
    assert_eq!(
        profiler.observe(&json!({"y":3})),
        Err(ProfileError::LimitExceeded(kind))
    );
}

#[test]
fn repeated_array_items_share_one_admitted_path() {
    let mut profiler = Profiler::new(
        ProfilerOptions::default().with_limits(
            Limits::builder()
                .profile_paths(2)
                .profile_path_bytes(1)
                .build()
                .unwrap(),
        ),
    );
    profiler.observe(&json!([1, 2, 3, 4])).unwrap();
    assert_eq!(
        profiler
            .finish()
            .unwrap()
            .field(&ProfilePath::root().each_item())
            .unwrap()
            .present(),
        4
    );
}

#[test]
fn failed_observation_prevents_finishing() {
    let mut profiler = Profiler::new(
        ProfilerOptions::default().with_limits(Limits::builder().documents(1).build().unwrap()),
    );
    profiler.observe(&json!(1)).unwrap();
    assert!(profiler.observe(&json!(2)).is_err());
    assert!(matches!(profiler.finish(), Err(ProfileError::Incomplete)));
}

#[test]
fn failed_observation_prevents_further_ingestion() {
    let mut profiler = Profiler::new(
        ProfilerOptions::default().with_limits(Limits::builder().documents(1).build().unwrap()),
    );
    profiler.observe(&json!(1)).unwrap();
    assert!(profiler.observe(&json!(2)).is_err());
    assert_eq!(profiler.observe(&json!(3)), Err(ProfileError::Incomplete));
}

#[test]
fn limits_do_not_traverse_unselected_sibling_values() {
    let scope = Scope::selected([ProfilePath::root().property("x")]).unwrap();
    let mut profiler = Profiler::new(
        ProfilerOptions::default()
            .with_scope(scope)
            .with_limits(Limits::builder().document_nodes(2).build().unwrap()),
    );
    assert!(
        profiler
            .observe(&json!({"x":1,"ignored":[[[[1,2,3,4]]]]}))
            .is_ok()
    );
}

#[rstest]
#[case("/~0/~1/", vec!["~", "/", ""])]
#[case("/*/0", vec!["*", "0"])]
fn property_pointers_preserve_literal_names(#[case] pointer: &str, #[case] names: Vec<&str>) {
    let expected = names
        .into_iter()
        .fold(ProfilePath::root(), |path, name| path.property(name));
    assert_eq!(
        ProfilePath::from_property_pointer(pointer).unwrap(),
        expected
    );
}

#[rstest]
#[case("a")]
#[case("/~2")]
#[case("/~")]
fn rejects_malformed_pointer(#[case] pointer: &str) {
    assert!(ProfilePath::from_property_pointer(pointer).is_err());
}

#[test]
fn contradictory_selection_containers_are_rejected() {
    assert!(
        Scope::selected([
            ProfilePath::root().each_item(),
            ProfilePath::root().property("x")
        ])
        .is_err()
    );
}

#[test]
fn redundant_selection_is_collapsed() {
    let parent = ProfilePath::root().property("x");
    assert_eq!(
        Scope::selected([parent.clone().property("a"), parent.clone()])
            .unwrap()
            .paths(),
        &[parent]
    );
}

#[test]
fn empty_corpus_is_an_error() {
    assert!(matches!(
        Profiler::default().finish(),
        Err(ProfileError::EmptyCorpus)
    ));
}

#[test]
fn report_does_not_retain_scalar_values() {
    let report = profile(&[json!({"secret":"never-retain-this-scalar"})]);
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("never-retain-this-scalar")
    );
}

#[rstest]
#[case(1)]
#[case(7)]
#[case(42)]
#[case(2026)]
#[case(123456789)]
fn expansive_schema_covers_generated_nested_corpora(#[case] seed: u64) {
    fn next(state: &mut u64) -> u64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *state >> 32
    }
    fn value(state: &mut u64, depth: usize) -> Value {
        match next(state) % if depth == 0 { 5 } else { 7 } {
            0 => Value::Null,
            1 => json!(next(state).is_multiple_of(2)),
            2 => json!(next(state) % 100),
            3 => json!(0.5),
            4 => json!("synthetic"),
            5 => Value::Array(
                (0..next(state) % 4)
                    .map(|_| value(state, depth - 1))
                    .collect(),
            ),
            _ => Value::Object(
                (0..next(state) % 4)
                    .map(|key| (format!("key_{key}"), value(state, depth - 1)))
                    .collect(),
            ),
        }
    }
    let mut state = seed;
    let documents: Vec<_> = (0..128).map(|_| value(&mut state, 4)).collect();
    let candidate = schema(&profile(&documents), InferencePolicy::expansive());
    let validator = jsonschema::draft202012::new(&candidate).unwrap();
    assert!(
        documents
            .iter()
            .all(|document| validator.is_valid(document))
    );
}

#[rstest]
#[case("1.0")]
#[case("1e50")]
#[case("12345678901234567890123456789012345678901234567890")]
#[case("1.000000000000000000000000000000000001")]
fn numeric_candidate_covers_exact_input(#[case] token: &str) {
    let value: Value = serde_json::from_str(token).unwrap();
    let candidate = schema(
        &profile(std::slice::from_ref(&value)),
        InferencePolicy::strict(),
    );
    assert!(
        jsonschema::draft202012::new(&candidate)
            .unwrap()
            .is_valid(&value)
    );
}
