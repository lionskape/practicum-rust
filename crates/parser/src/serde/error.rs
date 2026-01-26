//! Unified error type for Serde-based serialization/deserialization.

use std::{fmt, io, str::Utf8Error, string::FromUtf8Error};

use serde::{de, ser};

/// Error type for custom Serde serializers and deserializers.
#[derive(Debug)]
pub enum Error {
    // === General errors ===
    /// Custom message from Serde framework.
    Message(String),

    /// I/O error during read/write operations.
    Io(io::Error),

    /// Invalid UTF-8 in string data.
    InvalidUtf8(FromUtf8Error),

    /// Invalid UTF-8 in string slice.
    InvalidUtf8Slice(Utf8Error),

    /// CSV parsing or writing error.
    Csv(::csv::Error),

    /// Unexpected end of input.
    UnexpectedEof,

    // === Binary format errors ===
    /// Invalid magic bytes (expected "YPBN").
    InvalidMagic([u8; 4]),

    /// Invalid enum discriminant value.
    InvalidEnumValue {
        /// Field name (e.g., "TX_TYPE", "STATUS").
        field: &'static str,
        /// Actual byte value.
        value: u8,
    },

    /// Record size mismatch between header and actual data.
    RecordSizeMismatch {
        /// Size declared in header.
        expected: u32,
        /// Actual size of body.
        actual: u32,
    },

    // === Text format errors ===
    /// Required field is missing.
    MissingField(String),

    /// Invalid field format (e.g., missing quotes around description).
    InvalidFieldFormat(String),

    // === Serde-specific errors ===
    /// Expected a struct, got something else.
    ExpectedStruct,

    /// Unknown field name.
    UnknownField(String),

    /// Unsupported type for this format.
    UnsupportedType(&'static str),

    /// Trailing data after deserialization.
    TrailingData,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(msg) => write!(f, "{msg}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::InvalidUtf8(err) => write!(f, "Invalid UTF-8: {err}"),
            Self::InvalidUtf8Slice(err) => write!(f, "Invalid UTF-8: {err}"),
            Self::Csv(err) => write!(f, "CSV error: {err}"),
            Self::UnexpectedEof => write!(f, "Unexpected end of input"),
            Self::InvalidMagic(magic) => {
                write!(f, "Invalid magic bytes: {:?} (expected \"YPBN\")", magic)
            }
            Self::InvalidEnumValue { field, value } => {
                write!(f, "Invalid enum value {value} for field {field}")
            }
            Self::RecordSizeMismatch { expected, actual } => {
                write!(f, "Record size mismatch: header says {expected}, actual is {actual}")
            }
            Self::MissingField(field) => write!(f, "Missing required field: {field}"),
            Self::InvalidFieldFormat(msg) => write!(f, "Invalid field format: {msg}"),
            Self::ExpectedStruct => write!(f, "Expected a struct"),
            Self::UnknownField(field) => write!(f, "Unknown field: {field}"),
            Self::UnsupportedType(ty) => write!(f, "Unsupported type: {ty}"),
            Self::TrailingData => write!(f, "Trailing data after deserialization"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::InvalidUtf8(err) => Some(err),
            Self::InvalidUtf8Slice(err) => Some(err),
            Self::Csv(err) => Some(err),
            _ => None,
        }
    }
}

// Required for serde::ser::Serializer
impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Message(msg.to_string())
    }
}

// Required for serde::de::Deserializer
impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::Message(msg.to_string())
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(err: FromUtf8Error) -> Self {
        Self::InvalidUtf8(err)
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::InvalidUtf8Slice(err)
    }
}

impl From<::csv::Error> for Error {
    fn from(err: ::csv::Error) -> Self {
        Self::Csv(err)
    }
}

/// Shorthand Result type for serde operations.
pub type Result<T> = std::result::Result<T, Error>;
