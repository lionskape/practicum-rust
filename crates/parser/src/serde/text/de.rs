//! Text format Serde Deserializer implementation.
//!
//! Provides both buffered and streaming deserializers for YPBank text format.

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read},
};

use serde::de::{self, DeserializeSeed, EnumAccess, MapAccess, VariantAccess, Visitor};

use crate::serde::{Error, Result};

// ============================================================================
// Streaming Deserializer (recommended for files)
// ============================================================================

/// Streaming deserializer for YPBank text format.
///
/// Reads records separated by empty lines from a `BufRead` source.
pub struct StreamingTextDeserializer<R> {
    reader: R,
    /// Pre-parsed fields for current record
    fields: HashMap<String, String>,
    /// Current value being deserialized
    current_value: Option<String>,
}

impl<R: BufRead> StreamingTextDeserializer<R> {
    /// Creates a new streaming deserializer.
    pub fn new(reader: R) -> Self {
        Self { reader, fields: HashMap::new(), current_value: None }
    }

    /// Creates from any `Read` by wrapping in `BufReader`.
    pub fn from_reader(reader: impl Read) -> StreamingTextDeserializer<BufReader<impl Read>> {
        StreamingTextDeserializer::new(BufReader::new(reader))
    }

    /// Reads the next record (block of KEY: VALUE lines until empty line or EOF).
    ///
    /// Returns `Ok(true)` if a record was read, `Ok(false)` at EOF.
    pub fn read_record(&mut self) -> Result<bool> {
        self.fields.clear();
        let mut has_content = false;
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = self.reader.read_line(&mut line)?;

            // EOF
            if bytes_read == 0 {
                return Ok(has_content);
            }

            let trimmed = line.trim();

            // Empty line = end of record (if we have content)
            if trimmed.is_empty() {
                if has_content {
                    return Ok(true);
                }
                // Skip leading empty lines
                continue;
            }

            // Skip comments
            if trimmed.starts_with('#') {
                continue;
            }

            // Parse KEY: VALUE
            if let Some((key, value)) = trimmed.split_once(':') {
                self.fields.insert(key.trim().to_string(), value.trim().to_string());
                has_content = true;
            }
        }
    }

    /// Consumes the reader, returning it.
    pub fn into_inner(self) -> R {
        self.reader
    }

    fn set_current(&mut self, value: String) {
        self.current_value = Some(value);
    }

    fn take_current(&mut self) -> Result<String> {
        self.current_value.take().ok_or_else(|| Error::Message("No current value".to_string()))
    }
}

impl<'de, R: BufRead> de::Deserializer<'de> for &mut StreamingTextDeserializer<R> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("any"))
    }

    fn deserialize_bool<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("bool"))
    }

    fn deserialize_i8<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("i8"))
    }

    fn deserialize_i16<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("i16"))
    }

    fn deserialize_i32<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("i32"))
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let v: i64 = s
            .parse()
            .map_err(|_| Error::InvalidFieldFormat(format!("Cannot parse '{}' as i64", s)))?;
        visitor.visit_i64(v)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let v: u8 = s
            .parse()
            .map_err(|_| Error::InvalidFieldFormat(format!("Cannot parse '{}' as u8", s)))?;
        visitor.visit_u8(v)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let v: u16 = s
            .parse()
            .map_err(|_| Error::InvalidFieldFormat(format!("Cannot parse '{}' as u16", s)))?;
        visitor.visit_u16(v)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let v: u32 = s
            .parse()
            .map_err(|_| Error::InvalidFieldFormat(format!("Cannot parse '{}' as u32", s)))?;
        visitor.visit_u32(v)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let v: u64 = s
            .parse()
            .map_err(|_| Error::InvalidFieldFormat(format!("Cannot parse '{}' as u64", s)))?;
        visitor.visit_u64(v)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("f32"))
    }

    fn deserialize_f64<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("f64"))
    }

    fn deserialize_char<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("char"))
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let unquoted = unquote(&s);
        visitor.visit_string(unquoted.to_string())
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let unquoted = unquote(&s);
        visitor.visit_string(unquoted.to_string())
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("bytes"))
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("byte_buf"))
    }

    fn deserialize_option<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("option"))
    }

    fn deserialize_unit<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("unit"))
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value> {
        Err(Error::UnsupportedType("unit_struct"))
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value> {
        Err(Error::UnsupportedType("newtype_struct"))
    }

    fn deserialize_seq<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("seq"))
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("tuple"))
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value> {
        Err(Error::UnsupportedType("tuple_struct"))
    }

    fn deserialize_map<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("map"))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_map(StreamingTextMapAccess::new(self, fields))
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        let variant = self.take_current()?;
        visitor.visit_enum(StreamingTextEnumAccess { variant })
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::Message("deserialize_identifier should not be called directly".to_string()))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("ignored_any"))
    }
}

