use std::collections::BTreeMap;

use schema_analysis::{Coalesce, context::Context, traits::Aggregate};

// Counters cannot overflow: validated admission bounds total visited nodes by u64::MAX.
#[derive(Clone, Debug, Default)]
pub(crate) struct Count(pub(crate) u64);

impl<T: ?Sized> Aggregate<T> for Count {
    fn aggregate(&mut self, _: &T) {
        self.0 += 1;
    }
}

impl Coalesce for Count {
    fn coalesce(&mut self, other: Self) {
        self.0 += other.0;
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ObjectCount {
    pub(crate) count: u64,
    pub(crate) keys: BTreeMap<String, u64>,
}

impl Aggregate<[String]> for ObjectCount {
    fn aggregate(&mut self, keys: &[String]) {
        self.count += 1;
        for key in keys {
            *self.keys.entry(key.clone()).or_default() += 1;
        }
    }
}

impl Coalesce for ObjectCount {
    fn coalesce(&mut self, other: Self) {
        self.count += other.count;
        for (key, count) in other.keys {
            *self.keys.entry(key).or_default() += count;
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ArrayCount {
    pub(crate) count: u64,
    pub(crate) elements: u64,
}

impl Aggregate<usize> for ArrayCount {
    fn aggregate(&mut self, size: &usize) {
        self.count += 1;
        self.elements += *size as u64;
    }
}

impl Coalesce for ArrayCount {
    fn coalesce(&mut self, other: Self) {
        self.count += other.count;
        self.elements += other.elements;
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Statistics;

impl Context for Statistics {
    type Null = Count;
    type Boolean = Count;
    type Integer = Count;
    type Float = Count;
    type String = Count;
    type Bytes = Count;
    type Sequence = ArrayCount;
    type Struct = ObjectCount;
}
