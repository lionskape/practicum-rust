//! Serde-based serialization for transaction formats.
//!
//! This module provides custom Serde `Serializer` and `Deserializer` implementations
//! for the YPBN binary and text formats, offering a unified streaming API.
//!
//! # Streaming API
//!
//! All formats support streaming read/write via:
//! - `read_one()` — read a single transaction
//! - `write_one()` — write a single transaction
//! - `iter_reader()` — iterate over transactions
//!
//! # Example
//!
//! ```ignore
//! use parser::serde::{binary, text};
//! use parser::Transaction;
//!
//! // Binary format
//! let bytes = binary::to_bytes(&tx)?;
//! let decoded: Transaction = binary::from_bytes(&bytes)?;
//!
//! // Text format (streaming)
//! for tx in text::iter_reader(file) {
//!     let tx = tx?;
//!     println!("{:?}", tx);
//! }
//! ```

pub mod binary;
mod error;
pub mod text;

use std::io::{BufRead, Read, Write};

pub use error::{Error, Result};

use crate::transaction::Transaction;

/// Marker type for Binary format.
#[derive(Debug, Clone, Copy, Default)]
pub struct Binary;

/// Marker type for Text format.
#[derive(Debug, Clone, Copy, Default)]
pub struct Text;

/// Trait for streaming serialization/deserialization of transactions.
///
/// Implemented by marker types (`Binary`, `Text`) to provide format-specific
/// streaming operations.
///
/// Note: `read_one` takes `BufRead` to support text format's line-by-line reading.
/// For binary format, `BufRead` is a superset of `Read`, so it works seamlessly.
pub trait SerdeFormat {
    /// Reads a single transaction from a buffered reader.
    ///
    /// Returns `Ok(Some(tx))` if a transaction was read, `Ok(None)` at EOF.
    ///
    /// Takes `BufRead` instead of `Read` to support text format's line-by-line
    /// reading while maintaining buffer state across multiple calls.
    fn read_one<R: BufRead>(reader: &mut R) -> Result<Option<Transaction>>;

    /// Writes a single transaction to a writer.
    fn write_one<W: Write>(writer: &mut W, tx: &Transaction) -> Result<()>;

    /// Writes a header if the format requires one.
    ///
    /// Default implementation is a no-op.
    fn write_header<W: Write>(_writer: &mut W) -> Result<()> {
        Ok(())
    }
}

impl SerdeFormat for Binary {
    fn read_one<R: BufRead>(reader: &mut R) -> Result<Option<Transaction>> {
        // BufRead is a superset of Read, so this works
        binary::read_one(reader)
    }

    fn write_one<W: Write>(writer: &mut W, tx: &Transaction) -> Result<()> {
        binary::write_one(writer, tx)
    }
}

impl SerdeFormat for Text {
    fn read_one<R: BufRead>(reader: &mut R) -> Result<Option<Transaction>> {
        // Now we can pass the reader directly without creating a new BufReader
        text::read_one(reader)
    }

    fn write_one<W: Write>(writer: &mut W, tx: &Transaction) -> Result<()> {
        text::write_one(writer, tx)
    }
}

/// Format enum for runtime format selection.
///
/// Use this when the format is determined at runtime (e.g., from file extension).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// Binary YPBN format.
    Binary,
    /// Text KEY: VALUE format.
    Text,
}

impl Format {
    /// Determines format from file extension.
    ///
    /// # Examples
    ///
    /// ```
    /// use parser::serde::Format;
    ///
    /// assert_eq!(Format::from_extension("txt"), Some(Format::Text));
    /// assert_eq!(Format::from_extension("bin"), Some(Format::Binary));
    /// assert_eq!(Format::from_extension("json"), None);
    /// ```
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "txt" | "ypbank" | "text" => Some(Self::Text),
            "bin" | "ypbin" | "binary" => Some(Self::Binary),
            _ => None,
        }
    }

    /// Detects format from file content (magic bytes).
    ///
    /// Checks for YPBN magic bytes, otherwise assumes text format.
    pub fn detect<R: Read>(reader: &mut R) -> Result<Self> {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;

        if &magic == b"YPBN" { Ok(Self::Binary) } else { Ok(Self::Text) }
    }
}
