//! Serde-based text format serialization.
//!
//! This module provides Serde `Serializer` and `Deserializer` implementations
//! for the YPBank text format with `KEY: VALUE` pairs.
//!
//! # Format
//!
//! ```text
//! TX_ID: 1234567890
//! TX_TYPE: DEPOSIT
//! FROM_USER_ID: 0
//! TO_USER_ID: 9876543210
//! AMOUNT: 50000
//! TIMESTAMP: 1700000000000
//! STATUS: SUCCESS
//! DESCRIPTION: "Test deposit"
//! ```
//!
//! Records are separated by empty lines.
//!
//! # Streaming Example
//!
//! ```ignore
//! use parser::serde::text;
//! use std::fs::File;
//!
//! let file = File::open("transactions.txt")?;
//! for tx in text::iter_reader(file) {
//!     let tx = tx?;
//!     println!("{:?}", tx);
//! }
//! ```

mod de;
mod ser;

use std::io::{BufRead, BufReader, Read, Write};

pub use de::{StreamingTextDeserializer, TextDeserializer};
pub use ser::TextSerializer;
use serde::{Deserialize, Serialize};

// Re-export Error for tests
#[cfg(test)]
use super::Error;
use super::Result;

// ============================================================================
// Streaming API (recommended for files)
// ============================================================================

/// Reads a single transaction from a reader (streaming).
///
/// Returns `Ok(Some(tx))` if a transaction was read, `Ok(None)` at EOF.
/// Records are separated by empty lines.
///
/// # Example
///
/// ```ignore
/// use parser::serde::text;
/// use std::fs::File;
/// use std::io::BufReader;
///
/// let file = BufReader::new(File::open("transactions.txt")?);
/// while let Some(tx) = text::read_one(&mut reader)? {
///     println!("{:?}", tx);
/// }
/// ```
pub fn read_one<R: BufRead, T: for<'de> Deserialize<'de>>(reader: &mut R) -> Result<Option<T>> {
    let mut de = StreamingTextDeserializer::new(reader);

    // Try to read next record - returns false at EOF
    if !de.read_record()? {
        return Ok(None);
    }

    let value = T::deserialize(&mut de)?;
    Ok(Some(value))
}

/// Reads a single transaction from any `Read` source.
///
/// Wraps the reader in `BufReader` for buffered line reading.
pub fn read_one_from<R: Read, T: for<'de> Deserialize<'de>>(reader: R) -> Result<Option<T>> {
    let mut buf_reader = BufReader::new(reader);
    read_one(&mut buf_reader)
}

/// Writes a single transaction to a writer (streaming).
///
/// Appends an empty line after the record to separate from the next one.
///
/// # Example
///
/// ```ignore
/// use parser::serde::text;
/// use std::fs::File;
///
/// let mut file = File::create("output.txt")?;
/// text::write_one(&mut file, &tx)?;
/// ```
pub fn write_one<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<()> {
    let text = to_string(value)?;
    writer.write_all(text.as_bytes())?;
    // Add separator line for next record
    writer.write_all(b"\n")?;
    Ok(())
}

/// Creates an iterator over transactions in a reader.
///
/// # Example
///
/// ```ignore
/// use parser::serde::text;
/// use std::fs::File;
///
/// let file = File::open("transactions.txt")?;
/// for result in text::iter_reader::<_, Transaction>(file) {
///     let tx = result?;
///     println!("{:?}", tx);
/// }
/// ```
pub fn iter_reader<R: Read, T: for<'de> Deserialize<'de>>(
    reader: R,
) -> impl Iterator<Item = Result<T>> {
    ReaderIterator::new(BufReader::new(reader))
}

/// Creates an iterator from a `BufRead` source (avoids double buffering).
pub fn iter_buf_reader<R: BufRead, T: for<'de> Deserialize<'de>>(
    reader: R,
) -> impl Iterator<Item = Result<T>> {
    ReaderIterator::new(reader)
}

/// Iterator adapter for streaming reads.
struct ReaderIterator<R, T> {
    de: StreamingTextDeserializer<R>,
    finished: bool,
    _marker: std::marker::PhantomData<T>,
}

