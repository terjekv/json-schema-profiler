use schema_analysis::{Field, InferredSchema, Schema};
use serde::Serialize;

use crate::{DocumentEvidence, ProfilePath, Scope, statistics::Statistics};

/// Disjoint observed kinds: FractionalNumber excludes mathematically integral values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonKind {
    Null,
    Boolean,
    Integer,
    FractionalNumber,
    String,
    Array,
    Object,
}

impl JsonKind {
    pub(crate) const ALL: [Self; 7] = [
        Self::Null,
        Self::Boolean,
        Self::Integer,
        Self::FractionalNumber,
        Self::String,
        Self::Array,
        Self::Object,
    ];

    pub(crate) fn schema_type(self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Integer => "integer",
            Self::FractionalNumber => "number",
            Self::String => "string",
            Self::Array => "array",
            Self::Object => "object",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct TypeCounts {
    null: u64,
    boolean: u64,
    integer: u64,
    fractional_number: u64,
    string: u64,
    array: u64,
    object: u64,
}

impl TypeCounts {
    pub fn get(&self, kind: JsonKind) -> u64 {
        match kind {
            JsonKind::Null => self.null,
            JsonKind::Boolean => self.boolean,
            JsonKind::Integer => self.integer,
            JsonKind::FractionalNumber => self.fractional_number,
            JsonKind::String => self.string,
            JsonKind::Array => self.array,
            JsonKind::Object => self.object,
        }
    }

    pub fn kinds(&self) -> impl Iterator<Item = JsonKind> + '_ {
        JsonKind::ALL.into_iter().filter(|kind| self.get(*kind) > 0)
    }

    pub(crate) fn add(&mut self, kind: JsonKind, count: u64) {
        let value = match kind {
            JsonKind::Null => &mut self.null,
            JsonKind::Boolean => &mut self.boolean,
            JsonKind::Integer => &mut self.integer,
            JsonKind::FractionalNumber => &mut self.fractional_number,
            JsonKind::String => &mut self.string,
            JsonKind::Array => &mut self.array,
            JsonKind::Object => &mut self.object,
        };
        *value += count;
    }

    pub(crate) fn total(&self) -> u64 {
        self.kinds().map(|kind| self.get(kind)).sum()
    }

    pub(crate) fn families(&self) -> usize {
        self.kinds()
            .filter(|kind| {
                *kind != JsonKind::Null
                    && !(*kind == JsonKind::Integer && self.fractional_number > 0)
            })
            .count()
    }
}

/// Counts are occurrences within applicable parents, not distinct document counts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FieldProfile {
    path: ProfilePath,
    applicable_parents: u64,
    present: u64,
    types: TypeCounts,
    evidence: DocumentEvidence,
    #[serde(skip)]
    pub(crate) properties: Vec<String>,
}

impl FieldProfile {
    pub fn path(&self) -> &ProfilePath {
        &self.path
    }
    pub fn applicable_parents(&self) -> u64 {
        self.applicable_parents
    }
    pub fn present(&self) -> u64 {
        self.present
    }
    pub fn missing(&self) -> u64 {
        self.applicable_parents - self.present
    }
    pub fn types(&self) -> &TypeCounts {
        &self.types
    }
    pub fn evidence(&self) -> &DocumentEvidence {
        &self.evidence
    }
}

/// A completed profile; scalar values and upstream representation are not retained.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Profile {
    documents: u64,
    pub(crate) scope: Scope,
    fields: Vec<FieldProfile>,
}

impl Profile {
    pub fn documents(&self) -> u64 {
        self.documents
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn fields(&self) -> impl ExactSizeIterator<Item = &FieldProfile> {
        self.fields.iter()
    }
    pub fn field(&self, path: &ProfilePath) -> Option<&FieldProfile> {
        self.fields
            .binary_search_by(|field| field.path.cmp(path))
            .ok()
            .map(|index| &self.fields[index])
    }

    pub(crate) fn from_inferred(
        inferred: InferredSchema<Statistics>,
        documents: u64,
        scope: Scope,
        mut evidence: BTreeMap<ProfilePath, DocumentEvidence>,
    ) -> Self {
        let Schema::Sequence { field, .. } = inferred.schema else {
            unreachable!("adapter always wraps documents in a list")
        };
        let mut fields = Vec::new();
        collect_field(
            *field,
            ProfilePath::root(),
            documents,
            documents,
            &mut fields,
        );
        fields.sort_by(|a, b| a.path.cmp(&b.path));
        for field in &mut fields {
            field.evidence = evidence.remove(&field.path).unwrap_or_default();
        }
        Self {
            documents,
            scope,
            fields,
        }
    }
}

fn collect_field(
    field: Field<Statistics>,
    path: ProfilePath,
    parents: u64,
    present: u64,
    output: &mut Vec<FieldProfile>,
) {
    let mut profile = FieldProfile {
        path,
        applicable_parents: parents,
        present,
        types: TypeCounts::default(),
        evidence: DocumentEvidence::default(),
        properties: Vec::new(),
    };
    if let Some(schema) = field.schema {
        collect_schema(schema, &mut profile, output);
    }
    // Nested nulls do not reach Context::Null. Exact presence comes from key/item counts.
    let nulls = present
        .checked_sub(profile.types.total())
        .expect("upstream counts must not exceed admitted occurrences");
    profile.types.add(JsonKind::Null, nulls);
    output.push(profile);
}

fn collect_schema(
    schema: Schema<Statistics>,
    profile: &mut FieldProfile,
    output: &mut Vec<FieldProfile>,
) {
    match schema {
        Schema::Null(count) => profile.types.add(JsonKind::Null, count.0),
        Schema::Boolean(count) => profile.types.add(JsonKind::Boolean, count.0),
        Schema::Integer(count) => profile.types.add(JsonKind::Integer, count.0),
        Schema::Float(count) => profile.types.add(JsonKind::FractionalNumber, count.0),
        Schema::String(count) => profile.types.add(JsonKind::String, count.0),
        Schema::Bytes(_) => unreachable!("JSON adapter never produces bytes"),
        Schema::Sequence { field, context } => {
            profile.types.add(JsonKind::Array, context.count);
            collect_field(
                *field,
                profile.path.clone().each_item(),
                context.elements,
                context.elements,
                output,
            );
        }
        Schema::Struct { fields, context } => {
            profile.types.add(JsonKind::Object, context.count);
            for (key, field) in fields {
                let present = context.keys[&key];
                collect_field(
                    field,
                    profile.path.clone().property(&key),
                    context.count,
                    present,
                    output,
                );
                profile.properties.push(key);
            }
            profile.properties.sort();
        }
        Schema::Union { variants } => {
            for variant in variants {
                collect_schema(variant, profile, output);
            }
        }
    }
}
use std::collections::BTreeMap;
