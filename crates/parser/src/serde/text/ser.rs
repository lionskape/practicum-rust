//! Text format Serde Serializer implementation.

use serde::ser::{self, Serialize};

use crate::serde::{Error, Result};

/// Serializer for YPBank text format.
///
/// Writes data as `KEY: VALUE` pairs, one per line.
pub struct TextSerializer {
    output: String,
}

impl TextSerializer {
    /// Creates a new serializer.
    pub fn new() -> Self {
        Self { output: String::new() }
    }

    /// Consumes the serializer and returns the output string.
    pub fn into_output(self) -> String {
        self.output
    }
}

impl Default for TextSerializer {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ser::Serializer for &'a mut TextSerializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = ser::Impossible<(), Error>;
    type SerializeTuple = ser::Impossible<(), Error>;
    type SerializeTupleStruct = ser::Impossible<(), Error>;
    type SerializeTupleVariant = ser::Impossible<(), Error>;
    type SerializeMap = ser::Impossible<(), Error>;
    type SerializeStruct = TextStructSerializer<'a>;
    type SerializeStructVariant = ser::Impossible<(), Error>;

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
        self.output.push_str(&v.to_string());
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.output.push_str(&v.to_string());
        Ok(())
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.output.push_str(&v.to_string());
        Ok(())
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.output.push_str(&v.to_string());
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output.push_str(&v.to_string());
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
        // Strings are wrapped in double quotes
        self.output.push('"');
        self.output.push_str(v);
        self.output.push('"');
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

    /// Serialize enum variant as its name (string).
    ///
    /// For TransactionType and TransactionStatus, we use the variant name
    /// (DEPOSIT, TRANSFER, etc.) as defined by #[serde(rename = "...")].
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.output.push_str(variant);
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
        Ok(TextStructSerializer { ser: self })
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

/// Helper for serializing struct fields as KEY: VALUE pairs.
pub struct TextStructSerializer<'a> {
    ser: &'a mut TextSerializer,
}

impl ser::SerializeStruct for TextStructSerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        // Write "KEY: "
        self.ser.output.push_str(key);
        self.ser.output.push_str(": ");

        // Write value
        value.serialize(&mut *self.ser)?;

        // Newline
        self.ser.output.push('\n');

        Ok(())
    }

    fn end(self) -> Result<()> {
        // Add blank line between records
        self.ser.output.push('\n');
        Ok(())
    }
}
