use serde::de::{Error, Visitor};

use crate::{Schema, traits::Aggregate, traits::Coalesce};

use super::{
    Context,
    field::{InferredField, InferredFieldSeed},
    schema::SchemaVisitor,
};

// OrderMap indices stay stable while this visitor only appends fields. Keep the
// first 64 flags inline so small objects need no presence-tracking allocation
// or variable bit shifts. Wider objects use compact spill words.
struct SeenFields {
    first: [bool; 64],
    rest: Vec<u64>,
}

impl SeenFields {
    fn new(fields: usize) -> Self {
        Self {
            first: [false; 64],
            rest: vec![0; fields.saturating_sub(64).div_ceil(64)],
        }
    }

    fn insert(&mut self, index: usize) -> bool {
        if index < 64 {
            let duplicate = self.first[index];
            self.first[index] = true;
            return duplicate;
        }
        let offset = index / 64 - 1;
        if offset >= self.rest.len() {
            self.rest.resize(offset + 1, 0);
        }
        let word = &mut self.rest[offset];
        let bit = 1 << (index % 64);
        let duplicate = *word & bit != 0;
        *word |= bit;
        duplicate
    }

    fn contains(&self, index: usize) -> bool {
        if index < 64 {
            return self.first[index];
        }
        let word = self.rest.get(index / 64 - 1).copied().unwrap_or(0);
        word & (1 << (index % 64)) != 0
    }
}

