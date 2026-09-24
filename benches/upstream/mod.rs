use std::{collections::BTreeMap, hint::black_box};

use json_schema_profiler::Profiler;
use schema_analysis::{
    Coalesce, InferredSchema,
    context::{Context, DefaultContext},
    traits::Aggregate,
};
use serde::{
    Deserialize,
    de::{DeserializeSeed, value::SeqDeserializer},
};
use serde_json::Value;

// Independent, minimal example of the upstream extension API. This baseline intentionally
// has no path admission, selection, canonical report, or JSON numeric adapter.
#[derive(Default)]
struct Count(u64);
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

#[derive(Default)]
struct Keys {
    count: u64,
    keys: BTreeMap<String, u64>,
}
impl Aggregate<[String]> for Keys {
    fn aggregate(&mut self, keys: &[String]) {
        self.count += 1;
        for key in keys {
            *self.keys.entry(key.clone()).or_default() += 1;
        }
    }
}
impl Coalesce for Keys {
    fn coalesce(&mut self, other: Self) {
        self.count += other.count;
        for (key, count) in other.keys {
            *self.keys.entry(key).or_default() += count;
        }
    }
}

#[derive(Default)]
struct Elements {
    count: u64,
    elements: u64,
}
impl Aggregate<usize> for Elements {
    fn aggregate(&mut self, size: &usize) {
        self.count += 1;
        self.elements += *size as u64;
    }
}
impl Coalesce for Elements {
    fn coalesce(&mut self, other: Self) {
        self.count += other.count;
        self.elements += other.elements;
    }
}

#[derive(Default)]
struct Counters;
impl Context for Counters {
    type Null = Count;
    type Boolean = Count;
    type Integer = Count;
    type Float = Count;
    type String = Count;
    type Bytes = Count;
    type Sequence = Elements;
    type Struct = Keys;
}

fn upstream<C: Context + Default>(documents: &[Value]) {
    let mut inferred: Option<InferredSchema<C>> = None;
    for document in documents {
        // Same incremental one-document envelope used by the library, without its adapter.
        let input = SeqDeserializer::<_, serde_json::Error>::new(std::iter::once(document));
        if let Some(inferred) = &mut inferred {
            inferred.deserialize(input).unwrap();
        } else {
            inferred = Some(InferredSchema::deserialize(input).unwrap());
        }
    }
    black_box(inferred);
}

pub fn run(engine: &str, documents: &[Value]) {
    match engine {
        "minimal" => upstream::<()>(documents),
        "default" => upstream::<DefaultContext>(documents),
        "counts" => upstream::<Counters>(documents),
        "profiler" => {
            let mut profiler = Profiler::default();
            for document in documents {
                profiler.observe(document).unwrap();
            }
            black_box(profiler.finish().unwrap());
        }
        _ => panic!("unknown benchmark engine"),
    }
}
