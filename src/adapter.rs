use std::collections::BTreeMap;

use serde::{
    Deserializer,
    de::{DeserializeSeed, MapAccess, SeqAccess, Visitor, value::BorrowedStrDeserializer},
    forward_to_deserialize_any,
};
use serde_json::{Map, Value, map::Iter};

use crate::{
    DocumentEvidence, DocumentId, EvidenceLimits, JsonKind, LimitKind, Limits, PathSegment,
    ProfileError, ProfilePath,
    evidence::{EvidenceBudget, EvidenceRecord},
    numbers::is_integer,
    scope::Selection,
};

/// Admission indexes paths and document evidence; upstream owns occurrence/shape statistics.
pub(crate) struct Admission {
    pub(crate) limits: Limits,
    paths: Vec<AdmittedPath>,
    path_bytes: usize,
    document_nodes: usize,
    document: u64,
    document_id: Option<DocumentId>,
    evidence_budget: EvidenceBudget,
}

struct AdmittedPath {
    path: ProfilePath,
    bytes: usize,
    properties: BTreeMap<String, usize>,
    items: Option<usize>,
    evidence: EvidenceRecord,
}

impl Admission {
    pub(crate) fn new(limits: Limits, evidence: EvidenceLimits) -> Self {
        Self {
            limits,
            paths: vec![AdmittedPath {
                path: ProfilePath::root(),
                bytes: 0,
                properties: BTreeMap::new(),
                items: None,
                evidence: EvidenceRecord::default(),
            }],
            path_bytes: 0,
            document_nodes: 0,
            document: 0,
            document_id: None,
            evidence_budget: EvidenceBudget::new(evidence),
        }
    }

    pub(crate) fn begin_document(&mut self, document: u64, id: Option<&DocumentId>) {
        self.document_nodes = 0;
        self.document = document;
        self.document_id = id.cloned();
    }

    pub(crate) fn into_evidence(self) -> BTreeMap<ProfilePath, DocumentEvidence> {
        self.paths
            .into_iter()
            .map(|node| (node.path, node.evidence.evidence))
            .collect()
    }

    fn visit(&mut self, path: usize, kind: JsonKind) -> Result<(), ProfileError> {
        if self.document_nodes == self.limits.document_nodes {
            return Err(ProfileError::LimitExceeded(LimitKind::DocumentNodes));
        }
        self.document_nodes += 1;
        self.paths[path].evidence.observe(
            self.document,
            kind,
            self.document_id.as_ref(),
            &mut self.evidence_budget,
        );
        Ok(())
    }

    fn child(&mut self, parent: usize, name: Option<&str>) -> Result<usize, ProfileError> {
        let node = &self.paths[parent];
        let existing = name.map_or(node.items, |name| node.properties.get(name).copied());
        if let Some(index) = existing {
            return Ok(index);
        }
        if node.path.0.len() == self.limits.depth {
            return Err(ProfileError::LimitExceeded(LimitKind::Depth));
        }
        if self.paths.len() == self.limits.profile_paths {
            return Err(ProfileError::LimitExceeded(LimitKind::ProfilePaths));
        }
        // Check before allocating or asking upstream to allocate the property name.
        let extra = name.map_or(1, |name| name.len().saturating_add(1));
        let bytes = node.bytes.saturating_add(extra);
        if bytes > self.limits.profile_path_bytes - self.path_bytes {
            return Err(ProfileError::LimitExceeded(LimitKind::ProfilePathBytes));
        }
        let mut path = node.path.clone();
        path.0.push(name.map_or(PathSegment::EachItem, |name| {
            PathSegment::Property(name.to_owned())
        }));
        let index = self.paths.len();
        self.paths.push(AdmittedPath {
            path,
            bytes,
            properties: BTreeMap::new(),
            items: None,
            evidence: EvidenceRecord::default(),
        });
        if let Some(name) = name {
            self.paths[parent].properties.insert(name.to_owned(), index);
        } else {
            self.paths[parent].items = Some(index);
        }
        self.path_bytes += bytes;
        Ok(index)
    }
}

/// A virtual one-item list routes root nulls through upstream Field handling.
/// Upstream's generic Coalesce currently does not combine root Null variants.
pub(crate) struct Document<'a, 'b> {
    pub(crate) value: &'a Value,
    pub(crate) selection: &'a Selection,
    pub(crate) admission: &'b mut Admission,
}