impl<R: BufRead, T> ReaderIterator<R, T> {
    fn new(reader: R) -> Self {
        Self {
            de: StreamingTextDeserializer::new(reader),
            finished: false,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<R: BufRead, T: for<'de> Deserialize<'de>> Iterator for ReaderIterator<R, T> {
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // Try to read next record
        match self.de.read_record() {
            Ok(true) => {
                // Record was read, deserialize it
                match T::deserialize(&mut self.de) {
                    Ok(value) => Some(Ok(value)),
                    Err(e) => {
                        self.finished = true;
                        Some(Err(e))
                    }
                }
            }
            Ok(false) => {
                // EOF
                self.finished = true;
                None
            }
            Err(e) => {
                self.finished = true;
                Some(Err(e))
            }
        }
    }
}

// ============================================================================
// Buffered API (for in-memory operations)
// ============================================================================

/// Serializes a value to text format string.
///
/// # Example
///
/// ```ignore
/// let text = text::to_string(&transaction)?;
/// ```
pub fn to_string<T: Serialize>(value: &T) -> Result<String> {
    let mut serializer = TextSerializer::new();
    value.serialize(&mut serializer)?;
    Ok(serializer.into_output())
}

/// Serializes a value to a writer in text format.
#[deprecated(since = "0.2.0", note = "use write_one instead")]
pub fn to_writer<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<()> {
    write_one(writer, value)
}

/// Deserializes a value from text format string.
///
/// # Example
///
/// ```ignore
/// let tx: Transaction = text::from_str(&text)?;
/// ```
pub fn from_str<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T> {
    let mut deserializer = TextDeserializer::new(s)?;
    T::deserialize(&mut deserializer)
}

/// Deserializes a value from a reader in text format (streaming).
///
/// Reads one record. For multiple records, use `iter_reader` or `read_one`.
#[deprecated(since = "0.2.0", note = "use read_one for streaming, or iter_reader for iteration")]
pub fn from_reader<R: Read, T: for<'de> Deserialize<'de>>(reader: R) -> Result<Option<T>> {
    read_one_from(reader)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::{Transaction, TransactionStatus, TransactionType};

    fn sample_transaction() -> Transaction {
        Transaction {
            tx_id: 1234567890,
            tx_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 9876543210,
            amount: 50000,
            timestamp: 1700000000000,
            status: TransactionStatus::Success,
            description: "Test deposit".to_string(),
        }
    }

    #[test]
    fn test_serialize() {
        let tx = sample_transaction();
        let text = to_string(&tx).unwrap();

        assert!(text.contains("TX_ID: 1234567890"));
        assert!(text.contains("TX_TYPE: DEPOSIT"));
        assert!(text.contains("FROM_USER_ID: 0"));
        assert!(text.contains("TO_USER_ID: 9876543210"));
        assert!(text.contains("AMOUNT: 50000"));
        assert!(text.contains("STATUS: SUCCESS"));
        assert!(text.contains("DESCRIPTION: \"Test deposit\""));
    }

    #[test]
    fn test_roundtrip() {
        let original = sample_transaction();
        let text = to_string(&original).unwrap();
        let decoded: Transaction = from_str(&text).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_deserialize_with_comments() {
        let input = r#"# This is a comment
TX_ID: 100
TX_TYPE: TRANSFER
FROM_USER_ID: 1
TO_USER_ID: 2
AMOUNT: 500
TIMESTAMP: 1633036800000
STATUS: PENDING
DESCRIPTION: "Transfer test"
"#;
        let tx: Transaction = from_str(input).unwrap();
        assert_eq!(tx.tx_id, 100);
        assert_eq!(tx.tx_type, TransactionType::Transfer);
        assert_eq!(tx.status, TransactionStatus::Pending);
    }

    #[test]
    fn test_empty_description() {
        let tx = Transaction {
            tx_id: 1,
            tx_type: TransactionType::Withdrawal,
            from_user_id: 100,
            to_user_id: 0,
            amount: 1000,
            timestamp: 1000000,
            status: TransactionStatus::Failure,
            description: String::new(),
        };

        let text = to_string(&tx).unwrap();
        let decoded: Transaction = from_str(&text).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_missing_field() {
        let input = r#"TX_ID: 100
TX_TYPE: DEPOSIT
"#;
        let result: Result<Transaction> = from_str(input);
        assert!(matches!(result, Err(Error::MissingField(_))));
    }

    #[test]
    fn test_streaming_read_one() {
        let input = r#"TX_ID: 1234567890
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 9876543210
AMOUNT: 50000
TIMESTAMP: 1700000000000
STATUS: SUCCESS
DESCRIPTION: "Test deposit"
"#;
        let mut cursor = std::io::Cursor::new(input);
        let mut buf_reader = BufReader::new(&mut cursor);
        let tx: Option<Transaction> = read_one(&mut buf_reader).unwrap();

        assert_eq!(tx, Some(sample_transaction()));
    }

    #[test]
    fn test_streaming_multiple_records() {
        let input = r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 42
AMOUNT: 100
TIMESTAMP: 1000
STATUS: SUCCESS
DESCRIPTION: "First"

TX_ID: 2
TX_TYPE: TRANSFER
FROM_USER_ID: 42
TO_USER_ID: 100
AMOUNT: 50
TIMESTAMP: 2000
STATUS: PENDING
DESCRIPTION: "Second"
"#;
        let txs: Vec<Transaction> =
            iter_reader(std::io::Cursor::new(input)).collect::<Result<Vec<_>>>().unwrap();

        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0].tx_id, 1);
        assert_eq!(txs[0].tx_type, TransactionType::Deposit);
        assert_eq!(txs[1].tx_id, 2);
        assert_eq!(txs[1].tx_type, TransactionType::Transfer);
    }

    #[test]
    fn test_iter_reader_empty() {
        let input = "";
        let txs: Vec<Transaction> =
            iter_reader(std::io::Cursor::new(input)).collect::<Result<Vec<_>>>().unwrap();

        assert!(txs.is_empty());
    }

    #[test]
    fn test_write_and_read_multiple() {
        let tx1 = sample_transaction();
        let tx2 = Transaction {
            tx_id: 999,
            tx_type: TransactionType::Withdrawal,
            from_user_id: 42,
            to_user_id: 0,
            amount: 100,
            timestamp: 2000000000000,
            status: TransactionStatus::Failure,
            description: "Second tx".to_string(),
        };

        // Write multiple records
        let mut buffer = Vec::new();
        write_one(&mut buffer, &tx1).unwrap();
        write_one(&mut buffer, &tx2).unwrap();

        // Read back
        let txs: Vec<Transaction> =
            iter_reader(std::io::Cursor::new(buffer)).collect::<Result<Vec<_>>>().unwrap();

        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0], tx1);
        assert_eq!(txs[1], tx2);
    }
}
