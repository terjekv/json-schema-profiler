use std::{error::Error, time::Instant};

use inventory::{ApprovalFailure, Inventory};
use json_schema_profiler::{
    EvaluationOptions, EvaluationStatus, EvaluationStop, InferencePolicy, JsonKind, MixedTypes,
    ProfilePath, Suggestion, SuggestionOptions, VerificationError,
};
use serde_json::json;

mod inventory;
#[cfg(test)]
mod tests;

fn reviewed_policy() -> SuggestionOptions {
    SuggestionOptions::new(InferencePolicy::strict())
        .with_path_policy(
            ProfilePath::root().property("hardware").property("cores"),
            InferencePolicy::strict().with_mixed_types(MixedTypes::Union),
        )
        .unwrap()
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut inventory = Inventory::synthetic();
    let started = Instant::now();
    let snapshot = inventory.snapshot();
    let parse_time = started.elapsed();
    let started = Instant::now();
    let analysis = snapshot.analyze()?;
    let profile_time = started.elapsed();
    let profile = analysis.profile();
    let address = profile
        .field(
            &ProfilePath::root()
                .property("interfaces")
                .each_item()
                .property("address"),
        )
        .unwrap();
    let owner = profile
        .field(&ProfilePath::root().property("hardware").property("owner"))
        .unwrap();
    assert_eq!((address.present(), address.evidence().documents()), (3, 2));
    assert_eq!((owner.missing(), owner.types().get(JsonKind::Null)), (1, 1));
    assert!(address.evidence().witnesses_truncated());
    let Suggestion::Blocked(conflicts) = profile.suggest(InferencePolicy::strict())? else {
        panic!("the fixture's mixed core types must block strict inference");
    };
    println!(
        "Strict policy blocked: {}",
        serde_json::to_string(&conflicts)?
    );
    let started = Instant::now();
    let review = analysis.propose(reviewed_policy())?;
    let suggestion_time = started.elapsed();
    println!("Reviewed candidate: {}", review.candidate().schema());
    println!(
        "Findings: {}",
        serde_json::to_string(review.candidate().findings())?
    );
    let started = Instant::now();
    let compiled = review.compile()?;
    let compile_time = started.elapsed();
    let started = Instant::now();
    let approved = compiled
        .verify(&snapshot, EvaluationOptions::default())
        .unwrap();
    let replay_time = started.elapsed();
    assert_eq!(approved.evidence().evaluation().valid(), 3);
    inventory.apply(approved).unwrap();
    assert!(inventory.applied_schema().is_some());
    println!("Verified and applied revision 1: 3/3 records accepted.");

    let unreadable = inventory.unreadable_snapshot(1);
    let failure = compiled
        .verify(&unreadable, EvaluationOptions::default())
        .unwrap_err();
    let ApprovalFailure::Replay(VerificationError::Input(error)) = failure else {
        panic!("a source failure must prevent approval");
    };
    assert_eq!(error.evaluation().valid(), 1);
    println!(
        "Read failure: {}",
        serde_json::to_string(error.evaluation())?
    );

    let limited = compiled
        .verify(
            &snapshot,
            EvaluationOptions::default().with_document_limit(1),
        )
        .unwrap_err();
    assert!(
        matches!(limited, ApprovalFailure::Replay(VerificationError::Evaluation(ref report))
        if report.status() == &EvaluationStatus::Incomplete(EvaluationStop::DocumentLimit))
    );
    println!("Limited replay: approval rejected.");

    let pending = compiled
        .verify(&snapshot, EvaluationOptions::default())
        .unwrap();
    inventory.replace_row(
        0,
        &json!({"hardware":{"cores":true},"interfaces":[]}).to_string(),
    );
    assert!(inventory.apply(pending).is_err());
    let changed = inventory.snapshot();
    assert!(matches!(
        compiled.verify(&changed, EvaluationOptions::default()),
        Err(ApprovalFailure::Stale(_))
    ));
    let report = compiled
        .schema()
        .try_evaluate(changed.replay(), EvaluationOptions::default())
        .unwrap();
    assert_eq!((report.valid(), report.invalid()), (2, 1));
    println!(
        "Stale revision rejected; changed data: {}",
        serde_json::to_string(&report)?
    );

    for index in 0..3 {
        inventory.replace_row(index, r#"{"hardware":{"cores":false},"interfaces":[]}"#);
    }
    let rejected = inventory.snapshot();
    let report = compiled
        .schema()
        .try_evaluate(
            rejected.replay(),
            EvaluationOptions::default().with_diagnostic_limits(1, 1),
        )
        .unwrap();
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
    println!(
        "Bounded rejection diagnostics: {}",
        serde_json::to_string(&report)?
    );

    inventory.replace_row(1, "{invalid JSON");
    let malformed = inventory.snapshot();
    let failure = compiled
        .schema()
        .try_verify(malformed.replay(), EvaluationOptions::default())
        .unwrap_err();
    assert!(matches!(failure, VerificationError::Input(_)));
    println!(
        "Parse failure: {}",
        serde_json::to_string(failure.evaluation())?
    );
    println!(
        "Snapshot parse={parse_time:?}; profile={profile_time:?}; suggest={suggestion_time:?}; compile={compile_time:?}; replay={replay_time:?}"
    );
    println!(
        "Retained encoded outputs: profile={} bytes, candidate={} bytes; parsed snapshot memory belongs to the application.",
        serde_json::to_vec(profile)?.len(),
        serde_json::to_vec(review.candidate())?.len()
    );
    Ok(())
}