impl<'de> Deserializer<'de> for Document<'de, '_> {
    type Error = ProfileError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(DocumentItem {
            value: Some(self.value),
            selection: self.selection,
            admission: self.admission,
        })
    }

    forward_to_deserialize_any! { bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any }
}

struct DocumentItem<'a, 'b> {
    value: Option<&'a Value>,
    selection: &'a Selection,
    admission: &'b mut Admission,
}

impl<'de> SeqAccess<'de> for DocumentItem<'de, '_> {
    type Error = ProfileError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        self.value
            .take()
            .map(|value| {
                seed.deserialize(Shape {
                    value,
                    path: 0,
                    selection: self.selection,
                    admission: self.admission,
                })
            })
            .transpose()
    }
}

struct Shape<'a, 'b> {
    value: &'a Value,
    path: usize,
    selection: &'a Selection,
    admission: &'b mut Admission,
}

impl<'de> Deserializer<'de> for Shape<'de, '_> {
    type Error = ProfileError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let kind = match self.value {
            Value::Null => JsonKind::Null,
            Value::Bool(_) => JsonKind::Boolean,
            Value::String(_) => JsonKind::String,
            Value::Number(number) if is_integer(number) => JsonKind::Integer,
            Value::Number(_) => JsonKind::FractionalNumber,
            Value::Array(_) => JsonKind::Array,
            Value::Object(_) => JsonKind::Object,
        };
        self.admission.visit(self.path, kind)?;
        if !self.selection.all {
            let correct = if self.selection.items.is_some() {
                self.value.is_array()
            } else {
                self.value.is_object()
            };
            if !correct {
                return Err(ProfileError::ScopeMismatch(
                    self.admission.paths[self.path].path.clone(),
                ));
            }
        }
        match self.value {
            Value::Null => visitor.visit_unit(),
            Value::Bool(_) => visitor.visit_bool(false),
            Value::String(_) => visitor.visit_borrowed_str(""),
            Value::Number(_) => {
                // Only kind information reaches the counters-only upstream context.
                // Markers avoid lossy conversion and serde_json's private Number map.
                if kind == JsonKind::Integer {
                    visitor.visit_i128(0)
                } else {
                    visitor.visit_f64(0.5)
                }
            }
            Value::Array(values) => visitor.visit_seq(Items {
                values: values.iter(),
                parent: self.path,
                selection: self.selection.item(),
                admission: self.admission,
            }),
            Value::Object(values) => visitor.visit_map(Properties::new(
                values,
                self.path,
                self.selection,
                self.admission,
            )),
        }
    }

    forward_to_deserialize_any! { bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any }
}

struct Items<'a, 'b> {
    values: std::slice::Iter<'a, Value>,
    parent: usize,
    selection: &'a Selection,
    admission: &'b mut Admission,
}

impl<'de> SeqAccess<'de> for Items<'de, '_> {
    type Error = ProfileError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        let Some(value) = self.values.next() else {
            return Ok(None);
        };
        let path = self.admission.child(self.parent, None)?;
        let result = seed.deserialize(Shape {
            value,
            path,
            selection: self.selection,
            admission: self.admission,
        });
        result.map(Some)
    }
}

struct Properties<'a, 'b> {
    values: Iter<'a>,
    pending: Option<(usize, &'a Value, &'a Selection)>,
    parent: usize,
    selection: &'a Selection,
    admission: &'b mut Admission,
}

impl<'a, 'b> Properties<'a, 'b> {
    fn new(
        values: &'a Map<String, Value>,
        parent: usize,
        selection: &'a Selection,
        admission: &'b mut Admission,
    ) -> Self {
        Self {
            values: values.iter(),
            pending: None,
            parent,
            selection,
            admission,
        }
    }
}

impl<'de> MapAccess<'de> for Properties<'de, '_> {
    type Error = ProfileError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        for (key, value) in self.values.by_ref() {
            if let Some(selection) = self.selection.property(key) {
                let path = self.admission.child(self.parent, Some(key))?;
                self.pending = Some((path, value, selection));
                return seed
                    .deserialize(BorrowedStrDeserializer::new(key))
                    .map(Some);
            }
        }
        Ok(None)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        let (path, value, selection) = self
            .pending
            .take()
            .expect("upstream requests a value after its key");
        seed.deserialize(Shape {
            value,
            path,
            selection,
            admission: self.admission,
        })
    }
}
