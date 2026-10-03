//! Regression contract for the bundled upstream object visitor.

use super::{Coalesce, InferredSchema, Schema, context::Context, traits::Aggregate};
use rstest::rstest;
use serde::de::DeserializeSeed;
use serde_json::json;

#[derive(Default)]
struct KeyLog(Vec<Vec<String>>);

impl Aggregate<[String]> for KeyLog {
    fn aggregate(&mut self, keys: &[String]) {
        self.0.push(keys.to_vec());
    }
}

impl Coalesce for KeyLog {
    fn coalesce(&mut self, other: Self) {
        self.0.extend(other.0);
    }
}

#[derive(Default)]
struct KeyContext;

impl Context for KeyContext {
    type Null = ();
    type Boolean = ();
    type Integer = ();
    type Float = ();
    type String = ();
    type Bytes = ();
    type Sequence = ();
    type Struct = KeyLog;
}

fn observe<C: Context>(inferred: &mut InferredSchema<C>, input: &str) {
    let mut deserializer = serde_json::Deserializer::from_str(input);
    inferred.deserialize(&mut deserializer).unwrap();
    deserializer.end().unwrap();
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(63)]
#[case(64)]
#[case(65)]
#[case(127)]
#[case(128)]
#[case(129)]
#[case(1024)]
fn reordered_object_keys_preserve_presence_and_first_seen_order(#[case] width: usize) {
    let keys: Vec<_> = (0..width).map(|index| format!("key_{index:04}")).collect();
    let object = |keys: Vec<&String>| {
        format!(
            "{{{}}}",
            keys.iter()
                .map(|key| format!("{}:null", json!(key)))
                .collect::<Vec<_>>()
                .join(",")
        )
    };
    let mut inferred: InferredSchema<()> =
        serde_json::from_str(&object(keys.iter().collect())).unwrap();
    observe(&mut inferred, &object(keys.iter().rev().collect()));
    let Schema::Struct { fields, .. } = &inferred.schema else {
        panic!("expected object schema");
    };
    assert_eq!(
        fields.keys().collect::<Vec<_>>(),
        keys.iter().collect::<Vec<_>>()
    );
    assert!(fields.values().all(|field| {
        field.status.may_be_null && !field.status.may_be_missing && !field.status.may_be_duplicate
    }));
}

