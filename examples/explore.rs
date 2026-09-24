use json_schema_profiler::{
    CompiledSchema, Document, DocumentId, EvaluationOptions, InferencePolicy, ProfilePath,
    Profiler, SchemaOptions, Suggestion,
};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let documents = [
        json!({"size":4}),
        json!({"size":"4"}),
        json!({"size":null}),
        json!({}),
    ];
    let ids = (0..documents.len())
        .map(|index| DocumentId::new(format!("object-{index}")))
        .collect::<Result<Vec<_>, _>>()?;
    let mut profiler = Profiler::default();
    for (id, document) in ids.iter().zip(&documents) {
        profiler.observe_with_id(id, document)?;
    }
    let profile = profiler.finish()?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            profile
                .field(&ProfilePath::root().property("size"))
                .unwrap()
        )?
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&profile.suggest(InferencePolicy::strict())?)?
    );
    let expansive = profile.suggest(InferencePolicy::expansive())?;
    println!("{}", serde_json::to_string_pretty(&expansive)?);
    let replay = || {
        documents
            .iter()
            .zip(&ids)
            .map(|(value, id)| Document::new(value).with_id(id))
    };
    if let Suggestion::Candidate(candidate) = expansive {
        let compiled = candidate.compile(SchemaOptions::default())?;
        println!(
            "Expansive candidate coverage:\n{}",
            serde_json::to_string_pretty(
                &compiled.evaluate(replay(), EvaluationOptions::default())
            )?
        );
    }
    let proposed = CompiledSchema::new(
        &json!({"type":"object","properties":{"size":{"type":"integer"}},"required":["size"]}),
        SchemaOptions::default(),
    )?;
    println!(
        "Supplied integer-only schema coverage:\n{}",
        serde_json::to_string_pretty(&proposed.evaluate(replay(), EvaluationOptions::default()))?
    );
    Ok(())
}
