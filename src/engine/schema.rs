// Derived from schema_analysis 0.7.0, MIT OR Apache-2.0.
// See licenses/schema_analysis for attribution and maintained-source provenance.

use std::mem;

use ordermap::{OrderMap, map::Entry};
use serde::{Deserialize, Serialize};

use crate::engine::{Coalesce, context::Context};

/// This enum is the core output of the analysis, it describes the structure of a document.
///
/// Each variant also contains [context](crate::engine::context) data that allows it to store information
/// about the values it has encountered.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Schema<C: Context = ()> {
    /// The Null variant is a special one that is only ever found when a document has a single
    /// null value at the root of the document.
    /// Null values in [Struct](Schema::Struct)s or [Sequence](Schema::Sequence)s are instead
    /// handled at the [Field] level, where it is more ergonomic.
    Null(C::Null),
    /// Represents a boolean value.
    Boolean(C::Boolean),
    /// Represents an integer value.
    Integer(C::Integer),
    /// Represents a floating point value.
    Float(C::Float),
    /// Represents a textual value.
    String(C::String),
    /// Represents a value of raw bytes.
    Bytes(C::Bytes),
    /// Represents a sequence of values described by a [Field].
    /// It assumes all values share the same schema.
    Sequence {
        /// The field is the structure shared by all the elements of the sequence.
        field: Box<Field<C>>,
        /// The context aggregates information about the sequence.
        /// It is passed the length of the sequence.
        context: C::Sequence,
    },
    /// Represents a [String]->[Field] mapping.
    ///
    /// Note: currently there is not a true map and only strings may be used as keys.
    Struct {
        /// Each [String] key gets assigned a [Field].
        /// Fields retain their first-seen order in an OrderMap.
        fields: OrderMap<String, Field<C>>,
        /// The context aggregates information about the struct.
        /// It is passed a vector of the key names.
        context: C::Struct,
    },
    /// Simply a vector of [Schema]s, it should never contain an Union or multiple instances of the
    /// same variant inside.
    ///
    /// Note: content needs to be a struct variant to work with `#[serde(tag = "type")]`.
    Union {
        /// A list of the possible schemas that were found.
        variants: Vec<Schema<C>>,
    },
    // Tuple(..),
    // Map(..),
}

/// A [Field] is a useful abstraction to record metadata that does not belong or would be unyieldy
/// to place into the [Schema] and to account for cases in which the existence of a [Field] might be
/// known, but nothing is known about its shape.
#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "Schema<C>: Serialize",
    deserialize = "Schema<C>: Deserialize<'de>"
))]
pub struct Field<C: Context = ()> {
    /// The status holds information on the the field, like whether it might be null or
    /// missing altogether. Duplicate fields are also recorded.
    #[serde(flatten)]
    pub status: FieldStatus,
    /// The inner Schema is optional because we might have no information on the shape of the field
    /// (like for an empty array).
    #[serde(flatten)]
    pub schema: Option<Schema<C>>,
}
impl<C: Context> Default for Field<C> {
    fn default() -> Self {
        Self {
            status: FieldStatus::default(),
            schema: None,
        }
    }
}

/// The FieldStatus keeps track of what kind of values a [Field] has been found to have.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct FieldStatus {
    /// The [Field] has been found to be [None] or of the unit type `()`.
    pub may_be_null: bool,
    /// The [Field] has been found to be a normal value, where normal means
    /// any valid value for the [Schema] associated with the [Field].
    pub may_be_normal: bool,
    /// The [Field] was found only on some [Struct](Schema::Struct)s or the
    /// [Sequence](Schema::Sequence) to which it belongs might be empty (if only empty sequences
    /// are found, then the [Schema] in the [Field] will also be [None]).
    pub may_be_missing: bool,
    /// Sometimes a field might appear more than once in the same [Struct](Schema::Struct).
    /// In that case all instances are considered, but this flag is also enabled.
    /// This is useful to spot suspicious data, but also to detect sequences in xml files.
    pub may_be_duplicate: bool,
}

