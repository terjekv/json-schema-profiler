// Derived from schema_analysis 0.7.0, MIT OR Apache-2.0.
// See licenses/schema_analysis for attribution and maintained-source provenance.

use serde::{Deserialize, de::DeserializeSeed};

use crate::engine::{Coalesce, Schema, context::Context};

mod field;
mod schema;
mod schema_seed;

use schema::SchemaVisitor;
use schema_seed::SchemaVisitorSeed;

/**
[InferredSchema] is at the heart of this crate, it is a wrapper around [Schema] that interfaces
with the analysis code.
It implements both [Deserialize] and [DeserializeSeed] to allow for analysis both when no schema is
yet available and when we wish to expand an existing schema (for data across files, for example).
 */
pub struct InferredSchema<C: Context = ()> {
    /// Where the juicy info lays.
    pub schema: Schema<C>,
}
impl<C: Context> Coalesce for InferredSchema<C>
where
    Schema<C>: Coalesce,
{
    fn coalesce(&mut self, other: Self) {
        self.schema.coalesce(other.schema)
    }
}
impl<'de, C: Context + Default> Deserialize<'de> for InferredSchema<C> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let schema = deserializer.deserialize_any(SchemaVisitor::new())?;
        Ok(InferredSchema { schema })
    }
}
impl<'de, C: Context> DeserializeSeed<'de> for &mut InferredSchema<C>
where
    Schema<C>: Coalesce,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let visitor = SchemaVisitorSeed {
            schema: &mut self.schema,
        };
        deserializer.deserialize_any(visitor)?;
        Ok(())
    }
}

mod boilerplate {
    use std::fmt;

    use crate::engine::{Schema, context::Context};

    use super::InferredSchema;

    // Auto-generated, with bounds changed. (TODO: use perfect derive.)
    impl<C: Context> fmt::Debug for InferredSchema<C>
    where
        Schema<C>: fmt::Debug,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("InferredSchema")
                .field("schema", &self.schema)
                .finish()
        }
    }
    // Auto-generated, with bounds changed. (TODO: use perfect derive.)
    impl<C: Context> Clone for InferredSchema<C>
    where
        Schema<C>: Clone,
    {
        fn clone(&self) -> Self {
            Self {
                schema: self.schema.clone(),
            }
        }
    }
    // Auto-generated, with bounds changed. (TODO: use perfect derive.)
    impl<C: Context> PartialEq for InferredSchema<C>
    where
        Schema<C>: PartialEq,
    {
        fn eq(&self, other: &Self) -> bool {
            self.schema == other.schema
        }
    }
}
