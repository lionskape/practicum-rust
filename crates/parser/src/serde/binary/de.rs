//! Binary format Serde Deserializer implementation.
//!
//! Provides both buffered (`BinaryDeserializer`) and streaming (`StreamingBinaryDeserializer`)
//! implementations for the YPBN binary format.

use std::io::Read;

use serde::de::{self, DeserializeSeed, EnumAccess, MapAccess, VariantAccess, Visitor};

use super::MAGIC;
use crate::serde::{Error, Result};

/// Streaming deserializer for YPBN binary format.
///
/// Reads data directly from any `Read` source without buffering the entire input.
/// Each record is read independently: `[MAGIC:4][SIZE:4][BODY:size]`.
///
/// Supports recovery after errors via [`skip_to_next_magic()`](Self::skip_to_next_magic).
pub struct StreamingBinaryDeserializer<R> {
    reader: R,
    /// Total bytes read from the stream.
    bytes_read: u64,
    /// Number of successfully deserialized records.
    records_read: usize,
    /// Flag indicating MAGIC was already consumed (for recovery).
    magic_consumed: bool,
}

impl<R: Read> StreamingBinaryDeserializer<R> {
    /// Creates a new streaming deserializer.
    ///
    /// Does NOT read any data yet — call methods to start reading.
    pub fn new(reader: R) -> Self {
        Self { reader, bytes_read: 0, records_read: 0, magic_consumed: false }
    }

    /// Returns the total number of bytes read from the stream.
    #[must_use]
    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }

    /// Returns the number of successfully deserialized records.
    #[must_use]
    pub fn records_read(&self) -> usize {
        self.records_read
    }

    /// Increments records_read counter. Called after successful deserialization.
    pub(crate) fn record_completed(&mut self) {
        self.records_read += 1;
    }

    /// Reads and validates the record header (magic + size).
    ///
    /// Returns the body size in bytes, or `None` if EOF is reached cleanly.
    /// Call this before deserializing each record.
    pub fn read_header(&mut self) -> Result<Option<u32>> {
        // If magic was already consumed by skip_to_next_magic(), skip reading it
        if !self.magic_consumed {
            let mut magic = [0u8; 4];

            // Try to read magic bytes - EOF here is OK (no more records)
            match self.reader.read_exact(&mut magic) {
                Ok(()) => self.bytes_read += 4,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
                Err(e) => return Err(e.into()),
            }

            // Validate magic
            if &magic != MAGIC {
                return Err(Error::InvalidMagic(magic));
            }
        } else {
            // Reset the flag for next record
            self.magic_consumed = false;
        }

        // Read size
        let mut size_bytes = [0u8; 4];
        self.reader.read_exact(&mut size_bytes)?;
        self.bytes_read += 4;
        let size = u32::from_be_bytes(size_bytes);

        Ok(Some(size))
    }

    /// Scans the stream for the next MAGIC sequence "YPBN".
    ///
    /// This method is used for recovery after encountering corrupted data.
    /// It reads byte-by-byte until it finds the MAGIC sequence, then positions
    /// the stream so that the next [`read_header()`](Self::read_header) call
    /// will read the SIZE field directly.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(skipped))` — found MAGIC after skipping `skipped` bytes
    /// - `Ok(None)` — reached EOF without finding MAGIC
    /// - `Err(e)` — I/O error
    ///
    /// # Example
    ///
    /// ```ignore
    /// loop {
    ///     match de.read_header() {
    ///         Ok(Some(size)) => { /* deserialize record */ }
    ///         Ok(None) => break, // EOF
    ///         Err(_) => {
    ///             // Try to recover
    ///             if de.skip_to_next_magic()?.is_none() {
    ///                 break; // EOF during recovery
    ///             }
    ///             // Continue with next record
    ///         }
    ///     }
    /// }
    /// ```
    pub fn skip_to_next_magic(&mut self) -> Result<Option<u64>> {
        let mut window = [0u8; 4];
        let mut skipped: u64 = 0;

        // Read first 4 bytes to initialize window
        match self.reader.read_exact(&mut window) {
            Ok(()) => self.bytes_read += 4,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        }

        loop {
            // Check if current window matches MAGIC
            if &window == MAGIC {
                // Found MAGIC! Set flag so read_header() skips magic reading
                self.magic_consumed = true;
                return Ok(Some(skipped));
            }

            // Read next byte
            let mut byte = [0u8; 1];
            match self.reader.read_exact(&mut byte) {
                Ok(()) => self.bytes_read += 1,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
                Err(e) => return Err(e.into()),
            }

            // Shift window left and add new byte
            window[0] = window[1];
            window[1] = window[2];
            window[2] = window[3];
            window[3] = byte[0];
            skipped += 1;
        }
    }

    /// Reads a single byte.
    fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.reader.read_exact(&mut buf)?;
        self.bytes_read += 1;
        Ok(buf[0])
    }

    /// Reads a u32 in big-endian format.
    fn read_u32_be(&mut self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.reader.read_exact(&mut buf)?;
        self.bytes_read += 4;
        Ok(u32::from_be_bytes(buf))
    }

    /// Reads a u64 in big-endian format.
    fn read_u64_be(&mut self) -> Result<u64> {
        let mut buf = [0u8; 8];
        self.reader.read_exact(&mut buf)?;
        self.bytes_read += 8;
        Ok(u64::from_be_bytes(buf))
    }

    /// Reads an i64 in big-endian format.
    fn read_i64_be(&mut self) -> Result<i64> {
        let mut buf = [0u8; 8];
        self.reader.read_exact(&mut buf)?;
        self.bytes_read += 8;
        Ok(i64::from_be_bytes(buf))
    }

    /// Reads a length-prefixed string.
    fn read_string(&mut self) -> Result<String> {
        let len = self.read_u32_be()? as usize;
        let mut buf = vec![0u8; len];
        self.reader.read_exact(&mut buf)?;
        self.bytes_read += len as u64;
        String::from_utf8(buf).map_err(Error::from)
    }

    /// Consumes the reader, returning it.
    pub fn into_inner(self) -> R {
        self.reader
    }
}