//
// Schema implementations
//
impl<C: Context> Schema<C> {
    fn take(&mut self) -> Self {
        mem::replace(self, Self::Null(C::Null::default()))
    }
}
impl<C: Context> Coalesce for Schema<C> {
    fn coalesce(&mut self, other: Self) {
        use Schema::*;
        match (self, other) {
            (Boolean(s), Boolean(o)) => s.coalesce(o),
            (Integer(s), Integer(o)) => s.coalesce(o),
            (Float(s), Float(o)) => s.coalesce(o),
            (String(s), String(o)) => s.coalesce(o),
            (Bytes(s), Bytes(o)) => s.coalesce(o),

            (
                Sequence {
                    field: self_boxed,
                    context: self_agg,
                },
                Sequence {
                    field: other_boxed,
                    context: other_agg,
                },
            ) => {
                self_agg.coalesce(other_agg);
                self_boxed.coalesce(*other_boxed);
            }

            (
                Struct {
                    fields: self_fields,
                    context: self_agg,
                },
                Struct {
                    fields: other_fields,
                    context: other_agg,
                },
            ) => {
                self_agg.coalesce(other_agg);
                coalesce_struct_fields(self_fields, other_fields);
            }
            (
                Union {
                    variants: self_alternatives,
                },
                Union {
                    variants: other_alternatives,
                },
            ) => coalesce_unions(self_alternatives, other_alternatives),
            (
                Union {
                    variants: self_alternatives,
                },
                any_other,
            ) => coalesce_to_alternatives(self_alternatives, any_other),
            (
                any_self,
                Union {
                    variants: mut other_alternatives,
                },
            ) => {
                coalesce_to_alternatives(&mut other_alternatives, any_self.take());
                *any_self = Schema::Union {
                    variants: other_alternatives,
                };
            }

            (any_self, any_other) => {
                *any_self = Union {
                    variants: vec![any_self.take(), any_other],
                };
            }
        };
        return;

        fn coalesce_unions<C: Context>(selfs: &mut Vec<Schema<C>>, others: Vec<Schema<C>>) {
            for o in others {
                coalesce_to_alternatives(selfs, o);
            }
        }

        /// This function attempts to match the incomming schema against all the
        /// alternatives already present, and if it fails it pushes it to the vector as a
        /// new alternative.
        fn coalesce_to_alternatives<C: Context>(
            alternatives: &mut Vec<Schema<C>>,
            mut other: Schema<C>,
        ) {
            use Schema::*;
            for s in alternatives.iter_mut() {
                match (s, other) {
                    // Nested unions should never happen.
                    // It is the job of the root impl of Coalesce for Schema to guarantee this.
                    (Union { .. }, _) | (_, Union { .. }) => {
                        unreachable!("nested union")
                    }

                    // If they are the same, go ahead and coalesce!
                    (Boolean(s), Boolean(o)) => {
                        s.coalesce(o);
                        return;
                    }
                    (Integer(s), Integer(o)) => {
                        s.coalesce(o);
                        return;
                    }
                    (Float(s), Float(o)) => {
                        s.coalesce(o);
                        return;
                    }
                    (String(s), String(o)) => {
                        s.coalesce(o);
                        return;
                    }
                    (Bytes(s), Bytes(o)) => {
                        s.coalesce(o);
                        return;
                    }

                    (
                        Sequence {
                            field: self_boxed,
                            context: self_agg,
                        },
                        Sequence {
                            field: other_boxed,
                            context: other_agg,
                        },
                    ) => {
                        self_agg.coalesce(other_agg);
                        self_boxed.coalesce(*other_boxed);
                        return;
                    }

                    (
                        Struct {
                            fields: self_fields,
                            context: self_agg,
                        },
                        Struct {
                            fields: other_fields,
                            context: other_agg,
                        },
                    ) => {
                        self_agg.coalesce(other_agg);
                        coalesce_struct_fields(self_fields, other_fields);
                        return;
                    }

                    // If they don't match just continue ahead to the next one.
                    (_, caught_other) => {
                        other = caught_other;
                    }
                }
            }

            // If we were unable to find a match, push the schema to the alternatives:
            alternatives.push(other);
        }

        fn coalesce_struct_fields<C: Context>(
            self_fields: &mut OrderMap<std::string::String, Field<C>>,
            other_fields: OrderMap<std::string::String, Field<C>>,
        ) where
            Field<C>: Coalesce,
        {
            // Fields in self but not in other were missing from other's documents.
            for (name, field) in self_fields.iter_mut() {
                if !other_fields.contains_key(name) {
                    field.status.may_be_missing = true;
                }
            }
            for (name, other_field) in other_fields {
                match self_fields.entry(name) {
                    Entry::Occupied(mut entry) => {
                        entry.get_mut().coalesce(other_field);
                    }
                    // Fields in other but not in self were missing from self's documents.
                    Entry::Vacant(entry) => {
                        let mut field = other_field;
                        field.status.may_be_missing = true;
                        entry.insert(field);
                    }
                }
            }
        }
    }
}

