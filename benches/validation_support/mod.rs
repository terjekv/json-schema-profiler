use serde_json::{Value, json};

pub fn schema(workload: &str) -> Value {
    match workload {
        "structural" | "invalid" => json!({"type":"object","properties":{"id":{"type":"integer"},"name":{"type":"string"}},"required":["id","name"],"additionalProperties":false}),
        "references" => json!({"$defs":{"item":{"type":"integer","minimum":0}},"type":"array","items":{"$ref":"#/$defs/item"},"uniqueItems":true}),
        "conditional" => json!({"if":{"properties":{"kind":{"const":"a"}},"required":["kind"]},"then":{"required":["a"]},"else":{"required":["b"]}}),
        "decimal" => serde_json::from_str(r#"{"type":"number","multipleOf":0.01,"minimum":0,"maximum":99999999999999999999999999999}"#).unwrap(),
        _ => panic!("unknown validation fixture"),
    }
}

pub fn corpus(workload: &str, documents: usize) -> Vec<Value> {
    (0..documents)
        .map(|index| match workload {
            "structural" => json!({"id":index,"name":"synthetic"}),
            "invalid" => json!({"id":"wrong","extra":true}),
            "references" => json!((0..32).collect::<Vec<_>>()),
            "conditional" => {
                if index.is_multiple_of(2) {
                    json!({"kind":"a","a":index})
                } else {
                    json!({"kind":"b","b":index})
                }
            }
            "decimal" => serde_json::from_str(&format!("{index}.12")).unwrap(),
            _ => panic!("unknown validation fixture"),
        })
        .collect()
}
