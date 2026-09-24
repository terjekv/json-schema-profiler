use std::{error::Error, fmt};

use serde::Serialize;

use crate::ProfilePath;

/// A logical resource bounded by profiling options.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitKind {
    Documents,
    DocumentNodes,
    Depth,
    ProfilePaths,
    ProfilePathBytes,
    ReportBytes,
}

/// Admission and configuration failures. Scalar input values are never included.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileError {
    InvalidOptions(String),
    InvalidPath(String),
    ConflictingSelection(ProfilePath),
    ScopeMismatch(ProfilePath),
    LimitExceeded(LimitKind),
    EmptyCorpus,
    Incomplete,
    Adapter(String),
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOptions(message) => write!(f, "invalid options: {message}"),
            Self::InvalidPath(message) => write!(f, "invalid property pointer: {message}"),
            Self::ConflictingSelection(path) => {
                write!(f, "incompatible object and array selections at {path}")
            }
            Self::ScopeMismatch(path) => write!(
                f,
                "selected path requires a different ancestor container at {path}"
            ),
            Self::LimitExceeded(kind) => write!(f, "profiling limit exceeded: {kind:?}"),
            Self::EmptyCorpus => f.write_str("cannot profile an empty corpus"),
            Self::Incomplete => f.write_str("profiler is incomplete after a failed observation"),
            Self::Adapter(message) => write!(f, "upstream adapter error: {message}"),
        }
    }
}

impl Error for ProfileError {}

impl serde::de::Error for ProfileError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::Adapter(message.to_string())
    }
}
