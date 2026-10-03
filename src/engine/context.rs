// Derived from schema_analysis 0.7.0, MIT OR Apache-2.0.
// See licenses/schema_analysis for attribution and maintained-source provenance.

use super::{Coalesce, traits::Aggregate};

/// Interface describing the custom analysis that will be run on each type
/// alongside the schema shape.
///
/// For no additional analysis, use `()`.
pub trait Context {
    /// The state for the analysis run on null values.
    type Null: Aggregate<()> + Coalesce + Default;
    /// The state for the analysis run on boolean values.
    type Boolean: Aggregate<bool> + Coalesce + Default;
    /// The state for the analysis run on integer values.
    type Integer: Aggregate<i128> + Coalesce + Default;
    /// The state for the analysis run on floating point values.
    type Float: Aggregate<f64> + Coalesce + Default;
    /// The state for the analysis run on strings.
    type String: Aggregate<str> + Coalesce + Default;
    /// The state for the analysis run on binary data.
    type Bytes: Aggregate<[u8]> + Coalesce + Default;
    /// The state for the analysis run on sequence values.
    type Sequence: Aggregate<usize> + Coalesce + Default;
    /// The state for the analysis run on struct values.
    type Struct: Aggregate<[String]> + Coalesce + Default;
}

impl Context for () {
    type Null = ();
    type Boolean = ();
    type Integer = ();
    type Float = ();
    type String = ();
    type Bytes = ();
    type Sequence = ();
    type Struct = ();
}
