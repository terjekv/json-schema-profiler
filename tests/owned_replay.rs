use std::cell::Cell;

use json_schema_profiler::{
    CompiledSchema, Document, DocumentId, EvaluationOptions, EvaluationStatus, EvaluationStop,
    OwnedDocument, SchemaOptions, ValueLimits, VerificationError,
};
use rstest::rstest;
use serde_json::{Value, json};

fn compiled() -> CompiledSchema {
    CompiledSchema::new(&json!({"type": "integer"}), SchemaOptions::default()).unwrap()
}

#[rstest]
#[case(EvaluationOptions::default())]
#[case(EvaluationOptions::default().with_diagnostic_limits(0, 0))]
#[case(EvaluationOptions::default().with_diagnostic_limits(1, 1))]
#[case(EvaluationOptions::default().with_document_limit(2))]
#[case(EvaluationOptions::default().with_document_limit(0))]
#[case(EvaluationOptions::default().with_report_bytes(512).unwrap())]
fn owned_and_borrowed_reports_match(#[case] options: EvaluationOptions) {
    let values = [json!(1), json!("private"), json!(true), json!(2)];
    let ids: Vec<_> = (0..values.len())
        .map(|i| DocumentId::new(format!("record-{i}")).unwrap())
        .collect();
    let compiled = compiled();
    let borrowed = compiled.evaluate(
        values
            .iter()
            .zip(&ids)
            .map(|(value, id)| Document::new(value).with_id(id)),
        options.clone(),
    );
    let owned = compiled.evaluate_owned(
        values
            .into_iter()
            .zip(ids)
            .map(|(value, id)| OwnedDocument::new(value).with_id(id)),
        options,
    );
    assert_eq!(
        serde_json::to_value(borrowed).unwrap(),
        serde_json::to_value(owned).unwrap()
    );
}

#[rstest]
#[case(0, 0)]
#[case(1, 1)]
#[case(10, 1)]
fn owned_source_failure_preserves_lookahead_and_stops_polling(
    #[case] limit: usize,
    #[case] prefix: usize,
) {
    let polls = Cell::new(0);
    let source = (0..).map(|i| {
        polls.set(polls.get() + 1);
        if i == prefix {
            Err("private source failure")
        } else {
            Ok(OwnedDocument::new(json!(i)))
        }
    });
    let error = compiled()
        .try_evaluate_owned(
            source,
            EvaluationOptions::default().with_document_limit(limit),
        )
        .unwrap_err();
    assert_eq!(polls.get(), prefix + 1);
    assert_eq!(error.evaluation().processed(), prefix as u64);
    assert_eq!(
        error.evaluation().status(),
        &EvaluationStatus::Incomplete(EvaluationStop::InputError {
            document_index: prefix as u64
        })
    );
    assert_eq!(*error.input_error(), "private source failure");
    assert!(!format!("{error:?}").contains("private source failure"));
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(3)]
fn owned_document_limit_consumes_only_one_lookahead(#[case] limit: usize) {
    let polls = Cell::new(0);
    let source = (0..).map(|i| {
        polls.set(polls.get() + 1);
        Ok::<_, ()>(OwnedDocument::new(json!(i)))
    });
    let report = compiled()
        .try_evaluate_owned(
            source,
            EvaluationOptions::default().with_document_limit(limit),
        )
        .unwrap();
    assert_eq!(polls.get(), limit + 1);
    assert_eq!(report.processed(), limit as u64);
    assert_eq!(
        report.status(),
        &EvaluationStatus::Incomplete(EvaluationStop::DocumentLimit)
    );
}

#[rstest]
#[case(vec![], false)]
#[case(vec![json!(1), json!(2)], true)]
#[case(vec![json!(1), json!("invalid")], false)]
fn owned_verification_requires_complete_nonempty_valid_input(
    #[case] values: Vec<Value>,
    #[case] valid: bool,
) {
    assert_eq!(
        compiled()
            .verify_owned(
                values.into_iter().map(OwnedDocument::new),
                EvaluationOptions::default()
            )
            .is_ok(),
        valid
    );
}

#[test]
fn parse_on_demand_preserves_the_original_parse_error() {
    let source = ["1", "2", "broken", "3"]
        .into_iter()
        .map(|line| serde_json::from_str::<Value>(line).map(OwnedDocument::new));
    let VerificationError::Input(error) = compiled()
        .try_verify_owned(source, EvaluationOptions::default())
        .unwrap_err()
    else {
        panic!("expected input failure")
    };
    assert_eq!(error.evaluation().valid(), 2);
    assert!(error.input_error().is_syntax());
}

#[test]
fn owned_value_limit_stops_before_polling_another_input() {
    let source = [Ok(OwnedDocument::new(json!([1, 2]))), Err("unread")];
    let result = compiled().try_verify_owned(
        source,
        EvaluationOptions::default().with_value_limits(ValueLimits::new(1, 4, 100).unwrap()),
    );
    assert!(matches!(result, Err(VerificationError::Evaluation(_))));
}

#[test]
fn owned_record_debug_does_not_expose_values_or_labels() {
    let document =
        OwnedDocument::new(json!("private value")).with_id(DocumentId::new("private ID").unwrap());
    assert_eq!(format!("{document:?}"), "OwnedDocument { .. }");
}
