use json_schema_profiler::{Document, Finding, ValueLimit, ValueLimits};
use rstest::rstest;

use super::*;

#[test]
fn selection_and_evidence_preserve_applicable_occurrences() {
    let snapshot = Inventory::synthetic().snapshot();
    let analysis = snapshot.analyze().unwrap();
    let profile = analysis.profile();
    assert!(
        profile
            .field(&ProfilePath::root().property("ignored"))
            .is_none()
    );
    let owner = profile
        .field(&ProfilePath::root().property("hardware").property("owner"))
        .unwrap();
    assert_eq!(
        (
            owner.present(),
            owner.missing(),
            owner.types().get(JsonKind::Null)
        ),
        (2, 1, 1)
    );
    let address = profile
        .field(
            &ProfilePath::root()
                .property("interfaces")
                .each_item()
                .property("address"),
        )
        .unwrap();
    assert_eq!((address.present(), address.evidence().documents()), (3, 2));
    assert_eq!(address.evidence().witnesses().len(), 1);
    assert!(address.evidence().witnesses_truncated());
    let encoded = serde_json::to_string(profile).unwrap();
    assert!(!encoded.contains("192.0.2.1"));
    assert!(!encoded.contains("private-a"));
}

#[test]
fn strict_policy_explains_the_mixed_type_conflict() {
    let snapshot = Inventory::synthetic().snapshot();
    let analysis = snapshot.analyze().unwrap();
    let Suggestion::Blocked(findings) = analysis
        .profile()
        .suggest(InferencePolicy::strict())
        .unwrap()
    else {
        panic!("strict policy must fail");
    };
    assert!(
        findings
            .iter()
            .any(|finding| matches!(finding, Finding::MixedTypes { path, .. }
        if path == &ProfilePath::root().property("hardware").property("cores")))
    );
    assert!(
        analysis
            .propose(SuggestionOptions::new(InferencePolicy::strict()))
            .is_err()
    );
}

#[rstest]
#[case(json!({"hardware":{"cores":true},"interfaces":[]}))]
#[case(json!({"hardware":{},"interfaces":[]}))]
#[case(json!({"hardware":{"cores":4,"surprise":1},"interfaces":[]}))]
fn union_override_preserves_other_constraints(#[case] invalid: serde_json::Value) {
    let snapshot = Inventory::synthetic().snapshot();
    let review = snapshot
        .analyze()
        .unwrap()
        .propose(reviewed_policy())
        .unwrap();
    let compiled = review.compile().unwrap();
    assert!(
        compiled
            .schema()
            .verify([Document::new(&invalid)], EvaluationOptions::default())
            .is_err()
    );
}

#[test]
fn same_snapshot_can_be_verified_and_applied() {
    let mut inventory = Inventory::synthetic();
    let snapshot = inventory.snapshot();
    let review = snapshot
        .analyze()
        .unwrap()
        .propose(reviewed_policy())
        .unwrap();
    let compiled = review.compile().unwrap();
    let approval = compiled
        .verify(&snapshot, EvaluationOptions::default())
        .unwrap();
    assert_eq!(approval.evidence().evaluation().valid(), 3);
    inventory.apply(approval).unwrap();
    assert_eq!(
        inventory.applied_schema(),
        Some(review.candidate().schema())
    );
}

#[test]
fn equal_revision_numbers_from_another_inventory_are_rejected() {
    let inventory = Inventory::synthetic();
    let snapshot = inventory.snapshot();
    let compiled = snapshot
        .analyze()
        .unwrap()
        .propose(reviewed_policy())
        .unwrap()
        .compile()
        .unwrap();
    let mut other = Inventory::synthetic();
    assert!(matches!(
        compiled.verify(&other.snapshot(), EvaluationOptions::default()),
        Err(ApprovalFailure::Stale(_))
    ));
    let approved = compiled
        .verify(&snapshot, EvaluationOptions::default())
        .unwrap();
    assert!(other.apply(approved).is_err());
    assert!(other.applied_schema().is_none());
}