/// MapAccess for streaming text deserializer.
struct StreamingTextMapAccess<'a, R> {
    de: &'a mut StreamingTextDeserializer<R>,
    fields: &'static [&'static str],
    field_idx: usize,
}

impl<'a, R: BufRead> StreamingTextMapAccess<'a, R> {
    fn new(de: &'a mut StreamingTextDeserializer<R>, fields: &'static [&'static str]) -> Self {
        Self { de, fields, field_idx: 0 }
    }
}

impl<'de, R: BufRead> MapAccess<'de> for StreamingTextMapAccess<'_, R> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.field_idx >= self.fields.len() {
            return Ok(None);
        }

        let field_name = self.fields[self.field_idx];
        seed.deserialize(de::value::BorrowedStrDeserializer::new(field_name)).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        let field_name = self.fields[self.field_idx];
        self.field_idx += 1;

        let value = self
            .de
            .fields
            .get(field_name)
            .ok_or_else(|| Error::MissingField(field_name.to_string()))?
            .clone();

        self.de.set_current(value);
        seed.deserialize(&mut *self.de)
    }
}

/// EnumAccess for streaming text deserializer.
struct StreamingTextEnumAccess {
    variant: String,
}

impl<'de> EnumAccess<'de> for StreamingTextEnumAccess {
    type Error = Error;
    type Variant = StreamingTextVariantAccess;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        let val = seed.deserialize(de::value::StrDeserializer::<Error>::new(&self.variant))?;
        Ok((val, StreamingTextVariantAccess))
    }
}

struct StreamingTextVariantAccess;

impl<'de> VariantAccess<'de> for StreamingTextVariantAccess {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, _seed: T) -> Result<T::Value> {
        Err(Error::UnsupportedType("newtype_variant"))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("tuple_variant"))
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value> {
        Err(Error::UnsupportedType("struct_variant"))
    }
}

// ============================================================================
// Buffered Deserializer (for in-memory strings)
// ============================================================================

/// Deserializer for YPBank text format from a string slice.
///
/// Parses `KEY: VALUE` pairs from text input.
pub struct TextDeserializer<'de> {
    fields: HashMap<&'de str, &'de str>,
    current_value: Option<&'de str>,
}

impl<'de> TextDeserializer<'de> {
    /// Creates a new deserializer from text input.
    ///
    /// Parses all KEY: VALUE pairs upfront into a HashMap.
    pub fn new(input: &'de str) -> Result<Self> {
        let mut fields = HashMap::new();

        for line in input.lines() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Parse KEY: VALUE
            if let Some((key, value)) = trimmed.split_once(':') {
                fields.insert(key.trim(), value.trim());
            }
        }

        Ok(Self { fields, current_value: None })
    }

    fn set_current(&mut self, value: &'de str) {
        self.current_value = Some(value);
    }

    fn take_current(&mut self) -> Result<&'de str> {
        self.current_value.take().ok_or_else(|| Error::Message("No current value".to_string()))
    }
}