//
// Field implementations
//
impl<C: Context> Field<C> {
    /// Returns a [Field] with the given [Schema] and default [FieldStatus].
    pub fn with_schema(schema: Schema<C>) -> Self {
        Self {
            status: FieldStatus::default(),
            schema: Some(schema),
        }
    }
}
impl<C: Context> Coalesce for Field<C>
where
    Schema<C>: Coalesce,
{
    fn coalesce(&mut self, other: Self) {
        self.status.coalesce(other.status);
        self.schema = match (self.schema.take(), other.schema) {
            (Some(mut s), Some(o)) => {
                s.coalesce(o);
                Some(s)
            }
            (Some(s), None) => Some(s),
            (None, Some(o)) => Some(o),
            (None, None) => None,
        }
    }
}
//
// FieldStatus implementations
//
impl FieldStatus {
    /// If the value passed is true, then the status will allow duplicates.
    /// Otherwise no changes are made.
    pub fn allow_duplicates(&mut self, is_duplicate: bool) {
        self.may_be_duplicate |= is_duplicate;
    }
}
impl Coalesce for FieldStatus {
    fn coalesce(&mut self, other: Self) {
        self.may_be_null |= other.may_be_null;
        self.may_be_normal |= other.may_be_normal;
        self.may_be_missing |= other.may_be_missing;
        self.may_be_duplicate |= other.may_be_duplicate;
    }
}

//
// Helper functions
//

mod boilerplate {
    use std::fmt;

    use crate::engine::context::Context;

    use super::{Field, Schema};