#[rstest]
#[case(0)]
#[case(63)]
#[case(64)]
#[case(65)]
#[case(127)]
#[case(128)]
fn newly_appended_fields_detect_duplicates_and_retain_missing_status(#[case] width: usize) {
    let seed = json!(
        (0..width)
            .map(|i| (format!("key_{i}"), json!(1)))
            .collect::<serde_json::Map<_, _>>()
    );
    let mut inferred: InferredSchema<()> = serde_json::from_value(seed).unwrap();
    observe(&mut inferred, r#"{"new":null,"new":1}"#);
    observe(&mut inferred, r#"{"new":2}"#);
    let Schema::Struct { fields, .. } = &inferred.schema else {
        panic!("expected object schema");
    };
    assert_eq!(fields.len(), width + 1);
    let field = &fields["new"];
    assert!(field.status.may_be_duplicate);
    assert!(field.status.may_be_missing);
    assert!(field.status.may_be_null);
    assert!(field.status.may_be_normal);
    assert!(fields.values().all(|field| field.status.may_be_missing));
}

#[test]
fn existing_duplicates_and_ordered_context_input_are_preserved() {
    let mut inferred: InferredSchema<KeyContext> =
        serde_json::from_str(r#"{"z":1,"a":null}"#).unwrap();
    observe(
        &mut inferred,
        r#"{"new":false,"a":null,"z":2,"a":3,"new":true}"#,
    );
    let Schema::Struct { fields, context } = &inferred.schema else {
        panic!("expected object schema");
    };
    assert_eq!(
        fields.keys().map(String::as_str).collect::<Vec<_>>(),
        ["z", "a", "new"]
    );
    assert_eq!(
        context.0,
        [vec!["z", "a"], vec!["new", "a", "z", "a", "new"]]
    );
    assert!(fields["a"].status.may_be_duplicate);
    assert!(fields["new"].status.may_be_duplicate);
    assert!(!fields["z"].status.may_be_duplicate);
    assert!(!fields["a"].status.may_be_missing);
    assert!(!fields["z"].status.may_be_missing);
    assert!(fields["new"].status.may_be_missing);
}

#[test]
fn missing_status_is_not_reset_when_a_field_reappears() {
    let mut inferred: InferredSchema<()> = serde_json::from_str(r#"{"a":1}"#).unwrap();
    observe(&mut inferred, "{}");
    observe(&mut inferred, r#"{"a":2}"#);
    let Schema::Struct { fields, .. } = &inferred.schema else {
        panic!("expected object schema");
    };
    assert!(fields["a"].status.may_be_missing);
    assert!(!fields["a"].status.may_be_duplicate);
}

#[rstest]
#[case(65)]
#[case(129)]
#[case(1025)]
fn growing_from_empty_preserves_duplicates_across_presence_words(#[case] width: usize) {
    let mut inferred: InferredSchema<()> = serde_json::from_str("{}").unwrap();
    let mut entries: Vec<_> = (0..width)
        .map(|index| format!("\"key_{index}\":null"))
        .collect();
    entries.push(entries[0].clone());
    entries.push(entries[width - 1].clone());
    observe(&mut inferred, &format!("{{{}}}", entries.join(",")));
    let Schema::Struct { fields, .. } = &inferred.schema else {
        panic!("expected object schema");
    };
    assert_eq!(fields.len(), width);
    for (index, field) in fields.values().enumerate() {
        assert_eq!(
            field.status.may_be_duplicate,
            index == 0 || index == width - 1
        );
        assert!(field.status.may_be_missing);
        assert!(field.status.may_be_null);
    }
}

#[rstest]
#[case(0)]
#[case(16)]
#[case(64)]
#[case(65)]
#[case(129)]
#[case(1025)]
fn object_aggregation_matches_the_published_upstream_release(#[case] width: usize) {
    let mut patched: InferredSchema<OracleContext> = serde_json::from_str("{}").unwrap();
    let mut original: schema_analysis::InferredSchema = serde_json::from_str("{}").unwrap();
    for record in 0..6 {
        let mut entries = Vec::new();
        for index in 0..width {
            if (index + record) % 3 == 0 {
                continue;
            }
            let key = json!(format!("/~field_{index}"));
            let value = match (index + record) % 4 {
                0 => "null",
                1 => "1",
                2 => "[true,false]",
                _ => "{\"nested\":\"synthetic\"}",
            };
            entries.push(format!("{key}:{value}"));
            if index % 5 == 0 {
                entries.push(format!("{key}:null"));
            }
        }
        if record % 2 == 0 {
            entries.reverse();
        }
        let input = format!("{{{}}}", entries.join(","));
        observe(&mut patched, &input);
        let mut deserializer = serde_json::Deserializer::from_str(&input);
        (&mut original).deserialize(&mut deserializer).unwrap();
        deserializer.end().unwrap();
        assert_eq!(
            serde_json::to_value(&patched.schema).unwrap(),
            serde_json::to_value(&original.schema).unwrap(),
            "aggregation diverged at record {record}"
        );
    }
}

// Reuse published sampling contexts only in tests to compare every serialized
// aggregation field; no sampling or upstream implementation type is in the API.
#[derive(Default, serde::Serialize)]
#[serde(transparent)]
struct Oracle<T>(T);
impl<T: schema_analysis::Coalesce> Coalesce for Oracle<T> {
    fn coalesce(&mut self, other: Self) {
        self.0.coalesce(other.0);
    }
}
impl<V: ?Sized, T: schema_analysis::traits::Aggregate<V>> Aggregate<V> for Oracle<T> {
    fn aggregate(&mut self, value: &V) {
        self.0.aggregate(value);
    }
}
#[derive(Default, serde::Serialize)]
struct OracleContext;
impl Context for OracleContext {
    type Null = Oracle<schema_analysis::context::NullContext>;
    type Boolean = Oracle<schema_analysis::context::BooleanContext>;
    type Integer = Oracle<schema_analysis::context::NumberContext<i128>>;
    type Float = Oracle<schema_analysis::context::NumberContext<f64>>;
    type String = Oracle<schema_analysis::context::StringContext>;
    type Bytes = Oracle<schema_analysis::context::BytesContext>;
    type Sequence = Oracle<schema_analysis::context::SequenceContext>;
    type Struct = Oracle<schema_analysis::context::MapStructContext>;
}