impl<'de, R: Read> de::Deserializer<'de> for &mut StreamingBinaryDeserializer<R> {
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
        visitor.visit_i64(self.read_i64_be()?)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u8(self.read_u8()?)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("u16"))
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u32(self.read_u32_be()?)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u64(self.read_u64_be()?)
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
        // Streaming can't return borrowed str, use owned string
        visitor.visit_string(self.read_string()?)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_string(self.read_string()?)
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
        visitor.visit_map(StreamingBinaryMapAccess::new(self, fields))
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        // Read variant index as u8
        let idx = self.read_u8()?;
        visitor.visit_enum(BinaryEnumAccess { idx })
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::Message("deserialize_identifier should not be called directly".to_string()))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("ignored_any"))
    }
}

/// MapAccess implementation for streaming deserializer.
struct StreamingBinaryMapAccess<'a, R> {
    de: &'a mut StreamingBinaryDeserializer<R>,
    fields: &'static [&'static str],
    field_idx: usize,
}

impl<'a, R: Read> StreamingBinaryMapAccess<'a, R> {
    fn new(de: &'a mut StreamingBinaryDeserializer<R>, fields: &'static [&'static str]) -> Self {
        Self { de, fields, field_idx: 0 }
    }
}

impl<'de, R: Read> MapAccess<'de> for StreamingBinaryMapAccess<'_, R> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.field_idx >= self.fields.len() {
            return Ok(None);
        }

        let field_name = self.fields[self.field_idx];
        seed.deserialize(de::value::BorrowedStrDeserializer::new(field_name)).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        self.field_idx += 1;
        seed.deserialize(&mut *self.de)
    }
}

// ============================================================================
// Legacy buffered deserializer (kept for from_bytes compatibility)
// ============================================================================

/// Buffered deserializer for YPBN binary format.
///
/// Reads data from a byte slice. Use `StreamingBinaryDeserializer` for
/// streaming from `Read` sources.
pub struct BinaryDeserializer<'de> {
    input: &'de [u8],
    pos: usize,
}

