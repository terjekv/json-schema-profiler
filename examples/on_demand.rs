//! Parse and verify one JSON Lines record at a time using a caller-owned reader.
use std::{
    error::Error,
    io::{BufRead, Cursor},
};

use json_schema_profiler::{
    DocumentId, EvaluationOptions, InferencePolicy, OwnedDocument, Profiler, SchemaOptions,
    Suggestion,
};
use serde_json::Value;

fn replay(reader: impl BufRead) -> impl Iterator<Item = Result<OwnedDocument, Box<dyn Error>>> {
    reader.lines().enumerate().map(|(index, line)| {
        let value: Value = serde_json::from_str(&line?)?;
        Ok(OwnedDocument::new(value).with_id(DocumentId::new(format!("line-{index}"))?))
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    // The immutable bytes are this example's snapshot; a file/database consumer
    // must supply its own snapshot identity and consistency guarantee.
    let source = b"{\"cores\":4}\n{\"cores\":8}\n";
    let mut profiler = Profiler::default();
    for line in Cursor::new(source).lines() {
        let value = serde_json::from_str(&line?)?;
        profiler.observe(&value)?;
    }
    let Suggestion::Candidate(candidate) =
        profiler.finish()?.suggest(InferencePolicy::balanced())?
    else {
        return Err("review inference findings before continuing".into());
    };
    let compiled = candidate.compile(SchemaOptions::default())?;
    let verified = compiled
        .try_verify_owned(replay(Cursor::new(source)), EvaluationOptions::default())
        .map_err(|error| format!("replay failed: {error}"))?;
    assert_eq!(verified.evaluation().valid(), 2);
    println!("Verified two records parsed on demand.");
    Ok(())
}

#[test]
fn reader_workflow_accepts_the_snapshot() {
    main().unwrap();
}

#[test]
fn reader_errors_cannot_become_successful_exhaustion() {
    let compiled = json_schema_profiler::CompiledSchema::new(
        &serde_json::json!({"type":"integer"}),
        SchemaOptions::default(),
    )
    .unwrap();
    let error = compiled
        .try_verify_owned(
            replay(Cursor::new(b"1\nbroken\n2\n")),
            EvaluationOptions::default(),
        )
        .unwrap_err();
    assert_eq!(error.evaluation().processed(), 1);
    assert!(matches!(
        error,
        json_schema_profiler::VerificationError::Input(_)
    ));
}
