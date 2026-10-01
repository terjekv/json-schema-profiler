use std::{error::Error, io, num::NonZeroU64, rc::Rc};

use json_schema_profiler::{
    CompiledSchema, Document, DocumentId, EvaluationOptions, EvidenceLimits, Limits, Profile,
    ProfilePath, Profiler, ProfilerOptions, SchemaCandidate, SchemaError, SchemaOptions, Scope,
    Suggestion, SuggestionOptions, VerificationError, VerifiedCorpus,
};
use serde_json::Value;

// Identity is process-local. Persistent consumers need a durable dataset ID and
// must make the revision comparison and schema write transactional.
#[derive(Clone, Debug)]
struct Revision {
    inventory: Rc<()>,
    sequence: NonZeroU64,
}

impl Revision {
    fn matches(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inventory, &other.inventory) && self.sequence == other.sequence
    }
}

pub struct Inventory {
    revision: Revision,
    rows: Vec<(DocumentId, String)>,
    applied_schema: Option<Value>,
}

impl Inventory {
    pub fn synthetic() -> Self {
        let rows = [
            ("node-a", r#"{"hardware":{"cores":4,"owner":null},"interfaces":[{"address":"192.0.2.1"},{"address":"192.0.2.2"}],"ignored":"private-a"}"#),
            ("node-b", r#"{"hardware":{"cores":"8"},"interfaces":[],"ignored":"private-b"}"#),
            ("node-c", r#"{"hardware":{"cores":16,"owner":"ops"},"interfaces":[{"address":"192.0.2.3"}],"ignored":"private-c"}"#),
        ]
        .into_iter()
        .map(|(id, json)| (DocumentId::new(id).unwrap(), json.to_owned()))
        .collect();
        Self {
            revision: Revision {
                inventory: Rc::new(()),
                sequence: NonZeroU64::MIN,
            },
            rows,
            applied_schema: None,
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.read_snapshot(None)
    }

    // Fault injection stands in for a storage read failure. It does not change
    // the inventory revision; a failed read of the same snapshot must be rejected.
    pub fn unreadable_snapshot(&self, index: usize) -> Snapshot {
        self.read_snapshot(Some(index))
    }

    fn read_snapshot(&self, fail_at: Option<usize>) -> Snapshot {
        let records = self
            .rows
            .iter()
            .enumerate()
            .map(|(index, (id, json))| {
                if fail_at == Some(index) {
                    return Err(io::Error::other("synthetic private read failure"));
                }
                serde_json::from_str(json)
                    .map(|value| Record {
                        id: id.clone(),
                        value,
                    })
                    .map_err(io::Error::other)
            })
            .collect();
        Snapshot {
            revision: self.revision.clone(),
            records,
        }
    }

    pub fn replace_row(&mut self, index: usize, json: &str) {
        // This demo has three fixed records. Every edit advances the revision,
        // even if it happens to preserve validity under an earlier schema.
        let next = self
            .revision
            .sequence
            .checked_add(1)
            .expect("revision exhausted");
        self.rows[index].1 = json.to_owned();
        self.revision.sequence = next;
    }

    pub fn apply(&mut self, approved: Approved) -> Result<(), StaleRevision> {
        if !self.revision.matches(&approved.revision) {
            return Err(StaleRevision);
        }
        self.applied_schema = Some(approved.evidence.schema().clone());
        Ok(())
    }

    pub fn applied_schema(&self) -> Option<&Value> {
        self.applied_schema.as_ref()
    }
}

struct Record {
    id: DocumentId,
    value: Value,
}

pub struct Snapshot {
    revision: Revision,
    records: Vec<Result<Record, io::Error>>,
}

impl Snapshot {
    pub fn replay(&self) -> impl Iterator<Item = Result<Document<'_>, &io::Error>> {
        self.records.iter().map(|record| {
            record
                .as_ref()
                .map(|record| Document::new(&record.value).with_id(&record.id))
        })
    }

    pub fn analyze(&self) -> Result<Analysis, Box<dyn Error>> {
        let options = ProfilerOptions::default()
            .with_scope(Scope::selected([
                ProfilePath::root().property("hardware"),
                ProfilePath::root().property("interfaces"),
            ])?)
            .with_limits(
                Limits::builder()
                    .documents(100)
                    .document_nodes(200)
                    .depth(10)
                    .profile_paths(100)
                    .profile_path_bytes(10_000)
                    .report_bytes(50_000)
                    .build()?,
            )
            .with_evidence_limits(EvidenceLimits::new(1, 32, 512));
        let mut profiler = Profiler::new(options);
        for record in &self.records {
            // Abort on source failure, without finalizing the partial profiler.
            // The original error remains owned by the snapshot.
            let record = record.as_ref().map_err(|_| "snapshot read/parse failed")?;
            profiler.observe_with_id(&record.id, &record.value)?;
        }
        Ok(Analysis {
            revision: self.revision.clone(),
            profile: profiler.finish()?,
        })
    }
}

pub struct Analysis {
    revision: Revision,
    profile: Profile,
}

impl Analysis {
    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    pub fn propose(&self, options: SuggestionOptions) -> Result<Review, Box<dyn Error>> {
        match self.profile.suggest_with(options)? {
            Suggestion::Candidate(candidate) => Ok(Review {
                revision: self.revision.clone(),
                candidate,
            }),
            Suggestion::Blocked(_) => Err("policy conflicts require review".into()),
            Suggestion::Insufficient(_) => Err("insufficient profile evidence".into()),
        }
    }
}

pub struct Review {
    revision: Revision,
    candidate: SchemaCandidate,
}

impl Review {
    pub fn candidate(&self) -> &SchemaCandidate {
        &self.candidate
    }

    pub fn compile(&self) -> Result<CompiledReview, SchemaError> {
        Ok(CompiledReview {
            revision: self.revision.clone(),
            schema: self.candidate.compile(SchemaOptions::default())?,
        })
    }
}

pub struct CompiledReview {
    revision: Revision,
    schema: CompiledSchema,
}

impl CompiledReview {
    pub fn schema(&self) -> &CompiledSchema {
        &self.schema
    }

    pub fn verify<'a>(
        &self,
        snapshot: &'a Snapshot,
        options: EvaluationOptions,
    ) -> Result<Approved, ApprovalFailure<'a>> {
        if !self.revision.matches(&snapshot.revision) {
            return Err(ApprovalFailure::Stale(StaleRevision));
        }
        self.schema
            .try_verify(snapshot.replay(), options)
            .map(|evidence| Approved {
                revision: self.revision.clone(),
                evidence,
            })
            .map_err(ApprovalFailure::Replay)
    }
}

#[derive(Debug)]
pub struct StaleRevision;

#[derive(Debug)]
pub enum ApprovalFailure<'a> {
    Stale(StaleRevision),
    Replay(VerificationError<&'a io::Error>),
}

#[derive(Debug)]
pub struct Approved {
    revision: Revision,
    evidence: VerifiedCorpus,
}

impl Approved {
    pub fn evidence(&self) -> &VerifiedCorpus {
        &self.evidence
    }
}
