use std::{error::Error, fmt};

use jsonschema::{PatternOptions, Retrieve, Uri, Validator, error::ValidationErrorKind};
use serde::Serialize;
use serde_json::Value;

use crate::{
    DocumentId, ProfileError, SchemaCandidate, ValueLimit, ValueLimits, bounded::json_size,
    schema_guard,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FormatPolicy {
    #[default]
    Annotate,
    Assert,
}

#[derive(Clone, Debug)]
pub struct SchemaOptions {
    formats: FormatPolicy,
    limits: ValueLimits,
}

impl Default for SchemaOptions {
    fn default() -> Self {
        Self {
            formats: FormatPolicy::Annotate,
            limits: ValueLimits::schema_default(),
        }
    }
}
impl SchemaOptions {
    pub fn with_formats(mut self, formats: FormatPolicy) -> Self {
        self.formats = formats;
        self
    }
    pub fn with_limits(mut self, limits: ValueLimits) -> Self {
        self.limits = limits;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchemaError {
    InvalidSchema,
    UnsupportedDraft,
    UnsupportedKeyword { path: String },
    UnsupportedVocabulary,
    InvalidReference { path: String },
    RecursiveReference,
    LimitExceeded(ValueLimit),
    Compilation { keyword: String, path: String },
}
impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "schema compilation failed: {self:?}")
    }
}
impl Error for SchemaError {}

struct DenyRetrieval;
impl Retrieve for DenyRetrieval {
    fn retrieve(&self, _: &Uri<String>) -> Result<Value, Box<dyn Error + Send + Sync>> {
        Err("external schema retrieval is disabled".into())
    }
}

/// Validated, compiled, self-contained Draft 2020-12 schema. No network/file retrieval.
pub struct CompiledSchema {
    schema: Value,
    validator: Validator,
    formats: FormatPolicy,
}

impl CompiledSchema {
    pub fn new(schema: &Value, options: SchemaOptions) -> Result<Self, SchemaError> {
        options
            .limits
            .check(schema)
            .map_err(SchemaError::LimitExceeded)?;
        schema_guard::check(schema, options.formats)?;
        let validator = jsonschema::draft202012::options()
            .with_retriever(DenyRetrieval)
            .should_validate_formats(options.formats == FormatPolicy::Assert)
            .should_ignore_unknown_formats(false)
            .with_pattern_options(
                PatternOptions::regex()
                    .size_limit(1_000_000)
                    .dfa_size_limit(1_000_000),
            )
            .build(schema)
            .map_err(|error| SchemaError::Compilation {
                keyword: error.kind().keyword().to_owned(),
                path: error.schema_path().to_string(),
            })?;
        Ok(Self {
            schema: schema.clone(),
            validator,
            formats: options.formats,
        })
    }
    pub fn schema(&self) -> &Value {
        &self.schema
    }
    pub fn format_policy(&self) -> FormatPolicy {
        self.formats
    }

    /// Counts input records, including repeated IDs, and never retains instance values.
    /// Diagnostic truncation is independent of corpus completion.
    pub fn evaluate<'a>(
        &self,
        documents: impl IntoIterator<Item = Document<'a>>,
        options: EvaluationOptions,
    ) -> Evaluation {
        let mut report = Evaluation {
            formats: self.formats,
            ..Evaluation::default()
        };
        let mut diagnostic_bytes = 0;
        for (index, document) in documents.into_iter().enumerate() {
            if index == options.documents {
                report.status = EvaluationStatus::Incomplete(EvaluationStop::DocumentLimit);
                break;
            }
            if let Err(limit) = options.value_limits.check(document.value) {
                report.status = EvaluationStatus::Incomplete(EvaluationStop::ValueLimit {
                    document_index: index as u64,
                    limit,
                });
                break;
            }
            let collect = options.per_document > 0
                && report.diagnostics.len() < options.diagnostics
                && diagnostic_bytes < options.report_bytes - 512;
            let mut errors = collect.then(|| self.validator.iter_errors(document.value));
            // Preserve engine failures while avoiding error-collection work once
            // only a document's validity is needed.
            let first = match &mut errors {
                Some(errors) => errors.next(),
                None => self.validator.validate(document.value).err(),
            };
            let Some(first) = first else {
                report.processed += 1;
                report.valid += 1;
                continue;
            };
            if engine_failure(first.kind()) {
                report.status = EvaluationStatus::Incomplete(EvaluationStop::EngineFailure {
                    document_index: index as u64,
                });
                break;
            }
            report.processed += 1;
            report.invalid += 1;
            if !collect {
                report.diagnostics_truncated = true;
                continue;
            }
            for (count, error) in std::iter::once(first)
                .chain(errors.into_iter().flatten())
                .enumerate()
            {
                if engine_failure(error.kind()) {
                    report.diagnostics_truncated = true;
                    break;
                }
                if count == options.per_document || report.diagnostics.len() == options.diagnostics
                {
                    report.diagnostics_truncated = true;
                    break;
                }
                let diagnostic = Violation {
                    document_index: index as u64,
                    document_id: document.id.cloned(),
                    instance_path: error.instance_path().to_string(),
                    schema_path: error.schema_path().to_string(),
                    keyword: error.kind().keyword().to_owned(),
                };
                let available = options
                    .report_bytes
                    .saturating_sub(512)
                    .saturating_sub(diagnostic_bytes)
                    .saturating_sub(1);
                let Some(bytes) = json_size(&diagnostic, available) else {
                    report.diagnostics_truncated = true;
                    break;
                };
                diagnostic_bytes += bytes + 1;
                report.diagnostics.push(diagnostic);
            }
        }
        // Header/status overhead is reserved above; verify the complete encoded report too.
        if json_size(&report, options.report_bytes).is_none() {
            report.diagnostics.clear();
            report.diagnostics_truncated = true;
        }
        report
    }

    /// Proves acceptance of this supplied replay only, not identity with an earlier profile.
    pub fn verify<'a>(
        &self,
        documents: impl IntoIterator<Item = Document<'a>>,
        options: EvaluationOptions,
    ) -> Result<VerifiedCorpus, Evaluation> {
        let evaluation = self.evaluate(documents, options);
        if evaluation.all_valid() {
            Ok(VerifiedCorpus {
                schema: self.schema.clone(),
                evaluation,
            })
        } else {
            Err(evaluation)
        }
    }
}

