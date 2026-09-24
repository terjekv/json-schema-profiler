use json_schema_profiler::{
    CompiledSchema, Document, DocumentId, EvaluationOptions, EvaluationStatus, EvaluationStop,
    FormatPolicy, SchemaError, SchemaOptions, ValueLimit, ValueLimits,
};
use rstest::rstest;
use serde_json::{Value, json};

fn compile(schema: &Value) -> CompiledSchema {
    CompiledSchema::new(schema, SchemaOptions::default()).unwrap()
}

#[rstest]
#[case(json!({"type":"integer"}),json!(1),json!("1"))]
#[case(json!({"type":["null","integer"]}),json!(null),json!(true))]
#[case(json!({"const":2}),json!(2),json!(3))]
#[case(json!({"enum":[1,2]}),json!(1),json!(3))]
#[case(json!({"minimum":1,"maximum":3}),json!(2),json!(4))]
#[case(json!({"exclusiveMinimum":1,"exclusiveMaximum":3}),json!(2),json!(1))]
#[case(json!({"multipleOf":0.1}),json!(0.3),json!(0.31))]
#[case(json!({"minLength":2,"maxLength":4}),json!("abc"),json!("a"))]
#[case(json!({"pattern":"^[a-z]+$"}),json!("abc"),json!("123"))]
#[case(json!({"minItems":1,"maxItems":2}),json!([1]),json!([]))]
#[case(json!({"uniqueItems":true}),json!([1,2]),json!([1,1]))]
#[case(json!({"prefixItems":[{"type":"string"}],"items":{"type":"integer"}}),json!(["a",1]),json!(["a","b"]))]
#[case(json!({"contains":{"type":"integer"},"minContains":2,"maxContains":3}),json!([1,2,"a"]),json!([1,"a"]))]
#[case(json!({"properties":{"x":{"type":"integer"}},"required":["x"],"additionalProperties":false}),json!({"x":1}),json!({"x":1,"y":2}))]
#[case(json!({"required":["x"]}),json!({"x":1}),json!({}))]
#[case(json!({"patternProperties":{"^x":{"type":"integer"}}}),json!({"x1":1}),json!({"x1":"a"}))]
#[case(json!({"propertyNames":{"pattern":"^[a-z]+$"}}),json!({"abc":1}),json!({"12":1}))]
#[case(json!({"minProperties":1,"maxProperties":2}),json!({"a":1}),json!({}))]
#[case(json!({"dependentRequired":{"a":["b"]}}),json!({"a":1,"b":2}),json!({"a":1}))]
#[case(json!({"dependentSchemas":{"a":{"required":["b"]}}}),json!({"a":1,"b":2}),json!({"a":1}))]
#[case(json!({"allOf":[{"minimum":1},{"maximum":3}]}),json!(2),json!(4))]
#[case(json!({"anyOf":[{"type":"string"},{"type":"integer"}]}),json!(1),json!(true))]
#[case(json!({"oneOf":[{"type":"number"},{"type":"integer"}]}),json!(1.5),json!(1))]
#[case(json!({"not":{"type":"string"}}),json!(1),json!("a"))]
#[case(json!({"if":{"type":"string"},"then":{"minLength":2},"else":{"minimum":2}}),json!(3),json!(1))]
#[case(json!({"allOf":[{"properties":{"a":true}}],"unevaluatedProperties":false}),json!({"a":1}),json!({"b":1}))]
#[case(json!({"prefixItems":[true],"unevaluatedItems":false}),json!([1]),json!([1,2]))]
#[case(json!({"$defs":{"a":{"type":"integer"}},"$ref":"#/$defs/a"}),json!(1),json!("a"))]
#[case(json!({"$defs":{"a/b":{"type":"integer"}},"$ref":"#/$defs/a~1b"}),json!(1),json!("a"))]
#[case(json!({"$defs":{"a b":{"type":"integer"}},"$ref":"#/$defs/a%20b"}),json!(1),json!("a"))]
fn supported_assertions_accept_and_reject(
    #[case] schema: Value,
    #[case] valid: Value,
    #[case] invalid: Value,
) {
    let report = compile(&schema).evaluate(
        [Document::new(&valid), Document::new(&invalid)],
        EvaluationOptions::default(),
    );
    assert_eq!(
        (report.valid(), report.invalid(), report.status()),
        (1, 1, &EvaluationStatus::Complete)
    );
}

