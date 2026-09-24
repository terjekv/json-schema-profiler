use serde_json::{Map, Value, json};

/// Deterministic parsed inputs; fixture construction happens outside measurements.
pub fn corpus(workload: &str, documents: usize, width: usize) -> Vec<Value> {
    (0..documents).map(|index| match workload {
        "homogeneous" => json!({"id":index,"active":true,"name":"node","hardware":{"cores":8,"vendor":"example"}}),
        "sparse" => {
            let mut object = Map::new();
            for field in 0..width {
                if (index + field) % 3 != 0 {
                    object.insert(format!("field_{field:04}"), if (index + field) % 7 == 0 { Value::Null } else { json!(index) });
                }
            }
            Value::Object(object)
        }
        "mixed" => json!({"value": match index % 4 { 0 => json!(index), 1 => json!("text"), 2 => json!([null,1]), _ => json!({"nested":true}) }, "id":index}),
        "arrays" => json!({"id":index,"interfaces":(0..width).map(|item| json!({"address":"192.0.2.1","vlan":item,"up":true})).collect::<Vec<_>>()}),
        "wide" => Value::Object((0..width).map(|field| (format!("field_{field:04}"), json!(index))).collect()),
        "deep" => (0..width).fold(json!(index), |value, level| json!({format!("level_{level}"):value})),
        "dynamic" => json!({format!("key_{index:06}"):index}),
        "selection" => json!({"hardware":{"cores":8},"irrelevant":(0..width).map(|field| json!({"id":field,"payload":["a","b","c"]})).collect::<Vec<_>>()}),
        _ => panic!("unknown synthetic workload"),
    }).collect()
}