impl SchemaCandidate {
    pub fn compile(&self, options: SchemaOptions) -> Result<CompiledSchema, SchemaError> {
        CompiledSchema::new(self.schema(), options)
    }
}

/// Borrowed replay input with an optional caller label.
#[derive(Clone, Copy)]
pub struct Document<'a> {
    value: &'a Value,
    id: Option<&'a DocumentId>,
}
impl<'a> Document<'a> {
    pub fn new(value: &'a Value) -> Self {
        Self { value, id: None }
    }
    pub fn with_id(mut self, id: &'a DocumentId) -> Self {
        self.id = Some(id);
        self
    }
}

#[derive(Clone, Debug)]
pub struct EvaluationOptions {
    documents: usize,
    diagnostics: usize,
    per_document: usize,
    report_bytes: usize,
    value_limits: ValueLimits,
}
impl Default for EvaluationOptions {
    fn default() -> Self {
        Self {
            documents: 1_000_000,
            diagnostics: 256,
            per_document: 8,
            report_bytes: 1_000_000,
            value_limits: ValueLimits::default(),
        }
    }
}
impl EvaluationOptions {
    pub fn with_document_limit(mut self, limit: usize) -> Self {
        self.documents = limit;
        self
    }
    pub fn with_diagnostic_limits(mut self, total: usize, per_document: usize) -> Self {
        self.diagnostics = total;
        self.per_document = per_document;
        self
    }
    pub fn with_value_limits(mut self, limits: ValueLimits) -> Self {
        self.value_limits = limits;
        self
    }
    pub fn with_report_bytes(mut self, bytes: usize) -> Result<Self, ProfileError> {
        if bytes < 512 {
            return Err(ProfileError::InvalidOptions(
                "evaluation reports require at least 512 bytes".into(),
            ));
        }
        self.report_bytes = bytes;
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Violation {
    document_index: u64,
    document_id: Option<DocumentId>,
    instance_path: String,
    schema_path: String,
    keyword: String,
}
impl Violation {
    pub fn document_index(&self) -> u64 {
        self.document_index
    }
    pub fn document_id(&self) -> Option<&DocumentId> {
        self.document_id.as_ref()
    }
    pub fn instance_path(&self) -> &str {
        &self.instance_path
    }
    pub fn schema_path(&self) -> &str {
        &self.schema_path
    }
    pub fn keyword(&self) -> &str {
        &self.keyword
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum EvaluationStop {
    DocumentLimit,
    ValueLimit {
        document_index: u64,
        limit: ValueLimit,
    },
    EngineFailure {
        document_index: u64,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(tag = "status", content = "detail", rename_all = "snake_case")]
pub enum EvaluationStatus {
    #[default]
    Complete,
    Incomplete(EvaluationStop),
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Evaluation {
    formats: FormatPolicy,
    processed: u64,
    valid: u64,
    invalid: u64,
    status: EvaluationStatus,
    diagnostics: Vec<Violation>,
    diagnostics_truncated: bool,
}
impl Evaluation {
    pub fn format_policy(&self) -> FormatPolicy {
        self.formats
    }
    pub fn processed(&self) -> u64 {
        self.processed
    }
    pub fn valid(&self) -> u64 {
        self.valid
    }
    pub fn invalid(&self) -> u64 {
        self.invalid
    }
    pub fn status(&self) -> &EvaluationStatus {
        &self.status
    }
    pub fn diagnostics(&self) -> &[Violation] {
        &self.diagnostics
    }
    pub fn diagnostics_truncated(&self) -> bool {
        self.diagnostics_truncated
    }
    pub fn all_valid(&self) -> bool {
        self.status == EvaluationStatus::Complete && self.processed > 0 && self.invalid == 0
    }
}

/// Evidence that every document of one nonempty, completely evaluated replay passed.
#[derive(Clone, Debug, Serialize)]
pub struct VerifiedCorpus {
    schema: Value,
    evaluation: Evaluation,
}
impl VerifiedCorpus {
    pub fn schema(&self) -> &Value {
        &self.schema
    }
    pub fn evaluation(&self) -> &Evaluation {
        &self.evaluation
    }
}

fn engine_failure(kind: &ValidationErrorKind) -> bool {
    match kind {
        ValidationErrorKind::BacktrackLimitExceeded { .. }
        | ValidationErrorKind::RegexEngineFailure { .. }
        | ValidationErrorKind::Referencing(_) => true,
        ValidationErrorKind::AnyOf { context }
        | ValidationErrorKind::OneOfMultipleValid { context }
        | ValidationErrorKind::OneOfNotValid { context } => context
            .iter()
            .flatten()
            .any(|error| engine_failure(error.kind())),
        ValidationErrorKind::PropertyNames { error } => engine_failure(error.kind()),
        _ => false,
    }
}
