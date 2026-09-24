use serde::Serialize;
use serde_json::Value;

use crate::{ProfileError, bounded::json_size};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueLimit {
    Nodes,
    Depth,
    Bytes,
    NumberSize,
}

/// Preflight bounds for complete values supplied to the validator. The depth ceiling
/// is 128; numeric tokens are limited to 1,024 bytes and exponent magnitude 4,096.
#[derive(Clone, Debug)]
pub struct ValueLimits {
    nodes: usize,
    depth: usize,
    bytes: usize,
}

impl Default for ValueLimits {
    fn default() -> Self {
        Self {
            nodes: 1_000_000,
            depth: 128,
            bytes: 16_000_000,
        }
    }
}

impl ValueLimits {
    pub fn new(nodes: usize, depth: usize, bytes: usize) -> Result<Self, ProfileError> {
        if nodes == 0 || depth == 0 || depth > 128 || bytes == 0 {
            return Err(ProfileError::InvalidOptions(
                "value limits require nonzero nodes/bytes and depth 1–128".into(),
            ));
        }
        Ok(Self {
            nodes,
            depth,
            bytes,
        })
    }
    pub(crate) fn schema_default() -> Self {
        Self {
            nodes: 10_000,
            depth: 64,
            bytes: 1_000_000,
        }
    }

    pub(crate) fn check(&self, value: &Value) -> Result<(), ValueLimit> {
        self.visit(value, 0, &mut 0)?;
        json_size(value, self.bytes).ok_or(ValueLimit::Bytes)?;
        Ok(())
    }

    fn visit(&self, value: &Value, depth: usize, nodes: &mut usize) -> Result<(), ValueLimit> {
        if depth > self.depth {
            return Err(ValueLimit::Depth);
        }
        if *nodes == self.nodes {
            return Err(ValueLimit::Nodes);
        }
        *nodes += 1;
        match value {
            Value::Array(values) => {
                for value in values {
                    self.visit(value, depth + 1, nodes)?;
                }
            }
            Value::Object(values) => {
                for value in values.values() {
                    self.visit(value, depth + 1, nodes)?;
                }
            }
            Value::Number(number) => {
                let token = number.as_str();
                if token.len() > 1024 {
                    return Err(ValueLimit::NumberSize);
                }
                if let Some((_, exponent)) = token.split_once(['e', 'E']) {
                    let magnitude = exponent
                        .trim_start_matches(['+', '-'])
                        .parse::<u32>()
                        .map_err(|_| ValueLimit::NumberSize)?;
                    if magnitude > 4096 {
                        return Err(ValueLimit::NumberSize);
                    }
                }
            }
            _ => (),
        }
        Ok(())
    }
}