#[rstest]
#[case(json!(true),1,0)]
#[case(json!(false),0,1)]
fn boolean_schemas_have_expected_coverage(
    #[case] schema: Value,
    #[case] valid: u64,
    #[case] invalid: u64,
) {
    let value = json!({"anything":[1,null]});
    let report = compile(&schema).evaluate([Document::new(&value)], EvaluationOptions::default());
    assert_eq!((report.valid(), report.invalid()), (valid, invalid));
}

#[rstest]
#[case(json!({"$ref":"https://example.invalid/schema"}))]
#[case(json!({"$ref":"file:///etc/passwd"}))]
#[case(json!({"$ref":"relative.json"}))]
#[case(json!({"$defs":{"a":{"$ref":"https://example.invalid"}}}))]
#[case(json!({"examples":[{"$ref":"file:///etc/passwd"}],"$ref":"#/examples/0"}))]
#[case(json!({"$ref":"#missing-anchor"}))]
#[case(json!({"$ref":"#/$defs/missing"}))]
#[case(json!({"$defs":{"x":{"type":"string"}},"$ref":"#/$defs/~2"}))]
fn unsupported_references_are_rejected_before_compilation(#[case] schema: Value) {
    assert!(matches!(
        CompiledSchema::new(&schema, SchemaOptions::default()),
        Err(SchemaError::InvalidReference { .. })
    ));
}

#[rstest]
#[case(json!({"$ref":"#"}))]
#[case(json!({"items":{"$ref":"#"}}))]
#[case(json!({"$defs":{"a":{"$ref":"#/$defs/b"},"b":{"$ref":"#/$defs/a"}},"$ref":"#/$defs/a"}))]
fn cyclic_schemas_are_rejected(#[case] schema: Value) {
    assert!(matches!(
        CompiledSchema::new(&schema, SchemaOptions::default()),
        Err(SchemaError::RecursiveReference)
    ));
}

#[rstest]
#[case(json!({"type":"nonsense"}))]
#[case(json!({"required":true}))]
#[case(json!({"pattern":"["}))]
#[case(json!({"pattern":"(?=x)x"}))]
#[case(json!(42))]
#[case(json!({"properties":{"a":{"$id":"nested"}}}))]
#[case(json!({"$dynamicRef":"#"}))]
#[case(json!({"$schema":"http://json-schema.org/draft-07/schema#"}))]
#[case(json!({"$vocabulary":{"https://example.invalid/vocab":true}}))]
fn unsupported_or_invalid_schemas_are_not_silently_accepted(#[case] schema: Value) {
    assert!(CompiledSchema::new(&schema, SchemaOptions::default()).is_err());
}

#[test]
fn annotation_payloads_are_not_interpreted_as_schemas() {
    let literal = json!({"$ref":"https://example.invalid","$id":"literal"});
    let schema = json!({"const":literal,"examples":[literal],"default":literal,"$vocabulary":{"https://example.invalid/optional":false}});
    assert!(
        compile(&schema)
            .evaluate([Document::new(&literal)], EvaluationOptions::default())
            .all_valid()
    );
}

#[rstest]
#[case(FormatPolicy::Annotate, true)]
#[case(FormatPolicy::Assert, false)]
fn formats_require_explicit_assertion(#[case] formats: FormatPolicy, #[case] valid: bool) {
    let schema = CompiledSchema::new(
        &json!({"format":"email"}),
        SchemaOptions::default().with_formats(formats),
    )
    .unwrap();
    assert_eq!(
        schema
            .evaluate(
                [Document::new(&json!("bad-address"))],
                EvaluationOptions::default()
            )
            .all_valid(),
        valid
    );
}

#[test]
fn required_format_vocabulary_cannot_be_silently_downgraded() {
    let schema = json!({"$vocabulary":{"https://json-schema.org/draft/2020-12/vocab/format-assertion":true}});
    assert!(matches!(
        CompiledSchema::new(&schema, SchemaOptions::default()),
        Err(SchemaError::UnsupportedVocabulary)
    ));
}

#[test]
fn unknown_asserted_formats_are_rejected() {
    assert!(
        CompiledSchema::new(
            &json!({"format":"unknown-format"}),
            SchemaOptions::default().with_formats(FormatPolicy::Assert)
        )
        .is_err()
    );
}

#[test]
fn content_keywords_are_annotations() {
    assert!(compile(&json!({"contentEncoding":"base64","contentMediaType":"application/json","contentSchema":false})).evaluate([Document::new(&json!("not base64"))],EvaluationOptions::default()).all_valid());
}

