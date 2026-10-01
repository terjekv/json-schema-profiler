use std::collections::BTreeMap;

use serde::Serialize;

use crate::{PathSegment, ProfileError, ProfilePath};

/// Selected subtrees with compatible ancestor container types.
/// Inference policy is supplied separately, including any per-path overrides.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Scope {
    paths: Vec<ProfilePath>,
    #[serde(skip)]
    pub(crate) root: Selection,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Selection {
    pub(crate) all: bool,
    pub(crate) properties: BTreeMap<String, Selection>,
    pub(crate) items: Option<Box<Selection>>,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            paths: vec![ProfilePath::root()],
            root: Selection {
                all: true,
                ..Selection::default()
            },
        }
    }
}

impl Scope {
    pub fn selected(paths: impl IntoIterator<Item = ProfilePath>) -> Result<Self, ProfileError> {
        let mut paths: Vec<_> = paths.into_iter().collect();
        if paths.is_empty() {
            return Err(ProfileError::InvalidOptions(
                "select at least one subtree".into(),
            ));
        }
        paths.sort_by(|a, b| a.0.len().cmp(&b.0.len()).then_with(|| a.cmp(b)));
        let mut scope = Self {
            paths: Vec::new(),
            root: Selection::default(),
        };
        for path in paths {
            if path.0.len() > 128 {
                return Err(ProfileError::InvalidPath(
                    "selection depth must be at most 128".into(),
                ));
            }
            if scope
                .paths
                .iter()
                .any(|parent| path.0.starts_with(&parent.0))
            {
                continue;
            }
            scope.root.insert(&path.0, &mut ProfilePath::root())?;
            scope.paths.push(path);
        }
        scope.paths.sort();
        Ok(scope)
    }

    pub fn paths(&self) -> &[ProfilePath] {
        &self.paths
    }

    pub(crate) fn includes_subtree(&self, path: &ProfilePath) -> bool {
        self.paths
            .iter()
            .any(|parent| path.0.starts_with(&parent.0))
    }
}

impl Selection {
    fn insert(
        &mut self,
        segments: &[PathSegment],
        cursor: &mut ProfilePath,
    ) -> Result<(), ProfileError> {
        let Some((head, rest)) = segments.split_first() else {
            self.all = true;
            return Ok(());
        };
        let child = match head {
            PathSegment::Property(name) => {
                if self.items.is_some() {
                    return Err(ProfileError::ConflictingSelection(cursor.clone()));
                }
                self.properties.entry(name.clone()).or_default()
            }
            PathSegment::EachItem => {
                if !self.properties.is_empty() {
                    return Err(ProfileError::ConflictingSelection(cursor.clone()));
                }
                self.items.get_or_insert_with(Default::default)
            }
        };
        cursor.0.push(head.clone());
        let result = child.insert(rest, cursor);
        cursor.0.pop();
        result
    }

    pub(crate) fn property(&self, key: &str) -> Option<&Self> {
        if self.all {
            Some(self)
        } else {
            self.properties.get(key)
        }
    }

    pub(crate) fn item(&self) -> &Self {
        if self.all {
            self
        } else {
            self.items
                .as_deref()
                .expect("array selection checked at boundary")
        }
    }
}