    // Auto-generated, with bounds changed. (TODO: use perfect derive.)
    impl<C: Context> fmt::Debug for Schema<C>
    where
        C::Null: fmt::Debug,
        C::Boolean: fmt::Debug,
        C::Integer: fmt::Debug,
        C::Float: fmt::Debug,
        C::String: fmt::Debug,
        C::Bytes: fmt::Debug,
        C::Sequence: fmt::Debug,
        C::Struct: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Null(arg0) => f.debug_tuple("Null").field(arg0).finish(),
                Self::Boolean(arg0) => f.debug_tuple("Boolean").field(arg0).finish(),
                Self::Integer(arg0) => f.debug_tuple("Integer").field(arg0).finish(),
                Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
                Self::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
                Self::Bytes(arg0) => f.debug_tuple("Bytes").field(arg0).finish(),
                Self::Sequence { field, context } => f
                    .debug_struct("Sequence")
                    .field("field", field)
                    .field("context", context)
                    .finish(),
                Self::Struct { fields, context } => f
                    .debug_struct("Struct")
                    .field("fields", fields)
                    .field("context", context)
                    .finish(),
                Self::Union { variants } => {
                    f.debug_struct("Union").field("variants", variants).finish()
                }
            }
        }
    }
    // Auto-generated, with bounds changed. (TODO: use perfect derive.)
    impl<C: Context> Clone for Schema<C>
    where
        C::Null: Clone,
        C::Boolean: Clone,
        C::Integer: Clone,
        C::Float: Clone,
        C::String: Clone,
        C::Bytes: Clone,
        C::Sequence: Clone,
        C::Struct: Clone,
    {
        fn clone(&self) -> Self {
            match self {
                Self::Null(arg0) => Self::Null(arg0.clone()),
                Self::Boolean(arg0) => Self::Boolean(arg0.clone()),
                Self::Integer(arg0) => Self::Integer(arg0.clone()),
                Self::Float(arg0) => Self::Float(arg0.clone()),
                Self::String(arg0) => Self::String(arg0.clone()),
                Self::Bytes(arg0) => Self::Bytes(arg0.clone()),
                Self::Sequence { field, context } => Self::Sequence {
                    field: field.clone(),
                    context: context.clone(),
                },
                Self::Struct { fields, context } => Self::Struct {
                    fields: fields.clone(),
                    context: context.clone(),
                },
                Self::Union { variants } => Self::Union {
                    variants: variants.clone(),
                },
            }
        }
    }
    // Auto-generated, with bounds changed. (TODO: use perfect derive.)
    impl<C: Context> PartialEq for Schema<C>
    where
        C::Null: PartialEq,
        C::Boolean: PartialEq,
        C::Integer: PartialEq,
        C::Float: PartialEq,
        C::String: PartialEq,
        C::Bytes: PartialEq,
        C::Sequence: PartialEq,
        C::Struct: PartialEq,
    {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Self::Null(l0), Self::Null(r0)) => l0 == r0,
                (Self::Boolean(l0), Self::Boolean(r0)) => l0 == r0,
                (Self::Integer(l0), Self::Integer(r0)) => l0 == r0,
                (Self::Float(l0), Self::Float(r0)) => l0 == r0,
                (Self::String(l0), Self::String(r0)) => l0 == r0,
                (Self::Bytes(l0), Self::Bytes(r0)) => l0 == r0,
                (
                    Self::Sequence {
                        field: l_field,
                        context: l_context,
                    },
                    Self::Sequence {
                        field: r_field,
                        context: r_context,
                    },
                ) => l_field == r_field && l_context == r_context,
                (
                    Self::Struct {
                        fields: l_fields,
                        context: l_context,
                    },
                    Self::Struct {
                        fields: r_fields,
                        context: r_context,
                    },
                ) => l_fields == r_fields && l_context == r_context,
                (
                    Self::Union {
                        variants: l_variants,
                    },
                    Self::Union {
                        variants: r_variants,
                    },
                ) => l_variants == r_variants,
                _ => false,
            }
        }
    }

    // Auto-generated, with bounds changed. (TODO: use perfect derive.)
    impl<C: Context> fmt::Debug for Field<C>
    where
        Schema<C>: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Field")
                .field("status", &self.status)
                .field("schema", &self.schema)
                .finish()
        }
    }
    // Auto-generated, with bounds changed. (TODO: use perfect derive.)
    impl<C: Context> Clone for Field<C>
    where
        Schema<C>: Clone,
    {
        fn clone(&self) -> Self {
            Self {
                status: self.status.clone(),
                schema: self.schema.clone(),
            }
        }
    }
    // Auto-generated, with bounds changed. (TODO: use perfect derive.)
    impl<C: Context> PartialEq for Field<C>
    where
        Schema<C>: PartialEq,
    {
        fn eq(&self, other: &Self) -> bool {
            self.status == other.status && self.schema == other.schema
        }
    }
}
