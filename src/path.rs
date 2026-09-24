use std::fmt;

use serde::Serialize;

use crate::ProfileError;

/// A property name is always literal; array traversal has its own segment.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PathSegment {
    Property(String),
    EachItem,
}

/// An unambiguous structural path. It is not a wildcard JSON Pointer.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ProfilePath(pub(crate) Vec<PathSegment>);

impl ProfilePath {
    pub fn root() -> Self {
        Self::default()
    }

    pub fn property(mut self, name: impl Into<String>) -> Self {
        self.0.push(PathSegment::Property(name.into()));
        self
    }

    pub fn each_item(mut self) -> Self {
        self.0.push(PathSegment::EachItem);
        self
    }

    pub fn segments(&self) -> &[PathSegment] {
        &self.0
    }

    /// Parse RFC 6901 escaping, interpreting every token as an object property.
    /// Numerical tokens and `*` remain literal names, not array selectors.
    pub fn from_property_pointer(pointer: &str) -> Result<Self, ProfileError> {
        if pointer.is_empty() {
            return Ok(Self::root());
        }
        let Some(tail) = pointer.strip_prefix('/') else {
            return Err(ProfileError::InvalidPath(
                "must be empty or start with /".into(),
            ));
        };
        let mut path = Self::root();
        for token in tail.split('/') {
            let mut chars = token.chars();
            let mut property = String::new();
            while let Some(ch) = chars.next() {
                property.push(match ch {
                    '~' => match chars.next() {
                        Some('0') => '~',
                        Some('1') => '/',
                        _ => {
                            return Err(ProfileError::InvalidPath(
                                "only ~0 and ~1 escapes are valid".into(),
                            ));
                        }
                    },
                    other => other,
                });
            }
            path.0.push(PathSegment::Property(property));
        }
        Ok(path)
    }
}

impl fmt::Display for ProfilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("$")?;
        for segment in &self.0 {
            match segment {
                PathSegment::Property(name) => write!(
                    f,
                    "[{}]",
                    serde_json::to_string(name).map_err(|_| fmt::Error)?
                )?,
                PathSegment::EachItem => f.write_str("[*]")?,
            }
        }
        Ok(())
    }
}
