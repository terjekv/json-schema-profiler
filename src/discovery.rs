use serde::Serialize;

use crate::{
    Frequency, InferenceError, JsonKind, OutputLimit, Profile, ProfileError, ProfilePath,
    bounded::json_size,
};

/// Controls observational findings only; it never changes a schema policy.
#[derive(Clone, Debug, Serialize)]
pub struct DiscoveryOptions {
    minimum_documents: u64,
    rare_type: Frequency,
    sparse_property: Frequency,
    findings: usize,
    report_bytes: usize,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            minimum_documents: 5,
            rare_type: Frequency::percent(5).expect("valid percentage"),
            sparse_property: Frequency::percent(50).expect("valid percentage"),
            findings: 50_000,
            report_bytes: 4_000_000,
        }
    }
}

impl DiscoveryOptions {
    pub fn minimum_documents(&self) -> u64 {
        self.minimum_documents
    }

    pub fn rare_type_threshold(&self) -> Frequency {
        self.rare_type
    }

    pub fn sparse_property_threshold(&self) -> Frequency {
        self.sparse_property
    }

    /// # Errors
    /// Rejects zero. This is an evidence threshold, not a confidence estimate.
    pub fn with_minimum_documents(mut self, minimum: u64) -> Result<Self, ProfileError> {
        if minimum == 0 {
            return Err(ProfileError::InvalidOptions(
                "minimum documents must be nonzero".into(),
            ));
        }
        self.minimum_documents = minimum;
        Ok(self)
    }

    pub fn with_rare_type_threshold(mut self, threshold: Frequency) -> Self {
        self.rare_type = threshold;
        self
    }

    pub fn with_sparse_property_threshold(mut self, threshold: Frequency) -> Self {
        self.sparse_property = threshold;
        self
    }

    /// Limits fail explicitly, without returning partial discovery.
    ///
    /// # Errors
    /// Rejects a zero byte budget. Zero findings permits only a report with no findings.
    pub fn with_output_limits(
        mut self,
        findings: usize,
        bytes: usize,
    ) -> Result<Self, ProfileError> {
        if bytes == 0 {
            return Err(ProfileError::InvalidOptions(
                "discovery report bytes must be nonzero".into(),
            ));
        }
        self.findings = findings;
        self.report_bytes = bytes;
        Ok(self)
    }
}

/// Counts describe observations, not domain requirements or probabilities.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiscoveryFinding {
    LowEvidence {
        path: ProfilePath,
        documents: u64,
    },
    Unobserved {
        path: ProfilePath,
    },
    MixedTypes {
        path: ProfilePath,
        types: Vec<JsonKind>,
    },
    NullOnly {
        path: ProfilePath,
        occurrences: u64,
    },
    Nullable {
        path: ProfilePath,
        nulls: u64,
        present: u64,
    },
    SparseProperty {
        path: ProfilePath,
        present: u64,
        applicable_parents: u64,
    },
    /// A kind occurs in this many distinct contributing documents. These counts
    /// can overlap: one array document may contribute multiple kinds at a path.
    RareType {
        path: ProfilePath,
        value_type: JsonKind,
        documents: u64,
        contributing_documents: u64,
    },
}

impl DiscoveryFinding {
    pub fn path(&self) -> &ProfilePath {
        match self {
            Self::LowEvidence { path, .. }
            | Self::Unobserved { path }
            | Self::MixedTypes { path, .. }
            | Self::NullOnly { path, .. }
            | Self::Nullable { path, .. }
            | Self::SparseProperty { path, .. }
            | Self::RareType { path, .. } => path,
        }
    }
}

/// Completed, deterministic structural discovery with explicit threshold provenance.
/// Retains paths and counts, but no instance values or caller IDs.
#[derive(Clone, Debug, Serialize)]
pub struct Discovery {
    documents: u64,
    options: DiscoveryOptions,
    findings: Vec<DiscoveryFinding>,
}

impl Discovery {
    pub fn documents(&self) -> u64 {
        self.documents
    }
    pub fn options(&self) -> &DiscoveryOptions {
        &self.options
    }
    pub fn findings(&self) -> &[DiscoveryFinding] {
        &self.findings
    }
}

impl Profile {
    /// Finds sparse fields, rare kinds and weak evidence without inferring constraints.
    /// Paths and kinds use the same deterministic ordering as the profile.
    ///
    /// # Errors
    /// Returns an output-limit error rather than silently omitting findings.
    ///
    /// ```rust
    /// use json_schema_profiler::{DiscoveryOptions, Profiler};
    /// use serde_json::json;
    /// let mut profiler = Profiler::default();
    /// profiler.observe(&json!({"owner": null}))?;
    /// let discoveries = profiler.finish()?.discover(DiscoveryOptions::default())?;
    /// assert!(!discoveries.findings().is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn discover(&self, options: DiscoveryOptions) -> Result<Discovery, InferenceError> {
        let mut findings = Vec::new();
        let mut push = |finding| {
            if findings.len() == options.findings {
                return Err(InferenceError::LimitExceeded(OutputLimit::Findings));
            }
            findings.push(finding);
            Ok(())
        };
        // Selections that were never encountered have no field entry.
        for path in self.scope().paths() {
            if self.field(path).is_none() {
                push(DiscoveryFinding::Unobserved { path: path.clone() })?;
            }
        }
        for field in self.fields() {
            let path = field.path();
            let documents = field.evidence().documents();
            if field.present() == 0 {
                push(DiscoveryFinding::Unobserved { path: path.clone() })?;
                continue;
            }
            if documents < options.minimum_documents {
                push(DiscoveryFinding::LowEvidence {
                    path: path.clone(),
                    documents,
                })?;
            }
            if field.types().families() > 1 {
                push(DiscoveryFinding::MixedTypes {
                    path: path.clone(),
                    types: field.types().kinds().collect(),
                })?;
            }
            let nulls = field.types().get(JsonKind::Null);
            if nulls == field.present() {
                push(DiscoveryFinding::NullOnly {
                    path: path.clone(),
                    occurrences: nulls,
                })?;
            } else if nulls > 0 {
                push(DiscoveryFinding::Nullable {
                    path: path.clone(),
                    nulls,
                    present: field.present(),
                })?;
            }
            if field.missing() > 0
                && options
                    .sparse_property
                    .at_most(field.present(), field.applicable_parents())
            {
                push(DiscoveryFinding::SparseProperty {
                    path: path.clone(),
                    present: field.present(),
                    applicable_parents: field.applicable_parents(),
                })?;
            }
            for kind in field.types().kinds() {
                let count = field.evidence().types().get(kind);
                if options.rare_type.at_most(count, documents) {
                    push(DiscoveryFinding::RareType {
                        path: path.clone(),
                        value_type: kind,
                        documents: count,
                        contributing_documents: documents,
                    })?;
                }
            }
        }
        findings.sort_by(|a, b| a.path().cmp(b.path()));
        let report = Discovery {
            documents: self.documents(),
            findings,
            options,
        };
        json_size(&report, report.options.report_bytes)
            .ok_or(InferenceError::LimitExceeded(OutputLimit::ReportBytes))?;
        Ok(report)
    }
}