impl<'de> de::Deserializer<'de> for &mut TextDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("any"))
    }

    fn deserialize_bool<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("bool"))
    }

    fn deserialize_i8<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("i8"))
    }

    fn deserialize_i16<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("i16"))
    }

    fn deserialize_i32<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("i32"))
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let v: i64 = s
            .parse()
            .map_err(|_| Error::InvalidFieldFormat(format!("Cannot parse '{}' as i64", s)))?;
        visitor.visit_i64(v)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let v: u8 = s
            .parse()
            .map_err(|_| Error::InvalidFieldFormat(format!("Cannot parse '{}' as u8", s)))?;
        visitor.visit_u8(v)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let v: u16 = s
            .parse()
            .map_err(|_| Error::InvalidFieldFormat(format!("Cannot parse '{}' as u16", s)))?;
        visitor.visit_u16(v)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let v: u32 = s
            .parse()
            .map_err(|_| Error::InvalidFieldFormat(format!("Cannot parse '{}' as u32", s)))?;
        visitor.visit_u32(v)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let v: u64 = s
            .parse()
            .map_err(|_| Error::InvalidFieldFormat(format!("Cannot parse '{}' as u64", s)))?;
        visitor.visit_u64(v)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("f32"))
    }

    fn deserialize_f64<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("f64"))
    }

    fn deserialize_char<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("char"))
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let unquoted = unquote(s);
        visitor.visit_borrowed_str(unquoted)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.take_current()?;
        let unquoted = unquote(s);
        visitor.visit_string(unquoted.to_string())
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("bytes"))
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("byte_buf"))
    }

    fn deserialize_option<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("option"))
    }

    fn deserialize_unit<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("unit"))
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value> {
        Err(Error::UnsupportedType("unit_struct"))
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value> {
        Err(Error::UnsupportedType("newtype_struct"))
    }

    fn deserialize_seq<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("seq"))
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("tuple"))
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value> {
        Err(Error::UnsupportedType("tuple_struct"))
    }

    fn deserialize_map<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("map"))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_map(TextMapAccess::new(self, fields))
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        let variant = self.take_current()?;
        visitor.visit_enum(TextEnumAccess { variant })
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::Message("deserialize_identifier should not be called directly".to_string()))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("ignored_any"))
    }
}

/// MapAccess for reading struct fields from the HashMap.
struct TextMapAccess<'a, 'de> {
    de: &'a mut TextDeserializer<'de>,
    fields: &'static [&'static str],
    field_idx: usize,
}

impl<'a, 'de> TextMapAccess<'a, 'de> {
    fn new(de: &'a mut TextDeserializer<'de>, fields: &'static [&'static str]) -> Self {
        Self { de, fields, field_idx: 0 }
    }
}

impl<'de> MapAccess<'de> for TextMapAccess<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.field_idx >= self.fields.len() {
            return Ok(None);
        }

        let field_name = self.fields[self.field_idx];
        seed.deserialize(de::value::BorrowedStrDeserializer::new(field_name)).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        let field_name = self.fields[self.field_idx];
        self.field_idx += 1;

        // Get value from HashMap
        let value = self
            .de
            .fields
            .get(field_name)
            .ok_or_else(|| Error::MissingField(field_name.to_string()))?;

        // Set current value for nested deserialization
        self.de.set_current(value);

        seed.deserialize(&mut *self.de)
    }
}

/// EnumAccess for deserializing enum variants by name.
struct TextEnumAccess<'de> {
    variant: &'de str,
}

impl<'de> EnumAccess<'de> for TextEnumAccess<'de> {
    type Error = Error;
    type Variant = TextVariantAccess;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        let val =
            seed.deserialize(de::value::BorrowedStrDeserializer::<Error>::new(self.variant))?;
        Ok((val, TextVariantAccess))
    }
}

/// VariantAccess for unit variants.
struct TextVariantAccess;

impl<'de> VariantAccess<'de> for TextVariantAccess {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, _seed: T) -> Result<T::Value> {
        Err(Error::UnsupportedType("newtype_variant"))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("tuple_variant"))
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value> {
        Err(Error::UnsupportedType("struct_variant"))
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Removes surrounding quotes from a string if present.
fn unquote(s: &str) -> &str {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 { &s[1..s.len() - 1] } else { s }
}
