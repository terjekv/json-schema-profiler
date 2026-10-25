use std::collections::BTreeMap;

use serde::Serialize;

use crate::{FieldProfile, Frequency, JsonKind, ProfileError, ProfilePath, Scope};

/// Opt-in retention of exact string values for enum detection.
/// Defaults: all profiled paths, 256 distinct values per path, 4,096 entries
/// globally and 256 KiB of UTF-8 value payload. A value at two paths uses two entries.
/// These are logical retention limits, not a process-memory ceiling.
#[derive(Clone, Debug)]
pub struct EnumCollection {
    scope: Scope,
    per_path: usize,
    values: usize,
    bytes: usize,
}

impl Default for EnumCollection {
    fn default() -> Self {
        Self {
            scope: Scope::default(),
            per_path: 256,
            values: 4_096,
            bytes: 256 * 1_024,
        }
    }
}

impl EnumCollection {
    /// Collect only strings inside these subtrees of the profiling scope.
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// # Errors
    /// Rejects zero limits. A per-path overflow invalidates that path's enum
    /// evidence; a global overflow invalidates all enum evidence. Profiling continues.
    pub fn with_limits(
        mut self,
        values_per_path: usize,
        total_values: usize,
        total_bytes: usize,
    ) -> Result<Self, ProfileError> {
        if [values_per_path, total_values, total_bytes].contains(&0) {
            return Err(ProfileError::InvalidOptions(
                "enum collection limits must be nonzero".into(),
            ));
        }
        self.per_path = values_per_path;
        self.values = total_values;
        self.bytes = total_bytes;
        Ok(self)
    }
}

/// Tunable string-enum recognition, separate from value collection and schema policy.
/// The ratio is distinct strings / distinct documents contributing a string.
/// Missing, null and other kinds do not contribute to the denominator. Repeated
/// array items count once per document; several distinct items can make the ratio
/// exceed one. Comparisons use exact integer arithmetic, with no outlier removal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct EnumDetectionOptions {
    max_distinct_ratio: Frequency,
    minimum_documents: u64,
    max_distinct_values: Option<usize>,
}

impl EnumDetectionOptions {
    /// Start with the supplied whole-percentage threshold, a minimum of 100
    /// contributing documents, and no absolute distinct-value ceiling.
    pub fn new(max_distinct_ratio: Frequency) -> Self {
        Self {
            max_distinct_ratio,
            minimum_documents: 100,
            max_distinct_values: None,
        }
    }

    /// # Errors
    /// Rejects zero. This threshold measures evidence, not statistical confidence.
    pub fn with_minimum_documents(mut self, value: u64) -> Result<Self, ProfileError> {
        if value == 0 {
            return Err(ProfileError::InvalidOptions(
                "minimum enum documents must be nonzero".into(),
            ));
        }
        self.minimum_documents = value;
        Ok(self)
    }

    /// # Errors
    /// Rejects a zero ceiling. `None` removes the absolute detection ceiling;
    /// collection and output limits still apply.
    pub fn with_max_distinct_values(mut self, value: Option<usize>) -> Result<Self, ProfileError> {
        if value == Some(0) {
            return Err(ProfileError::InvalidOptions(
                "maximum enum values must be nonzero".into(),
            ));
        }
        self.max_distinct_values = value;
        Ok(self)
    }

    pub fn max_distinct_ratio(&self) -> Frequency {
        self.max_distinct_ratio
    }

    pub fn minimum_documents(&self) -> u64 {
        self.minimum_documents
    }

    pub fn max_distinct_values(&self) -> Option<usize> {
        self.max_distinct_values
    }
}

/// One observed string, with occurrence and distinct-document counts.
/// Values are exact and unnormalized, and can contain sensitive information.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EnumValue {
    value: String,
    occurrences: u64,
    documents: u64,
}

impl EnumValue {
    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn occurrences(&self) -> u64 {
        self.occurrences
    }

    pub fn documents(&self) -> u64 {
        self.documents
    }
}

/// Exact string evidence, or an explicitly incomplete collection.
/// Incomplete collections expose no partial value list. Complete values are
/// sorted lexicographically. Opting in exposes values in Debug and serialized profiles.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EnumEvidence {
    complete: bool,
    values: Vec<EnumValue>,
}

impl EnumEvidence {
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    pub fn values(&self) -> &[EnumValue] {
        &self.values
    }

    /// Exact cardinality is unavailable after any collection loss.
    pub fn distinct_values(&self) -> Option<usize> {
        self.complete.then_some(self.values.len())
    }
}

/// Recognition outcome, in evaluation order. Ineligibility never proves free text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnumDetectionStatus {
    NoStrings,
    NotCollected,
    Incomplete,
    InsufficientDocuments,
    TooManyValues,
    RatioExceeded,
    Eligible,
}

