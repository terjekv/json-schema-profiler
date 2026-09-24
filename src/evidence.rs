use serde::Serialize;

use crate::{JsonKind, ProfileError, TypeCounts};

/// Caller label, not a deduplication key. Repeated labels still identify separate inputs.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct DocumentId(String);

impl DocumentId {
    pub fn new(value: impl Into<String>) -> Result<Self, ProfileError> {
        let value = value.into();
        if value.is_empty() || value.len() > 256 {
            return Err(ProfileError::InvalidOptions(
                "document IDs must contain 1–256 UTF-8 bytes".into(),
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Retain first-seen labels per path and kind. Zero disables witness retention.
#[derive(Clone, Debug)]
pub struct EvidenceLimits {
    pub(crate) per_kind: usize,
    pub(crate) witnesses: usize,
    pub(crate) id_bytes: usize,
}

impl Default for EvidenceLimits {
    fn default() -> Self {
        Self {
            per_kind: 2,
            witnesses: 256,
            id_bytes: 16_384,
        }
    }
}

impl EvidenceLimits {
    pub fn new(per_kind: usize, witnesses: usize, id_bytes: usize) -> Self {
        Self {
            per_kind,
            witnesses,
            id_bytes,
        }
    }
    pub fn disabled() -> Self {
        Self::new(0, 0, 0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Witness {
    kind: JsonKind,
    document_id: DocumentId,
}

impl Witness {
    pub fn kind(&self) -> JsonKind {
        self.kind
    }
    pub fn document_id(&self) -> &DocumentId {
        &self.document_id
    }
}

/// Exact distinct-input counts, with separately bounded, order-dependent witnesses.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DocumentEvidence {
    documents: u64,
    types: TypeCounts,
    witnesses: Vec<Witness>,
    witnesses_truncated: bool,
}

impl DocumentEvidence {
    pub fn documents(&self) -> u64 {
        self.documents
    }
    pub fn types(&self) -> &TypeCounts {
        &self.types
    }
    pub fn witnesses(&self) -> &[Witness] {
        &self.witnesses
    }
    pub fn witnesses_truncated(&self) -> bool {
        self.witnesses_truncated
    }
}

#[derive(Default)]
pub(crate) struct EvidenceRecord {
    pub(crate) evidence: DocumentEvidence,
    last_document: u64,
    last_kind: [u64; 7],
    retained: [usize; 7],
}

pub(crate) struct EvidenceBudget {
    pub(crate) limits: EvidenceLimits,
    witnesses: usize,
    bytes: usize,
}

impl EvidenceBudget {
    pub(crate) fn new(limits: EvidenceLimits) -> Self {
        Self {
            limits,
            witnesses: 0,
            bytes: 0,
        }
    }
}

impl EvidenceRecord {
    pub(crate) fn observe(
        &mut self,
        document: u64,
        kind: JsonKind,
        id: Option<&DocumentId>,
        budget: &mut EvidenceBudget,
    ) {
        if self.last_document != document {
            self.last_document = document;
            self.evidence.documents += 1;
        }
        let index = kind as usize;
        if self.last_kind[index] == document {
            return;
        }
        self.last_kind[index] = document;
        self.evidence.types.add(kind, 1);
        let Some(id) = id else {
            return;
        };
        if self.retained[index] >= budget.limits.per_kind
            || budget.witnesses >= budget.limits.witnesses
            || id.0.len() > budget.limits.id_bytes - budget.bytes
        {
            self.evidence.witnesses_truncated = true;
            return;
        }
        self.evidence.witnesses.push(Witness {
            kind,
            document_id: id.clone(),
        });
        self.retained[index] += 1;
        budget.witnesses += 1;
        budget.bytes += id.0.len();
    }
}
