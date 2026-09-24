use crate::ProfileError;

/// Validated logical admission limits. These are not an exact heap-byte quota.
#[derive(Clone, Debug)]
pub struct Limits {
    pub(crate) documents: usize,
    pub(crate) document_nodes: usize,
    pub(crate) depth: usize,
    pub(crate) profile_paths: usize,
    pub(crate) profile_path_bytes: usize,
    pub(crate) report_bytes: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            documents: 1_000_000,
            document_nodes: 1_000_000,
            depth: 64,
            profile_paths: 10_000,
            profile_path_bytes: 4_000_000,
            report_bytes: 8_000_000,
        }
    }
}

impl Limits {
    pub fn builder() -> LimitsBuilder {
        LimitsBuilder(Self::default())
    }
}

#[derive(Clone, Debug)]
pub struct LimitsBuilder(Limits);

impl LimitsBuilder {
    pub fn documents(mut self, value: usize) -> Self {
        self.0.documents = value;
        self
    }
    pub fn document_nodes(mut self, value: usize) -> Self {
        self.0.document_nodes = value;
        self
    }
    pub fn depth(mut self, value: usize) -> Self {
        self.0.depth = value;
        self
    }
    pub fn profile_paths(mut self, value: usize) -> Self {
        self.0.profile_paths = value;
        self
    }
    pub fn profile_path_bytes(mut self, value: usize) -> Self {
        self.0.profile_path_bytes = value;
        self
    }

    pub fn report_bytes(mut self, value: usize) -> Self {
        self.0.report_bytes = value;
        self
    }

    pub fn build(self) -> Result<Limits, ProfileError> {
        let limits = self.0;
        if [
            limits.documents,
            limits.document_nodes,
            limits.depth,
            limits.profile_paths,
            limits.profile_path_bytes,
            limits.report_bytes,
        ]
        .contains(&0)
        {
            return Err(ProfileError::InvalidOptions(
                "limits must be nonzero".into(),
            ));
        }
        if limits.depth > 128 {
            return Err(ProfileError::InvalidOptions(
                "recursive adapter depth must be at most 128".into(),
            ));
        }
        if (limits.documents as u64)
            .checked_mul(limits.document_nodes as u64)
            .is_none()
        {
            return Err(ProfileError::InvalidOptions(
                "document and node limits must fit u64 counters".into(),
            ));
        }
        Ok(limits)
    }
}