impl<'de> BinaryDeserializer<'de> {
    /// Creates a new deserializer, validating magic bytes.
    pub fn new(input: &'de [u8]) -> Result<Self> {
        if input.len() < 8 {
            return Err(Error::UnexpectedEof);
        }

        // Validate magic bytes
        let magic: [u8; 4] = input[0..4].try_into().unwrap();
        if &magic != MAGIC {
            return Err(Error::InvalidMagic(magic));
        }

        // Skip magic (4) and size (4) - we trust the size for now
        Ok(Self { input, pos: 8 })
    }

    /// Returns true if all input has been consumed.
    pub fn is_empty(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn read_u8(&mut self) -> Result<u8> {
        if self.pos >= self.input.len() {
            return Err(Error::UnexpectedEof);
        }
        let byte = self.input[self.pos];
        self.pos += 1;
        Ok(byte)
    }

    fn read_u32_be(&mut self) -> Result<u32> {
        if self.pos + 4 > self.input.len() {
            return Err(Error::UnexpectedEof);
        }
        let bytes: [u8; 4] = self.input[self.pos..self.pos + 4].try_into().unwrap();
        self.pos += 4;
        Ok(u32::from_be_bytes(bytes))
    }

    fn read_u64_be(&mut self) -> Result<u64> {
        if self.pos + 8 > self.input.len() {
            return Err(Error::UnexpectedEof);
        }
        let bytes: [u8; 8] = self.input[self.pos..self.pos + 8].try_into().unwrap();
        self.pos += 8;
        Ok(u64::from_be_bytes(bytes))
    }

    fn read_i64_be(&mut self) -> Result<i64> {
        if self.pos + 8 > self.input.len() {
            return Err(Error::UnexpectedEof);
        }
        let bytes: [u8; 8] = self.input[self.pos..self.pos + 8].try_into().unwrap();
        self.pos += 8;
        Ok(i64::from_be_bytes(bytes))
    }

    fn read_string(&mut self) -> Result<String> {
        let len = self.read_u32_be()? as usize;
        if self.pos + len > self.input.len() {
            return Err(Error::UnexpectedEof);
        }
        let bytes = &self.input[self.pos..self.pos + len];
        self.pos += len;
        String::from_utf8(bytes.to_vec()).map_err(Error::from)
    }
}

impl<'de> de::Deserializer<'de> for &mut BinaryDeserializer<'de> {
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
        visitor.visit_i64(self.read_i64_be()?)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u8(self.read_u8()?)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("u16"))
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u32(self.read_u32_be()?)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u64(self.read_u64_be()?)
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
        visitor.visit_string(self.read_string()?)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_string(self.read_string()?)
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
        visitor.visit_map(BinaryMapAccess::new(self, fields))
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        let idx = self.read_u8()?;
        visitor.visit_enum(BinaryEnumAccess { idx })
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::Message("deserialize_identifier should not be called directly".to_string()))
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::UnsupportedType("ignored_any"))
    }
}

/// MapAccess implementation for buffered deserializer.
struct BinaryMapAccess<'a, 'de> {
    de: &'a mut BinaryDeserializer<'de>,
    fields: &'static [&'static str],
    field_idx: usize,
}

impl<'a, 'de> BinaryMapAccess<'a, 'de> {
    fn new(de: &'a mut BinaryDeserializer<'de>, fields: &'static [&'static str]) -> Self {
        Self { de, fields, field_idx: 0 }
    }
}

impl<'de> MapAccess<'de> for BinaryMapAccess<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.field_idx >= self.fields.len() {
            return Ok(None);
        }

        let field_name = self.fields[self.field_idx];
        seed.deserialize(de::value::BorrowedStrDeserializer::new(field_name)).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        self.field_idx += 1;
        seed.deserialize(&mut *self.de)
    }
}

/// EnumAccess implementation for deserializing enum variants by index.
struct BinaryEnumAccess {
    idx: u8,
}

impl<'de> EnumAccess<'de> for BinaryEnumAccess {
    type Error = Error;
    type Variant = BinaryVariantAccess;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        let val = seed.deserialize(de::value::U32Deserializer::<Error>::new(self.idx as u32))?;
        Ok((val, BinaryVariantAccess))
    }
}

/// VariantAccess implementation for unit variants.
struct BinaryVariantAccess;

impl<'de> VariantAccess<'de> for BinaryVariantAccess {
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