#[test]
fn coverage_counts_documents_independently_of_diagnostic_caps() {
    let schema = compile(&json!({"required":["a","b"]}));
    let docs = vec![json!({}); 20];
    let report = schema.evaluate(
        docs.iter().map(Document::new),
        EvaluationOptions::default().with_diagnostic_limits(2, 1),
    );
    assert_eq!(
        (
            report.processed(),
            report.invalid(),
            report.diagnostics().len(),
            report.diagnostics_truncated(),
            report.status()
        ),
        (20, 20, 2, true, &EvaluationStatus::Complete)
    );
}

#[test]
fn diagnostics_include_ids_and_pointers_without_instance_values() {
    let schema = compile(&json!({"properties":{"a/b":{"type":"integer"}}}));
    let id = DocumentId::new("record-7").unwrap();
    let value = json!({"a/b":"secret-value"});
    let report = schema.evaluate(
        [Document::new(&value).with_id(&id)],
        EvaluationOptions::default(),
    );
    let diagnostic = &report.diagnostics()[0];
    assert_eq!(
        (
            diagnostic.document_id().unwrap().as_str(),
            diagnostic.instance_path(),
            diagnostic.keyword()
        ),
        ("record-7", "/a~1b", "type")
    );
    assert!(
        !serde_json::to_string(&report)
            .unwrap()
            .contains("secret-value")
    );
}

#[test]
fn report_byte_budget_preserves_coverage() {
    let value = json!({"x".repeat(2000):"private"});
    let schema = compile(&json!({"additionalProperties":{"type":"integer"}}));
    let report = schema.evaluate(
        [Document::new(&value)],
        EvaluationOptions::default().with_report_bytes(512).unwrap(),
    );
    assert!(
        report.invalid() == 1
            && report.diagnostics_truncated()
            && serde_json::to_vec(&report).unwrap().len() <= 512
    );
}

#[rstest]
#[case(1, EvaluationStatus::Complete)]
#[case(2, EvaluationStatus::Incomplete(EvaluationStop::DocumentLimit))]
fn document_limit_distinguishes_exact_completion(
    #[case] count: usize,
    #[case] expected: EvaluationStatus,
) {
    let docs = vec![json!(1); count];
    let report = compile(&json!(true)).evaluate(
        docs.iter().map(Document::new),
        EvaluationOptions::default().with_document_limit(1),
    );
    assert_eq!(report.status(), &expected);
}

#[test]
fn value_limits_stop_before_counting_unchecked_document() {
    let docs = [json!(1), json!([1])];
    let options =
        EvaluationOptions::default().with_value_limits(ValueLimits::new(1, 10, 1000).unwrap());
    let report = compile(&json!(true)).evaluate(docs.iter().map(Document::new), options);
    assert_eq!(
        (report.processed(), report.status()),
        (
            1,
            &EvaluationStatus::Incomplete(EvaluationStop::ValueLimit {
                document_index: 1,
                limit: ValueLimit::Nodes
            })
        )
    );
}

#[rstest]
#[case("1e1000000000")]
#[case("1e-1000000000")]
fn numeric_expansion_is_bounded_before_validation(#[case] token: &str) {
    let value: Value = serde_json::from_str(token).unwrap();
    let report = compile(&json!({"type":"integer"}))
        .evaluate([Document::new(&value)], EvaluationOptions::default());
    assert!(matches!(
        report.status(),
        EvaluationStatus::Incomplete(EvaluationStop::ValueLimit {
            limit: ValueLimit::NumberSize,
            ..
        })
    ));
}

