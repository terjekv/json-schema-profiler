use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::validation::{FormatPolicy, SchemaError};

pub(crate) const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// Visit schema positions, including reference targets reached through annotations.
/// Literal objects in const/enum/default/examples are otherwise data, not schemas.
pub(crate) fn check(root: &Value, formats: FormatPolicy) -> Result<(), SchemaError> {
    let mut pending = vec![String::new()];
    let mut graph = BTreeMap::<String, Vec<String>>::new();
    while let Some(path) = pending.pop() {
        if graph.contains_key(&path) {
            continue;
        }
        let value = root
            .pointer(&path)
            .ok_or_else(|| SchemaError::InvalidReference { path: path.clone() })?;
        let mut children = Vec::new();
        if let Some(object) = value.as_object() {
            for (key, value) in object {
                let location = child(&path, key);
                match key.as_str() {
                    "$schema"
                        if value
                            .as_str()
                            .is_none_or(|uri| uri.trim_end_matches('#') != DIALECT) =>
                    {
                        return Err(SchemaError::UnsupportedDraft);
                    }
                    "$id" if !path.is_empty() => {
                        return Err(SchemaError::UnsupportedKeyword { path: location });
                    }
                    "$dynamicRef" | "$dynamicAnchor" | "$recursiveRef" | "$recursiveAnchor"
                    | "dependencies" | "additionalItems" => {
                        return Err(SchemaError::UnsupportedKeyword { path: location });
                    }
                    "$vocabulary" => {
                        let Some(vocabularies) = value.as_object() else {
                            return Err(SchemaError::InvalidSchema);
                        };
                        for (uri, required) in vocabularies {
                            if required.as_bool() != Some(true) {
                                continue;
                            }
                            let name = uri
                                .strip_prefix("https://json-schema.org/draft/2020-12/vocab/")
                                .unwrap_or("");
                            let supported = matches!(
                                name,
                                "core"
                                    | "applicator"
                                    | "unevaluated"
                                    | "validation"
                                    | "meta-data"
                                    | "format-annotation"
                                    | "content"
                            ) || (name == "format-assertion"
                                && formats == FormatPolicy::Assert);
                            if !supported {
                                return Err(SchemaError::UnsupportedVocabulary);
                            }
                        }
                    }
                    "$ref" => {
                        let reference = value.as_str().ok_or(SchemaError::InvalidSchema)?;
                        let target = pointer_fragment(reference).ok_or_else(|| {
                            SchemaError::InvalidReference {
                                path: location.clone(),
                            }
                        })?;
                        if root.pointer(&target).is_none() {
                            return Err(SchemaError::InvalidReference { path: location });
                        }
                        children.push(target);
                    }
                    "$defs" | "definitions" | "properties" | "patternProperties"
                    | "dependentSchemas" => {
                        if let Some(map) = value.as_object() {
                            children.extend(map.keys().map(|key| child(&location, key)));
                        }
                    }
                    "allOf" | "anyOf" | "oneOf" | "prefixItems" => {
                        if let Some(array) = value.as_array() {
                            children.extend(
                                (0..array.len()).map(|index| child(&location, &index.to_string())),
                            );
                        }
                    }
                    "additionalProperties"
                    | "unevaluatedProperties"
                    | "propertyNames"
                    | "items"
                    | "unevaluatedItems"
                    | "contains"
                    | "not"
                    | "if"
                    | "then"
                    | "else"
                    | "contentSchema" => children.push(location),
                    _ => (),
                }
            }
        } else if !value.is_boolean() {
            return Err(SchemaError::InvalidSchema);
        }
        pending.extend(children.iter().cloned());
        graph.insert(path, children);
    }
    // Reject cycles and overly deep expansion before entering the recursive compiler.
    visit("", &graph, &mut BTreeSet::new(), &mut BTreeMap::new())?;
    Ok(())
}

fn visit(
    path: &str,
    graph: &BTreeMap<String, Vec<String>>,
    visiting: &mut BTreeSet<String>,
    depths: &mut BTreeMap<String, usize>,
) -> Result<usize, SchemaError> {
    if let Some(depth) = depths.get(path) {
        return Ok(*depth);
    }
    if visiting.len() == 128 || !visiting.insert(path.to_owned()) {
        return Err(SchemaError::RecursiveReference);
    }
    let mut depth = 1;
    for child in &graph[path] {
        depth = depth.max(1 + visit(child, graph, visiting, depths)?);
    }
    visiting.remove(path);
    if depth > 128 {
        return Err(SchemaError::RecursiveReference);
    }
    depths.insert(path.to_owned(), depth);
    Ok(depth)
}

fn child(parent: &str, key: &str) -> String {
    format!("{parent}/{}", key.replace('~', "~0").replace('/', "~1"))
}

fn pointer_fragment(reference: &str) -> Option<String> {
    let fragment = reference.strip_prefix('#')?;
    let mut bytes = Vec::with_capacity(fragment.len());
    let mut iter = fragment.bytes();
    while let Some(byte) = iter.next() {
        bytes.push(if byte == b'%' {
            let high = char::from(iter.next()?).to_digit(16)?;
            let low = char::from(iter.next()?).to_digit(16)?;
            (high * 16 + low) as u8
        } else {
            byte
        });
    }
    let decoded = String::from_utf8(bytes).ok()?;
    if !decoded.is_empty() && !decoded.starts_with('/') {
        return None;
    }
    // Value::pointer is lenient about malformed ~ escapes; reject them explicitly.
    let mut chars = decoded.chars();
    while let Some(ch) = chars.next() {
        if ch == '~' && !matches!(chars.next(), Some('0' | '1')) {
            return None;
        }
    }
    Some(decoded)
}