/// Value-free recognition report. Eligibility covers the observed strings only;
/// it makes no claim about unseen values. Thresholds are retained as provenance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct EnumDetection {
    status: EnumDetectionStatus,
    documents: u64,
    distinct_values: Option<usize>,
    options: EnumDetectionOptions,
}

impl EnumDetection {
    pub fn status(&self) -> EnumDetectionStatus {
        self.status
    }

    pub fn documents(&self) -> u64 {
        self.documents
    }

    pub fn distinct_values(&self) -> Option<usize> {
        self.distinct_values
    }

    pub fn options(&self) -> &EnumDetectionOptions {
        &self.options
    }

    /// Approximate ratio for display, never used to decide eligibility.
    /// The exact numerator and denominator are available through the count accessors.
    pub fn distinct_ratio(&self) -> Option<f64> {
        self.distinct_values
            .filter(|_| self.documents > 0)
            .map(|values| values as f64 / self.documents as f64)
    }
}

impl FieldProfile {
    /// Reconsider enum eligibility without rereading documents or retaining more data.
    /// Values must have been collected through [`EnumCollection`] during profiling.
    /// Use [`Self::enum_evidence`] for the complete value set and per-value counts.
    pub fn detect_enum(&self, options: EnumDetectionOptions) -> EnumDetection {
        let documents = self.evidence().types().get(JsonKind::String);
        let distinct_values = self.enum_evidence().and_then(EnumEvidence::distinct_values);
        let status = if documents == 0 {
            EnumDetectionStatus::NoStrings
        } else if self.enum_evidence().is_none() {
            EnumDetectionStatus::NotCollected
        } else if let Some(count) = distinct_values {
            if documents < options.minimum_documents {
                EnumDetectionStatus::InsufficientDocuments
            } else if options.max_distinct_values.is_some_and(|max| count > max) {
                EnumDetectionStatus::TooManyValues
            } else if !options.max_distinct_ratio.at_most(count as u64, documents) {
                EnumDetectionStatus::RatioExceeded
            } else {
                EnumDetectionStatus::Eligible
            }
        } else {
            EnumDetectionStatus::Incomplete
        };
        EnumDetection {
            status,
            documents,
            distinct_values,
            options,
        }
    }
}

#[derive(Default)]
struct ValueRecord {
    occurrences: u64,
    documents: u64,
    last_document: u64,
}

#[derive(Default)]
struct EnumRecord {
    values: BTreeMap<String, ValueRecord>,
    bytes: usize,
    incomplete: bool,
}

pub(crate) struct EnumCollector {
    options: EnumCollection,
    records: BTreeMap<usize, EnumRecord>,
    values: usize,
    bytes: usize,
    incomplete: bool,
}

impl EnumCollector {
    pub(crate) fn new(options: EnumCollection) -> Self {
        Self {
            options,
            records: BTreeMap::new(),
            values: 0,
            bytes: 0,
            incomplete: false,
        }
    }

    pub(crate) fn observe(&mut self, index: usize, path: &ProfilePath, document: u64, value: &str) {
        if self.incomplete || !self.options.scope.includes_subtree(path) {
            return;
        }
        let record = self.records.entry(index).or_default();
        if record.incomplete {
            return;
        }
        if !record.values.contains_key(value) {
            if record.values.len() == self.options.per_path {
                self.values -= record.values.len();
                self.bytes -= record.bytes;
                record.values.clear();
                record.bytes = 0;
                record.incomplete = true;
                return;
            }
            if self.values == self.options.values || value.len() > self.options.bytes - self.bytes {
                self.records.clear();
                self.incomplete = true;
                return;
            }
            record
                .values
                .insert(value.to_owned(), ValueRecord::default());
            record.bytes += value.len();
            self.values += 1;
            self.bytes += value.len();
        }
        let entry = record.values.get_mut(value).expect("value just admitted");
        entry.occurrences += 1;
        if entry.last_document != document {
            entry.documents += 1;
            entry.last_document = document;
        }
    }

    pub(crate) fn finish(&mut self, index: usize, path: &ProfilePath) -> Option<Box<EnumEvidence>> {
        if !self.options.scope.includes_subtree(path) {
            return None;
        }
        let record = self.records.remove(&index).unwrap_or_default();
        Some(Box::new(EnumEvidence {
            complete: !self.incomplete && !record.incomplete,
            values: record
                .values
                .into_iter()
                .map(|(value, counts)| EnumValue {
                    value,
                    occurrences: counts.occurrences,
                    documents: counts.documents,
                })
                .collect(),
        }))
    }
}
