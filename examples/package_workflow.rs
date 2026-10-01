use std::error::Error;

use json_schema_profiler::{
    CompiledSchema, Document, EvaluationOptions, InferencePolicy, Profiler, SchemaOptions,
    Suggestion, VerificationError,
};
use serde_json::{Value, json};

// This file is also copied into a separate application when auditing the archive.
// It deliberately uses only the consumer-facing API and normal dependencies.
fn fixture() -> Result<(CompiledSchema, [Value; 2]), Box<dyn Error>> {
    let documents = [json!({"cores": 4}), json!({"cores": 8})];
    let mut profiler = Profiler::default();
    for value in &documents {
        profiler.observe(value)?;
    }
    let Suggestion::Candidate(candidate) = profiler.finish()?.suggest(InferencePolicy::strict())?
    else {
        return Err("fixture must produce a candidate".into());
    };
    Ok((candidate.compile(SchemaOptions::default())?, documents))
}

fn accepted_replay() -> Result<(), Box<dyn Error>> {
    let (compiled, documents) = fixture()?;
    let verified = compiled
        .verify(
            documents.iter().map(Document::new),
            EvaluationOptions::default(),
        )
        .expect("the supplied corpus should pass");
    assert_eq!(verified.evaluation().valid(), 2);
    Ok(())
}

fn rejected_replay() -> Result<(), Box<dyn Error>> {
    let (compiled, _) = fixture()?;
    let invalid = json!({"cores": "four"});
    let rejected = compiled
        .verify([Document::new(&invalid)], EvaluationOptions::default())
        .unwrap_err();
    assert_eq!(rejected.invalid(), 1);
    assert!(!rejected.all_valid());
    Ok(())
}

fn failed_source() -> Result<(), Box<dyn Error>> {
    let (compiled, documents) = fixture()?;
    let replay = [
        Ok(Document::new(&documents[0])),
        Err("synthetic read failure"),
    ];
    let VerificationError::Input(error) = compiled
        .try_verify(replay, EvaluationOptions::default())
        .unwrap_err()
    else {
        return Err("source failure must stay distinct from validation failure".into());
    };
    assert_eq!(error.input_error(), &"synthetic read failure");
    assert_eq!(error.evaluation().valid(), 1);
    assert!(!error.evaluation().all_valid());
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    accepted_replay()?;
    rejected_replay()?;
    failed_source()?;
    println!("Package workflow passed: accepted replay, rejected data, preserved source failure.");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn accepts_supplied_corpus() {
        super::accepted_replay().unwrap();
    }

    #[test]
    fn rejects_incompatible_data() {
        super::rejected_replay().unwrap();
    }

    #[test]
    fn preserves_source_failure() {
        super::failed_source().unwrap();
    }
}