pub(super) struct SchemaVisitorSeed<'s, C: Context> {
    pub(super) schema: &'s mut Schema<C>,
}
impl<'de, C: Context> Visitor<'de> for SchemaVisitorSeed<'_, C>
where
    Schema<C>: Coalesce,
{
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("anything")
    }

    fn visit_bool<E: Error>(mut self, value: bool) -> Result<Self::Value, E> {
        match &mut self.schema {
            // The schema matches
            Schema::Boolean(aggregators) => aggregators.aggregate(&value),
            // Extend a different schema
            schema => {
                let new_schema = SchemaVisitor::new().visit_bool(value)?;

                schema.coalesce(new_schema);
            }
        }
        Ok(())
    }
    fn visit_i128<E: Error>(mut self, value: i128) -> Result<Self::Value, E> {
        match &mut self.schema {
            // The schema matches
            Schema::Integer(aggregators) => aggregators.aggregate(&value),
            // Extend a different schema
            schema => {
                let new_schema = SchemaVisitor::new().visit_i128(value)?;

                schema.coalesce(new_schema);
            }
        }
        Ok(())
    }
    fn visit_f64<E: Error>(mut self, value: f64) -> Result<Self::Value, E> {
        match &mut self.schema {
            // The schema matches
            Schema::Float(aggregators) => aggregators.aggregate(&value),
            // Extend a different schema
            schema => {
                let new_schema = SchemaVisitor::new().visit_f64(value)?;

                schema.coalesce(new_schema);
            }
        }
        Ok(())
    }
    fn visit_borrowed_str<E: Error>(mut self, value: &'de str) -> Result<Self::Value, E> {
        match &mut self.schema {
            // The schema matches
            Schema::String(aggregators) => aggregators.aggregate(value),
            // Extend a different schema
            schema => {
                let new_schema = SchemaVisitor::new().visit_borrowed_str(value)?;

                schema.coalesce(new_schema);
            }
        }
        Ok(())
    }
    fn visit_borrowed_bytes<E: Error>(mut self, value: &'de [u8]) -> Result<Self::Value, E> {
        match &mut self.schema {
            // The schema matches
            Schema::Bytes(aggregators) => aggregators.aggregate(value),
            // Extend a different schema
            schema => {
                let new_schema = SchemaVisitor::new().visit_borrowed_bytes(value)?;

                schema.coalesce(new_schema);
            }
        }
        Ok(())
    }

    fn visit_i8<E: Error>(self, value: i8) -> Result<Self::Value, E> {
        self.visit_i128(value.into())
    }
    fn visit_i16<E: Error>(self, value: i16) -> Result<Self::Value, E> {
        self.visit_i128(value.into())
    }
    fn visit_i32<E: Error>(self, value: i32) -> Result<Self::Value, E> {
        self.visit_i128(value.into())
    }
    fn visit_i64<E: Error>(self, value: i64) -> Result<Self::Value, E> {
        self.visit_i128(value.into())
    }
    fn visit_u8<E: Error>(self, value: u8) -> Result<Self::Value, E> {
        self.visit_i128(value.into())
    }
    fn visit_u16<E: Error>(self, value: u16) -> Result<Self::Value, E> {
        self.visit_i128(value.into())
    }
    fn visit_u32<E: Error>(self, value: u32) -> Result<Self::Value, E> {
        self.visit_i128(value.into())
    }
    fn visit_u64<E: Error>(self, value: u64) -> Result<Self::Value, E> {
        self.visit_i128(value.into())
    }
    fn visit_u128<E: Error>(self, value: u128) -> Result<Self::Value, E> {
        let as_i128 = std::convert::TryInto::try_into(value)
            .map_err(|_| E::custom("u128 value too large to fit into a i138"))?;
        self.visit_i128(as_i128)
    }

    fn visit_f32<E: Error>(self, value: f32) -> Result<Self::Value, E> {
        self.visit_f64(value.into())
    }

    fn visit_char<E: Error>(self, value: char) -> Result<Self::Value, E> {
        self.visit_string(value.into())
    }
    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        self.visit_borrowed_str(value)
    }
    fn visit_string<E: Error>(self, value: String) -> Result<Self::Value, E> {
        self.visit_borrowed_str(&value)
    }

    fn visit_bytes<E: Error>(self, value: &[u8]) -> Result<Self::Value, E> {
        self.visit_borrowed_bytes(value)
    }
    fn visit_byte_buf<E: Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
        self.visit_borrowed_bytes(&value)
    }

    /// This method should only be called if the Null value is at the root of the document,
    /// because otherwise null values are handled by `Field`.
    fn visit_none<E: Error>(mut self) -> Result<Self::Value, E> {
        match &mut self.schema {
            // The schema matches
            Schema::Null(aggregators) => {
                aggregators.aggregate(&());
            }
            // Extend a different schema
            schema => {
                let new_schema = SchemaVisitor::new().visit_none()?;

                schema.coalesce(new_schema);
            }
        }
        Ok(())
    }
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)?;
        Ok(())
    }
    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        // serde_json calls this method for `null`.
        self.visit_none()
    }

    fn visit_newtype_struct<D>(self, _deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        unreachable!("newtype structs are a rust construct")
    }

    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut count = 0;
        match &mut self.schema {
            // The schema matches
            Schema::Sequence {
                field: boxed_field,
                context: aggregators,
            } => {
                let field = boxed_field.as_mut();

                while let Some(()) = seq.next_element_seed(InferredFieldSeed { field })? {
                    count += 1;
                }

                if count == 0 {
                    field.status.may_be_missing = true;
                }

                aggregators.aggregate(&count);
            }
            // Extend a different schema
            schema => {
                let sequence_schema = SchemaVisitor::new().visit_seq(seq)?;
                schema.coalesce(sequence_schema);
            }
        };
        Ok(())
    }

    fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut keys = Vec::new();
        match &mut self.schema {
            Schema::Struct {
                fields,
                context: aggregators,
            } => {
                let mut seen = SeenFields::new(fields.len());
                while let Some(key) = map.next_key::<String>()? {
                    match fields.get_full_mut(&key) {
                        Some((index, _, old_field)) => {
                            old_field.status.allow_duplicates(seen.insert(index));
                            map.next_value_seed(InferredFieldSeed { field: old_field })?;
                        }

                        None => {
                            let mut new_field = map.next_value_seed(InferredField::new())?;
                            // If we are adding it to an existing schema it means that it was
                            // missing when this schema was created.
                            new_field.status.may_be_missing = true;
                            seen.insert(fields.len());
                            fields.insert(key.clone(), new_field);
                        }
                    }

                    keys.push(key);
                }

                for (index, (_, f)) in fields.iter_mut().enumerate() {
                    if !seen.contains(index) {
                        f.status.may_be_missing = true;
                    }
                }

                aggregators.aggregate(&keys);
            }
            schema => {
                let sequence_schema = SchemaVisitor::new().visit_map(map)?;
                schema.coalesce(sequence_schema);
            }
        }
        Ok(())
    }

    fn visit_enum<A>(self, _data: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::EnumAccess<'de>,
    {
        unreachable!("enum types are usually not available from the format's side")
    }
}
