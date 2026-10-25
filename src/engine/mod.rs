//! Private, maintained aggregation core derived from schema_analysis 0.7.0.
//! The object-presence optimization is included in every packaged build.
mod analysis;
pub(crate) mod context;
mod schema;
pub(crate) mod traits;
pub(crate) use analysis::InferredSchema;
pub(crate) use schema::{Field, Schema};
pub(crate) use traits::Coalesce;
#[cfg(test)]
mod tests;