#[test]
fn revision_change_before_replay_prevents_approval() {
    let mut inventory = Inventory::synthetic();
    let snapshot = inventory.snapshot();
    let compiled = snapshot
        .analyze()
        .unwrap()
        .propose(reviewed_policy())
        .unwrap()
        .compile()
        .unwrap();
    inventory.replace_row(0, r#"{"hardware":{"cores":8},"interfaces":[]}"#);
    assert!(matches!(
        compiled.verify(&inventory.snapshot(), EvaluationOptions::default()),
        Err(ApprovalFailure::Stale(_))
    ));
    assert!(inventory.applied_schema().is_none());
}

#[test]
fn revision_change_after_verification_prevents_application() {
    let mut inventory = Inventory::synthetic();
    let snapshot = inventory.snapshot();
    let compiled = snapshot
        .analyze()
        .unwrap()
        .propose(reviewed_policy())
        .unwrap()
        .compile()
        .unwrap();
    let approval = compiled
        .verify(&snapshot, EvaluationOptions::default())
        .unwrap();
    inventory.replace_row(0, r#"{"hardware":{"cores":8},"interfaces":[]}"#);
    assert!(inventory.apply(approval).is_err());
    assert!(inventory.applied_schema().is_none());
}

#[test]
fn read_failure_returns_incomplete_coverage_and_the_original_error() {
    let inventory = Inventory::synthetic();
    let snapshot = inventory.snapshot();
    let compiled = snapshot
        .analyze()
        .unwrap()
        .propose(reviewed_policy())
        .unwrap()
        .compile()
        .unwrap();
    let failed = inventory.unreadable_snapshot(1);
    assert!(failed.analyze().is_err());
    let ApprovalFailure::Replay(VerificationError::Input(error)) = compiled
        .verify(&failed, EvaluationOptions::default())
        .unwrap_err()
    else {
        panic!("expected a source failure");
    };
    assert_eq!(error.evaluation().valid(), 1);
    assert_eq!(
        error.evaluation().status(),
        &EvaluationStatus::Incomplete(EvaluationStop::InputError { document_index: 1 })
    );
    assert!(
        error
            .input_error()
            .to_string()
            .contains("private read failure")
    );
    assert!(!format!("{error:?}").contains("private read failure"));
    assert!(
        !serde_json::to_string(error.evaluation())
            .unwrap()
            .contains("private read failure")
    );
}

#[test]
fn malformed_json_is_a_source_failure_instead_of_iterator_exhaustion() {
    let mut inventory = Inventory::synthetic();
    let snapshot = inventory.snapshot();
    let compiled = snapshot
        .analyze()
        .unwrap()
        .propose(reviewed_policy())
        .unwrap()
        .compile()
        .unwrap();
    inventory.replace_row(1, "{invalid JSON");
    let malformed = inventory.snapshot();
    assert!(malformed.analyze().is_err());
    let VerificationError::Input(error) = compiled
        .schema()
        .try_verify(malformed.replay(), EvaluationOptions::default())
        .unwrap_err()
    else {
        panic!("expected a parse failure");
    };
    assert_eq!(error.evaluation().processed(), 1);
    assert!(
        error
            .input_error()
            .get_ref()
            .unwrap()
            .is::<serde_json::Error>()
    );
}

#[test]
fn diagnostic_truncation_preserves_exact_rejection_counts_and_labels() {
    let mut inventory = Inventory::synthetic();
    let snapshot = inventory.snapshot();
    let compiled = snapshot
        .analyze()
        .unwrap()
        .propose(reviewed_policy())
        .unwrap()
        .compile()
        .unwrap();
    for index in 0..3 {
        inventory.replace_row(index, r#"{"hardware":{"cores":true},"interfaces":[]}"#);
    }
    let invalid = inventory.snapshot();
    let VerificationError::Evaluation(report) = compiled
        .schema()
        .try_verify(
            invalid.replay(),
            EvaluationOptions::default().with_diagnostic_limits(1, 1),
        )
        .unwrap_err()
    else {
        panic!("expected rejected records");
    };
    assert_eq!(
        (
            report.processed(),
            report.invalid(),
            report.diagnostics().len()
        ),
        (3, 3, 1)
    );
    assert_eq!(report.status(), &EvaluationStatus::Complete);
    assert!(report.diagnostics_truncated());
    assert_eq!(
        report.diagnostics()[0].document_id().unwrap().as_str(),
        "node-a"
    );
    assert_eq!(report.diagnostics()[0].instance_path(), "/hardware/cores");
    assert_eq!(report.diagnostics()[0].keyword(), "type");
}

#[rstest]
#[case(EvaluationOptions::default().with_document_limit(1), EvaluationStop::DocumentLimit, 1)]
#[case(EvaluationOptions::default().with_value_limits(ValueLimits::new(1, 10, 1000).unwrap()),
    EvaluationStop::ValueLimit { document_index: 0, limit: ValueLimit::Nodes }, 0)]
fn resource_stops_cannot_be_applied(
    #[case] options: EvaluationOptions,
    #[case] stop: EvaluationStop,
    #[case] processed: u64,
) {
    let inventory = Inventory::synthetic();
    let snapshot = inventory.snapshot();
    let compiled = snapshot
        .analyze()
        .unwrap()
        .propose(reviewed_policy())
        .unwrap()
        .compile()
        .unwrap();
    let ApprovalFailure::Replay(VerificationError::Evaluation(report)) =
        compiled.verify(&snapshot, options).unwrap_err()
    else {
        panic!("expected a resource stop");
    };
    assert_eq!(report.status(), &EvaluationStatus::Incomplete(stop));
    assert_eq!(report.processed(), processed);
    assert!(inventory.applied_schema().is_none());
}