#[test]
fn exact_large_integer_bounds_are_not_rounded() {
    let schema: Value =
        serde_json::from_str(r#"{"maximum":123456789012345678901234567890}"#).unwrap();
    let invalid: Value = serde_json::from_str("123456789012345678901234567891").unwrap();
    assert_eq!(
        compile(&schema)
            .evaluate([Document::new(&invalid)], EvaluationOptions::default())
            .invalid(),
        1
    );
}

#[test]
fn empty_replay_cannot_create_verified_evidence() {
    assert!(
        compile(&json!(true))
            .verify([], EvaluationOptions::default())
            .is_err()
    );
}

#[test]
fn incomplete_replay_cannot_create_verified_evidence() {
    let docs = [json!(1), json!(2)];
    assert!(
        compile(&json!(true))
            .verify(
                docs.iter().map(Document::new),
                EvaluationOptions::default().with_document_limit(1)
            )
            .is_err()
    );
}

#[test]
fn completed_nonempty_replay_can_create_verified_evidence() {
    let value = json!(1);
    assert_eq!(
        compile(&json!({"type":"integer"}))
            .verify([Document::new(&value)], EvaluationOptions::default())
            .unwrap()
            .evaluation()
            .valid(),
        1
    );
}

#[rstest]
#[case("date", "2026-09-24", "2026-99-99")]
#[case("date-time", "2026-09-24T12:00:00Z", "invalid")]
#[case("time", "12:00:00Z", "invalid")]
#[case("duration", "P1D", "invalid")]
#[case("email", "user@example.com", "invalid")]
#[case("idn-email", "user@example.com", "invalid")]
#[case("hostname", "example.com", "bad host")]
#[case("idn-hostname", "example.com", "bad host")]
#[case("ipv4", "192.0.2.1", "999.999.999.999")]
#[case("ipv6", "2001:db8::1", "invalid")]
#[case("uri", "https://example.com/path", "bad uri")]
#[case("uri-reference", "/path", "bad uri")]
#[case("iri", "https://example.com/path", "bad iri")]
#[case("iri-reference", "/path", "bad iri")]
#[case("uri-template", "https://example.com/{id}", "{unclosed")]
#[case("json-pointer", "/a~1b", "invalid")]
#[case("relative-json-pointer", "0/a", "invalid")]
#[case("regex", "^[a-z]+$", "[")]
#[case("uuid", "550e8400-e29b-41d4-a716-446655440000", "invalid")]
fn documented_formats_accept_and_reject(
    #[case] format: &str,
    #[case] valid: &str,
    #[case] invalid: &str,
) {
    let schema = CompiledSchema::new(
        &json!({"format":format}),
        SchemaOptions::default().with_formats(FormatPolicy::Assert),
    )
    .unwrap();
    let values = [json!(valid), json!(invalid)];
    let report = schema.evaluate(
        values.iter().map(Document::new),
        EvaluationOptions::default(),
    );
    assert_eq!((report.valid(), report.invalid()), (1, 1));
}

#[rstest]
#[case(ValueLimits::new(1,10,1000).unwrap(),json!({"type":"integer"}),ValueLimit::Nodes)]
#[case(ValueLimits::new(100,1,1000).unwrap(),json!({"properties":{"a":true}}),ValueLimit::Depth)]
#[case(ValueLimits::new(100,10,1).unwrap(),json!({}),ValueLimit::Bytes)]
fn schema_limits_fail_before_compilation(
    #[case] limits: ValueLimits,
    #[case] schema: Value,
    #[case] expected: ValueLimit,
) {
    assert!(
        matches!(CompiledSchema::new(&schema,SchemaOptions::default().with_limits(limits)),Err(SchemaError::LimitExceeded(kind)) if kind==expected)
    );
}

#[test]
fn deep_reference_expansion_is_rejected_without_recursive_compilation() {
    let mut definitions = serde_json::Map::new();
    for index in 0..130 {
        definitions.insert(
            format!("item_{index}"),
            json!({"$ref":format!("#/$defs/item_{}",index+1)}),
        );
    }
    definitions.insert("item_130".into(), json!(true));
    let schema = json!({"$defs":definitions,"$ref":"#/$defs/item_0"});
    assert!(matches!(
        CompiledSchema::new(&schema, SchemaOptions::default()),
        Err(SchemaError::RecursiveReference)
    ));
}

#[test]
fn schema_compile_errors_do_not_include_schema_literal_values() {
    let schema = json!({"pattern":"[private-pattern","const":"private-scalar"});
    let Err(error) = CompiledSchema::new(&schema, SchemaOptions::default()) else {
        panic!("invalid regex");
    };
    assert!(!error.to_string().contains("private-"));
}

#[test]
fn required_format_assertion_vocabulary_works_when_enabled() {
    let schema = json!({"$vocabulary":{"https://json-schema.org/draft/2020-12/vocab/format-assertion":true},"format":"email"});
    let compiled = CompiledSchema::new(
        &schema,
        SchemaOptions::default().with_formats(FormatPolicy::Assert),
    )
    .unwrap();
    assert_eq!(
        compiled
            .evaluate(
                [Document::new(&json!("invalid"))],
                EvaluationOptions::default()
            )
            .invalid(),
        1
    );
}
