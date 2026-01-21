//! Binary format Serde Serializer implementation.

use serde::ser::{self, Serialize};

use crate::serde::{Error, Result};

/// Serializer for YPBN binary format.
///
/// Writes data in big-endian format directly to a byte buffer.
/// This serializer is designed specifically for the `Transaction` struct.
pub struct BinarySerializer<'w> {
    output: &'w mut Vec<u8>,
}

impl<'w> BinarySerializer<'w> {
    /// Creates a new serializer writing to the given buffer.
    pub fn new(output: &'w mut Vec<u8>) -> Self {
        Self { output }
    }
}

impl<'a, 'w> ser::Serializer for &'a mut BinarySerializer<'w> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = ser::Impossible<(), Error>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeStruct = BinaryStructSerializer<'a, 'w>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

    // === Primitive types ===

    fn serialize_bool(self, _v: bool) -> Result<()> {
        Err(Error::UnsupportedType("bool"))
    }

    fn serialize_i8(self, _v: i8) -> Result<()> {
        Err(Error::UnsupportedType("i8"))
    }

    fn serialize_i16(self, _v: i16) -> Result<()> {
        Err(Error::UnsupportedType("i16"))
    }

    fn serialize_i32(self, _v: i32) -> Result<()> {
        Err(Error::UnsupportedType("i32"))
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.output.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.output.push(v);
        Ok(())
    }

    fn serialize_u16(self, _v: u16) -> Result<()> {
        Err(Error::UnsupportedType("u16"))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.output.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output.extend_from_slice(&v.to_be_bytes());
        Ok(())
    }

    fn serialize_f32(self, _v: f32) -> Result<()> {
        Err(Error::UnsupportedType("f32"))
    }

    fn serialize_f64(self, _v: f64) -> Result<()> {
        Err(Error::UnsupportedType("f64"))
    }

    fn serialize_char(self, _v: char) -> Result<()> {
        Err(Error::UnsupportedType("char"))
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        // String format: length (u32 BE) + bytes
        let bytes = v.as_bytes();
        self.output.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
        self.output.extend_from_slice(bytes);
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::UnsupportedType("bytes"))
    }

    fn serialize_none(self) -> Result<()> {
        Err(Error::UnsupportedType("Option::None"))
    }

    fn serialize_some<T: ?Sized + Serialize>(self, _value: &T) -> Result<()> {
        Err(Error::UnsupportedType("Option::Some"))
    }

    fn serialize_unit(self) -> Result<()> {
        Err(Error::UnsupportedType("unit"))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Err(Error::UnsupportedType("unit struct"))
    }

    /// Serialize enum variant as its index (u8).
    ///
    /// For TransactionType and TransactionStatus, the variant index
    /// maps directly to the binary format encoding.
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.output.push(variant_index as u8);
        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<()> {
        Err(Error::UnsupportedType("newtype struct"))
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()> {
        Err(Error::UnsupportedType("newtype variant"))
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::UnsupportedType("sequence"))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::UnsupportedType("tuple"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::UnsupportedType("tuple struct"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::UnsupportedType("tuple variant"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::UnsupportedType("map"))
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(BinaryStructSerializer { ser: self })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::UnsupportedType("struct variant"))
    }
}

/// Helper for serializing struct fields.
pub struct BinaryStructSerializer<'a, 'w> {
    ser: &'a mut BinarySerializer<'w>,
}

impl<'a, 'w> ser::SerializeStruct for BinaryStructSerializer<'a, 'w> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        value: &T,
    ) -> Result<()> {
        // Fields are serialized in declaration order (guaranteed by derive)
        // We don't need to check key names since we trust the order
        value.serialize(&mut *self.ser)
    }

    fn end(self) -> Result<()> {
        // Nothing to finalize - all data is already in the buffer
        Ok(())
    }
}
