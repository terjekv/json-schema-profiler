// Derived from schema_analysis 0.7.0, MIT OR Apache-2.0.
// See licenses/schema_analysis for attribution and maintained-source provenance.

pub trait Coalesce: Sized {
    fn coalesce(&mut self, other: Self);
}
impl Coalesce for () {
    fn coalesce(&mut self, _: Self) {}
}
pub trait Aggregate<V: ?Sized> {
    fn aggregate(&mut self, value: &V);
}
impl<T: ?Sized> Aggregate<T> for () {
    fn aggregate(&mut self, _: &T) {}
}
