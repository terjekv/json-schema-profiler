use crate::engine::InferredSchema;
use serde::{Deserialize, de::DeserializeSeed};
use serde_json::Value;

use crate::{
    DocumentId, EvidenceLimits, LimitKind, Limits, Profile, ProfileError, Scope,
    adapter::{Admission, Document},
    bounded::json_size,
    statistics::Statistics,
};

#[derive(Clone, Debug, Default)]
pub struct ProfilerOptions {
    scope: Scope,
    limits: Limits,
    evidence: EvidenceLimits,
}

impl ProfilerOptions {
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }
    pub fn with_evidence_limits(mut self, limits: EvidenceLimits) -> Self {
        self.evidence = limits;
        self
    }
}

/// Incrementally borrows documents; retains structure and counts, not scalar values.
/// Any failed observation makes this profiler terminally incomplete.
pub struct Profiler {
    scope: Scope,
    admission: Admission,
    state: State,
    documents: usize,
}

enum State {
    Active(Option<InferredSchema<Statistics>>),
    Failed,
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new(ProfilerOptions::default())
    }
}

impl Profiler {
    pub fn new(options: ProfilerOptions) -> Self {
        Self {
            scope: options.scope,
            admission: Admission::new(options.limits, options.evidence),
            state: State::Active(None),
            documents: 0,
        }
    }

    pub fn observe(&mut self, value: &Value) -> Result<(), ProfileError> {
        self.observe_inner(None, value)
    }

    pub fn observe_with_id(&mut self, id: &DocumentId, value: &Value) -> Result<(), ProfileError> {
        self.observe_inner(Some(id), value)
    }

    fn observe_inner(
        &mut self,
        id: Option<&DocumentId>,
        value: &Value,
    ) -> Result<(), ProfileError> {
        if matches!(self.state, State::Failed) {
            return Err(ProfileError::Incomplete);
        }
        if self.documents == self.admission.limits.documents {
            self.state = State::Failed;
            return Err(ProfileError::LimitExceeded(LimitKind::Documents));
        }
        self.admission.begin_document(self.documents as u64 + 1, id);
        let State::Active(inferred) = &mut self.state else {
            unreachable!()
        };
        let document = Document {
            value,
            selection: &self.scope.root,
            admission: &mut self.admission,
        };
        let result = match inferred {
            Some(inferred) => inferred.deserialize(document),
            None => InferredSchema::deserialize(document).map(|schema| {
                *inferred = Some(schema);
            }),
        };
        match result {
            Ok(()) => {
                self.documents += 1;
                Ok(())
            }
            Err(error) => {
                self.state = State::Failed;
                Err(error)
            }
        }
    }

    pub fn finish(self) -> Result<Profile, ProfileError> {
        match self.state {
            State::Failed => Err(ProfileError::Incomplete),
            State::Active(None) => Err(ProfileError::EmptyCorpus),
            State::Active(Some(inferred)) => {
                let limit = self.admission.limits.report_bytes;
                let profile = Profile::from_inferred(
                    inferred,
                    self.documents as u64,
                    self.scope,
                    self.admission.into_evidence(),
                );
                json_size(&profile, limit)
                    .ok_or(ProfileError::LimitExceeded(LimitKind::ReportBytes))?;
                Ok(profile)
            }
        }
    }
}
