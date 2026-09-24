//! Experimental, bounded corpus profiling using `schema_analysis` for structural merging.
//!
//! ```
//! use json_schema_profiler::{InferencePolicy, Profiler, Suggestion};
//! use serde_json::json;
//!
//! let mut profiler = Profiler::default();
//! profiler.observe(&json!({"size": 4}))?;
//! profiler.observe(&json!({"size": "4"}))?;
//! let profile = profiler.finish()?;
//! assert!(matches!(profile.suggest(InferencePolicy::strict())?, Suggestion::Blocked(_)));
//! assert!(matches!(profile.suggest(InferencePolicy::expansive())?, Suggestion::Candidate(_)));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Observations describe supplied documents. A suggested schema has not been validated
//! against a replay of those documents and is not a guarantee about future data.

#![doc = include_str!("../README.md")]

mod adapter;
mod bounded;
mod error;
mod evidence;
mod limits;
mod numbers;
mod path;
mod policy;
mod profile;
mod profiler;
mod schema_guard;
mod scope;
mod statistics;
mod validation;
mod value_limits;

pub use error::{LimitKind, ProfileError};
pub use evidence::{DocumentEvidence, DocumentId, EvidenceLimits, Witness};
pub use limits::{Limits, LimitsBuilder};
pub use path::{PathSegment, ProfilePath};
pub use policy::{
    ExtraProperties, Finding, InferenceError, InferencePolicy, MixedTypes, Nullability,
    OutputLimit, OutputLimits, Presence, SchemaCandidate, Suggestion, SuggestionOptions,
};
pub use profile::{FieldProfile, JsonKind, Profile, TypeCounts};
pub use profiler::{Profiler, ProfilerOptions};
pub use scope::Scope;
pub use validation::{
    CompiledSchema, Document, Evaluation, EvaluationOptions, EvaluationStatus, EvaluationStop,
    FormatPolicy, SchemaError, SchemaOptions, VerifiedCorpus, Violation,
};
pub use value_limits::{ValueLimit, ValueLimits};
