use std::{error::Error, fmt};

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::{Frequency, JsonKind, Profile, ProfileError, ProfilePath, Scope, bounded::json_size};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Nullability {
    Allow,
    Reject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MixedTypes {
    Reject,
    Union,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    Observed,
    Optional,
    /// Require a property present in at least this fraction of applicable objects.
    /// Below 100%, this deliberately rejects observed objects missing the property.
    AtLeast(Frequency),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtraProperties {
    Allow,
    Deny,
}

/// Independent policy axes. Both presets preserve observed nulls and widen numeric kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct InferencePolicy {
    mixed_types: MixedTypes,
    presence: Presence,
    extra_properties: ExtraProperties,
    nullability: Nullability,
}

impl InferencePolicy {
    /// Allows mixed types and new properties, requiring consistently present fields.
    pub fn balanced() -> Self {
        Self::strict()
            .with_mixed_types(MixedTypes::Union)
            .with_extra_properties(ExtraProperties::Allow)
    }
    pub fn strict() -> Self {
        Self {
            mixed_types: MixedTypes::Reject,
            presence: Presence::Observed,
            extra_properties: ExtraProperties::Deny,
            nullability: Nullability::Allow,
        }
    }
    pub fn expansive() -> Self {
        Self {
            mixed_types: MixedTypes::Union,
            presence: Presence::Optional,
            extra_properties: ExtraProperties::Allow,
            nullability: Nullability::Allow,
        }
    }
    pub fn with_mixed_types(mut self, value: MixedTypes) -> Self {
        self.mixed_types = value;
        self
    }
    pub fn with_presence(mut self, value: Presence) -> Self {
        self.presence = value;
        self
    }
    pub fn with_extra_properties(mut self, value: ExtraProperties) -> Self {
        self.extra_properties = value;
        self
    }
    pub fn with_nullability(mut self, value: Nullability) -> Self {
        self.nullability = value;
        self
    }
}

/// Limits apply to compact JSON bytes and finding count, not temporary heap usage.
#[derive(Clone, Debug, Serialize)]
pub struct OutputLimits {
    schema_bytes: usize,
    report_bytes: usize,
    findings: usize,
}

impl Default for OutputLimits {
    fn default() -> Self {
        Self {
            schema_bytes: 1_000_000,
            report_bytes: 4_000_000,
            findings: 50_000,
        }
    }
}

impl OutputLimits {
    pub fn new(
        schema_bytes: usize,
        report_bytes: usize,
        findings: usize,
    ) -> Result<Self, ProfileError> {
        if schema_bytes == 0 || report_bytes == 0 {
            return Err(ProfileError::InvalidOptions(
                "output byte limits must be nonzero".into(),
            ));
        }
        Ok(Self {
            schema_bytes,
            report_bytes,
            findings,
        })
    }
}

#[derive(Clone, Debug, Serialize)]
struct PathPolicy {
    path: ProfilePath,
    policy: InferencePolicy,
}

/// Overrides apply to entire subtrees. Overlapping overrides are rejected.
#[derive(Clone, Debug, Serialize)]
pub struct SuggestionOptions {
    policy: InferencePolicy,
    overrides: Vec<PathPolicy>,
    limits: OutputLimits,
    minimum_documents: u64,
}

impl SuggestionOptions {
    pub fn new(policy: InferencePolicy) -> Self {
        Self {
            policy,
            overrides: Vec::new(),
            limits: OutputLimits::default(),
            minimum_documents: 1,
        }
    }
    pub fn with_output_limits(mut self, limits: OutputLimits) -> Self {
        self.limits = limits;
        self
    }
    /// Require at least this many distinct contributing records at every reported path.
    /// Repeated array items count once per document. A value of one preserves the
    /// default empty-array behavior; higher thresholds also require item evidence.
    ///
    /// # Errors
    /// Rejects zero; the default is one contributing record.
    pub fn with_minimum_documents(mut self, minimum: u64) -> Result<Self, ProfileError> {
        if minimum == 0 {
            return Err(ProfileError::InvalidOptions(
                "minimum documents must be nonzero".into(),
            ));
        }
        self.minimum_documents = minimum;
        Ok(self)
    }
    pub fn with_path_policy(
        mut self,
        path: ProfilePath,
        policy: InferencePolicy,
    ) -> Result<Self, InferenceError> {
        if self
            .overrides
            .iter()
            .any(|old| old.path.0.starts_with(&path.0) || path.0.starts_with(&old.path.0))
        {
            return Err(InferenceError::ConflictingPolicies(path));
        }
        if self.overrides.len() == 128 {
            return Err(InferenceError::LimitExceeded(OutputLimit::PolicyOverrides));
        }
        self.overrides.push(PathPolicy { path, policy });
        self.overrides.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(self)
    }
    fn at(&self, path: &ProfilePath) -> InferencePolicy {
        self.overrides
            .iter()
            .find(|entry| path.0.starts_with(&entry.path.0))
            .map_or(self.policy, |entry| entry.policy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputLimit {
    SchemaBytes,
    ReportBytes,
    Findings,
    PolicyOverrides,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InferenceError {
    UnknownPath(ProfilePath),
    ConflictingPolicies(ProfilePath),
    LimitExceeded(OutputLimit),
}

impl fmt::Display for InferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPath(path) => write!(f, "policy path was not profiled: {path}"),
            Self::ConflictingPolicies(path) => write!(f, "policy overrides overlap at {path}"),
            Self::LimitExceeded(limit) => write!(f, "inference output limit exceeded: {limit:?}"),
        }
    }
}
impl Error for InferenceError {}

struct Findings {
    values: Vec<Finding>,
    limit: usize,
}
impl Findings {
    fn push(&mut self, finding: Finding) -> Result<(), InferenceError> {
        if self.values.len() == self.limit {
            return Err(InferenceError::LimitExceeded(OutputLimit::Findings));
        }
        self.values.push(finding);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Finding {
    LowEvidence {
        path: ProfilePath,
        documents: u64,
        minimum: u64,
    },
    RequiredFromFrequency {
        path: ProfilePath,
        present: u64,
        applicable_parents: u64,
        threshold: Frequency,
    },
    MixedTypes {
        path: ProfilePath,
        types: Vec<JsonKind>,
    },
    ObservedNull {
        path: ProfilePath,
    },
    NullRejected {
        path: ProfilePath,
    },
    NumericWidening {
        path: ProfilePath,
    },
    OptionalProperty {
        path: ProfilePath,
    },
    OpenObject {
        path: ProfilePath,
    },
    UnconstrainedItems {
        path: ProfilePath,
    },
    UnobservedItems {
        path: ProfilePath,
    },
    UnobservedSelection {
        path: ProfilePath,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "status", content = "result", rename_all = "snake_case")]
pub enum Suggestion {
    Candidate(SchemaCandidate),
    Blocked(Vec<Finding>),
    Insufficient(Vec<Finding>),
}

/// An inferred candidate, without a validation or future-data guarantee.
#[derive(Clone, Debug, Serialize)]
pub struct SchemaCandidate {
    schema: Value,
    findings: Vec<Finding>,
    options: SuggestionOptions,
    documents: u64,
    scope: Scope,
}

impl SchemaCandidate {
    pub fn schema(&self) -> &Value {
        &self.schema
    }
    pub fn findings(&self) -> &[Finding] {
        &self.findings
    }
    pub fn policy(&self) -> InferencePolicy {
        self.options.policy
    }
    pub fn options(&self) -> &SuggestionOptions {
        &self.options
    }
    pub fn documents(&self) -> u64 {
        self.documents
    }
}

impl Profile {
    pub fn suggest(&self, policy: InferencePolicy) -> Result<Suggestion, InferenceError> {
        self.suggest_with(SuggestionOptions::new(policy))
    }

    pub fn suggest_with(&self, options: SuggestionOptions) -> Result<Suggestion, InferenceError> {
        let report_bytes = options.limits.report_bytes;
        let result = self.suggest_inner(options)?;
        json_size(&result, report_bytes)
            .ok_or(InferenceError::LimitExceeded(OutputLimit::ReportBytes))?;
        Ok(result)
    }

    fn suggest_inner(&self, options: SuggestionOptions) -> Result<Suggestion, InferenceError> {
        for entry in &options.overrides {
            if self.field(&entry.path).is_none() {
                return Err(InferenceError::UnknownPath(entry.path.clone()));
            }
        }
        let mut findings = Findings {
            values: Vec::new(),
            limit: options.limits.findings,
        };
        for path in self.scope.paths() {
            if self.field(path).is_none_or(|field| field.present() == 0) {
                findings.push(Finding::UnobservedSelection { path: path.clone() })?;
            }
        }
        if options.minimum_documents > 1 {
            for field in self.fields() {
                if field.evidence().documents() < options.minimum_documents {
                    findings.push(Finding::LowEvidence {
                        path: field.path().clone(),
                        documents: field.evidence().documents(),
                        minimum: options.minimum_documents,
                    })?;
                }
            }
        }
        if !findings.values.is_empty() {
            return Ok(Suggestion::Insufficient(findings.values));
        }
        let mut blocked = false;
        for field in self.fields() {
            let policy = options.at(field.path());
            if field.types().families() > 1 {
                findings.push(Finding::MixedTypes {
                    path: field.path().clone(),
                    types: field.types().kinds().collect(),
                })?;
                blocked |= policy.mixed_types == MixedTypes::Reject;
            }
            if field.types().get(JsonKind::Null) > 0 {
                findings.push(Finding::ObservedNull {
                    path: field.path().clone(),
                })?;
                if policy.nullability == Nullability::Reject {
                    findings.push(Finding::NullRejected {
                        path: field.path().clone(),
                    })?;
                    blocked = true;
                }
            }
            if field.types().get(JsonKind::Integer) > 0
                && field.types().get(JsonKind::FractionalNumber) > 0
            {
                findings.push(Finding::NumericWidening {
                    path: field.path().clone(),
                })?;
            }
        }
        if blocked {
            return Ok(Suggestion::Blocked(findings.values));
        }
        let mut schema = self.render(&ProfilePath::root(), &options, &mut findings)?;
        schema
            .as_object_mut()
            .expect("observed root generates a schema object")
            .insert(
                "$schema".into(),
                json!("https://json-schema.org/draft/2020-12/schema"),
            );
        json_size(&schema, options.limits.schema_bytes)
            .ok_or(InferenceError::LimitExceeded(OutputLimit::SchemaBytes))?;
        Ok(Suggestion::Candidate(SchemaCandidate {
            schema,
            findings: findings.values,
            options,
            documents: self.documents(),
            scope: self.scope.clone(),
        }))
    }

    fn render(
        &self,
        path: &ProfilePath,
        options: &SuggestionOptions,
        findings: &mut Findings,
    ) -> Result<Value, InferenceError> {
        let policy = options.at(path);
        let field = self.field(path).expect("profile children are indexed");
        if field.present() == 0 {
            if policy.nullability == Nullability::Reject {
                findings.push(Finding::UnobservedItems { path: path.clone() })?;
                return Ok(json!({"not": {"type": "null"}}));
            }
            findings.push(Finding::UnconstrainedItems { path: path.clone() })?;
            return Ok(Value::Bool(true));
        }
        let types: Vec<_> = field
            .types()
            .kinds()
            .filter(|kind| {
                !(*kind == JsonKind::Integer && field.types().get(JsonKind::FractionalNumber) > 0)
            })
            .map(JsonKind::schema_type)
            .collect();
        let mut schema = Map::new();
        schema.insert(
            "type".into(),
            if types.len() == 1 {
                json!(types[0])
            } else {
                json!(types)
            },
        );
        if field.types().get(JsonKind::Object) > 0 {
            let mut properties = Map::new();
            let mut required = Vec::new();
            for name in &field.properties {
                let child_path = path.clone().property(name);
                let child = self.field(&child_path).expect("property exists");
                let presence = options.at(&child_path).presence;
                let is_required = match presence {
                    Presence::Observed => child.missing() == 0,
                    Presence::Optional => false,
                    Presence::AtLeast(threshold) => {
                        threshold.reached(child.present(), child.applicable_parents())
                    }
                };
                if is_required {
                    required.push(name.clone());
                    if let Presence::AtLeast(threshold) = presence
                        && child.missing() > 0
                    {
                        findings.push(Finding::RequiredFromFrequency {
                            path: child_path.clone(),
                            present: child.present(),
                            applicable_parents: child.applicable_parents(),
                            threshold,
                        })?;
                    }
                } else {
                    findings.push(Finding::OptionalProperty {
                        path: child_path.clone(),
                    })?;
                }
                properties.insert(name.clone(), self.render(&child_path, options, findings)?);
            }
            schema.insert("properties".into(), Value::Object(properties));
            if !required.is_empty() {
                schema.insert("required".into(), json!(required));
            }
            let open = policy.extra_properties == ExtraProperties::Allow
                || !self.scope.includes_subtree(path);
            schema.insert("additionalProperties".into(), json!(open));
            if open {
                findings.push(Finding::OpenObject { path: path.clone() })?;
            }
        }
        if field.types().get(JsonKind::Array) > 0 {
            schema.insert(
                "items".into(),
                self.render(&path.clone().each_item(), options, findings)?,
            );
        }
        Ok(Value::Object(schema))
    }
}
