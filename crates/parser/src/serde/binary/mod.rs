//! Serde-based YPBN binary format serialization.
//!
//! This module provides Serde `Serializer` and `Deserializer` implementations
//! for the YPBank binary format with magic bytes `YPBN` and big-endian encoding.
//!
//! # Format
//!
//! ```text
//! [MAGIC: 4 bytes] [SIZE: 4 bytes BE] [BODY: variable]
//! "YPBN"           (u32)              TX_ID(8) + TX_TYPE(1) + ...
//! ```
//!
//! # Streaming Example
//!
//! ```ignore
//! use parser::serde::binary;
//! use std::fs::File;
//!
//! let file = File::open("transactions.bin")?;
//! for tx in binary::iter_reader(file) {
//!     let tx = tx?;
//!     println!("{:?}", tx);
//! }
//! ```

mod de;
mod ser;

use std::io::{Read, Write};

pub use de::{BinaryDeserializer, StreamingBinaryDeserializer};
pub use ser::BinarySerializer;
use serde::{Deserialize, Serialize};

use super::{Error, Result};

/// Magic bytes for YPBN format.
pub const MAGIC: &[u8; 4] = b"YPBN";

// ============================================================================
// Streaming API (recommended for files)
// ============================================================================

/// Reads a single transaction from a reader (streaming).
///
/// Returns `Ok(Some(tx))` if a transaction was read, `Ok(None)` at EOF,
/// or `Err` on parse error.
///
/// # Example
///
/// ```ignore
/// use parser::serde::binary;
/// use std::fs::File;
///
/// let mut file = File::open("transactions.bin")?;
/// while let Some(tx) = binary::read_one(&mut file)? {
///     println!("{:?}", tx);
/// }
/// ```
pub fn read_one<R: Read, T: for<'de> Deserialize<'de>>(reader: &mut R) -> Result<Option<T>> {
    let mut de = StreamingBinaryDeserializer::new(reader);

    // Try to read header - returns None at clean EOF
    if de.read_header()?.is_none() {
        return Ok(None);
    }

    // Deserialize the record body
    let value = T::deserialize(&mut de)?;
    Ok(Some(value))
}

/// Writes a single transaction to a writer (streaming).
///
/// Each call writes one complete record with magic bytes and size header.
///
/// # Example
///
/// ```ignore
/// use parser::serde::binary;
/// use std::fs::File;
///
/// let mut file = File::create("output.bin")?;
/// binary::write_one(&mut file, &tx)?;
/// ```
pub fn write_one<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<()> {
    let bytes = to_bytes(value)?;
    writer.write_all(&bytes)?;
    Ok(())
}

/// Creates an iterator over transactions in a reader.
///
/// # Example
///
/// ```ignore
/// use parser::serde::binary;
/// use std::fs::File;
///
/// let file = File::open("transactions.bin")?;
/// for result in binary::iter_reader::<_, Transaction>(file) {
///     let tx = result?;
///     println!("{:?}", tx);
/// }
/// ```
pub fn iter_reader<R: Read, T: for<'de> Deserialize<'de>>(
    reader: R,
) -> impl Iterator<Item = Result<T>> {
    ReaderIterator::new(reader)
}

/// Iterator adapter for streaming reads.
struct ReaderIterator<R, T> {
    reader: R,
    finished: bool,
    _marker: std::marker::PhantomData<T>,
}

impl<R: Read, T> ReaderIterator<R, T> {
    fn new(reader: R) -> Self {
        Self { reader, finished: false, _marker: std::marker::PhantomData }
    }
}

impl<R: Read, T: for<'de> Deserialize<'de>> Iterator for ReaderIterator<R, T> {
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        match read_one(&mut self.reader) {
            Ok(Some(value)) => Some(Ok(value)),
            Ok(None) => {
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
// Recovery API
// ============================================================================

/// Creates a recoverable iterator over transactions in a reader.
///
/// Unlike [`iter_reader`], this iterator attempts to recover from errors
/// by scanning for the next MAGIC sequence "YPBN" and continuing.
///
/// # Example
///
/// ```ignore
/// use parser::serde::binary;
/// use std::fs::File;
///
/// let file = File::open("possibly_corrupt.bin")?;
/// let reader = binary::iter_reader_with_recovery::<_, Transaction>(file);
///
/// for result in reader {
///     match result {
///         Ok(tx) => println!("Read: {:?}", tx),
///         Err(e) => println!("Skipped corrupt record: {}", e),
///     }
/// }
///
/// println!("Successfully read: {}, skipped: {}", reader.records_read(), reader.skipped_count());
/// ```
pub fn iter_reader_with_recovery<R: Read, T: for<'de> Deserialize<'de>>(
    reader: R,
) -> RecoverableIterator<R, T> {
    RecoverableIterator::new(reader)
}

/// Iterator that recovers from errors by seeking to next MAGIC sequence.
///
/// This iterator tracks statistics about successful and skipped records,
/// accessible via [`records_read()`](Self::records_read) and
/// [`skipped_count()`](Self::skipped_count).
pub struct RecoverableIterator<R, T> {
    de: StreamingBinaryDeserializer<R>,
    finished: bool,
    skipped: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<R: Read, T> RecoverableIterator<R, T> {
    fn new(reader: R) -> Self {
        Self {
            de: StreamingBinaryDeserializer::new(reader),
            finished: false,
            skipped: 0,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the number of successfully deserialized records.
    #[must_use]
    pub fn records_read(&self) -> usize {
        self.de.records_read()
    }

    /// Returns the number of skipped (corrupt) records.
    #[must_use]
    pub fn skipped_count(&self) -> usize {
        self.skipped
    }

    /// Returns the total bytes read from the stream.
    #[must_use]
    pub fn bytes_read(&self) -> u64 {
        self.de.bytes_read()
    }
}

impl<R: Read, T: for<'de> Deserialize<'de>> Iterator for RecoverableIterator<R, T> {
    type Item = Result<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        loop {
            // Try to read header
            match self.de.read_header() {
                Ok(Some(_size)) => {
                    // Try to deserialize
                    match T::deserialize(&mut self.de) {
                        Ok(value) => {
                            self.de.record_completed();
                            return Some(Ok(value));
                        }
                        Err(e) => {
                            // Deserialization failed, try to recover
                            self.skipped += 1;
                            match self.de.skip_to_next_magic() {
                                Ok(Some(_)) => continue, // Found next record, retry
                                Ok(None) => {
                                    self.finished = true;
                                    return Some(Err(e)); // EOF during recovery
                                }
                                Err(io_err) => {
                                    self.finished = true;
                                    return Some(Err(io_err));
                                }
                            }
                        }
                    }
                }
                Ok(None) => {
                    // Clean EOF
                    self.finished = true;
                    return None;
                }
                Err(e) => {
                    // Header read failed (e.g., invalid magic)
                    self.skipped += 1;
                    match self.de.skip_to_next_magic() {
                        Ok(Some(_)) => continue, // Found next record, retry
                        Ok(None) => {
                            self.finished = true;
                            return Some(Err(e)); // EOF during recovery
                        }
                        Err(io_err) => {
                            self.finished = true;
                            return Some(Err(io_err));
                        }
                    }
                }
            }
        }
    }
}

// ============================================================================
// Buffered API (for in-memory operations)
// ============================================================================

/// Serializes a value to YPBN binary bytes.
///
/// # Example
///
/// ```ignore
/// let bytes = binary::to_bytes(&transaction)?;
/// ```
pub fn to_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    // Estimate capacity: magic(4) + size(4) + typical body(~60)
    let mut result = Vec::with_capacity(68);

    // Write magic bytes
    result.extend_from_slice(MAGIC);

    // Placeholder for size (will be patched later)
    let size_pos = result.len();
    result.extend_from_slice(&[0u8; 4]);

    // Serialize body directly into result
    let body_start = result.len();
    let mut serializer = BinarySerializer::new(&mut result);
    value.serialize(&mut serializer)?;

    // Patch size with actual body length
    let body_size = (result.len() - body_start) as u32;
    result[size_pos..size_pos + 4].copy_from_slice(&body_size.to_be_bytes());

    Ok(result)
}

/// Serializes a value to a writer in YPBN format.
///
/// For optimal performance with seekable writers, consider using `to_bytes`
/// and writing the result. This function buffers one record.
#[deprecated(since = "0.2.0", note = "use write_one instead")]
pub fn to_writer<W: Write, T: Serialize>(writer: &mut W, value: &T) -> Result<()> {
    write_one(writer, value)
}

/// Deserializes a value from YPBN binary bytes.
///
/// # Example
///
/// ```ignore
/// let tx: Transaction = binary::from_bytes(&bytes)?;
/// ```
pub fn from_bytes<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> Result<T> {
    let mut deserializer = BinaryDeserializer::new(bytes)?;
    let value = T::deserialize(&mut deserializer)?;

    // Check for trailing data
    if !deserializer.is_empty() {
        return Err(Error::TrailingData);
    }

    Ok(value)
}

/// Deserializes a value from a reader in YPBN format (streaming).
///
/// Reads one record. For multiple records, use `iter_reader` or `read_one`.
#[deprecated(since = "0.2.0", note = "use read_one for streaming, or iter_reader for iteration")]
pub fn from_reader<R: Read, T: for<'de> Deserialize<'de>>(reader: &mut R) -> Result<Option<T>> {
    read_one(reader)
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
    fn test_roundtrip() {
        let original = sample_transaction();
        let bytes = to_bytes(&original).unwrap();
        let decoded: Transaction = from_bytes(&bytes).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_magic_bytes() {
        let tx = sample_transaction();
        let bytes = to_bytes(&tx).unwrap();
        assert_eq!(&bytes[0..4], b"YPBN");
    }

    #[test]
    fn test_size_field() {
        let tx = sample_transaction();
        let bytes = to_bytes(&tx).unwrap();

        // Read size from header
        let size = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        // Body starts at offset 8
        let actual_body_size = bytes.len() - 8;
        assert_eq!(size as usize, actual_body_size);
    }

    #[test]
    fn test_empty_description() {
        let tx = Transaction {
            tx_id: 1,
            tx_type: TransactionType::Transfer,
            from_user_id: 100,
            to_user_id: 200,
            amount: 1000,
            timestamp: 1000000,
            status: TransactionStatus::Pending,
            description: String::new(),
        };

        let bytes = to_bytes(&tx).unwrap();
        let decoded: Transaction = from_bytes(&bytes).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_invalid_magic() {
        let bytes = b"BADM\x00\x00\x00\x10rest of data...";
        let result: Result<Transaction> = from_bytes(bytes);
        assert!(matches!(result, Err(Error::InvalidMagic(_))));
    }

    #[test]
    fn test_streaming_read_one() {
        let tx = sample_transaction();
        let bytes = to_bytes(&tx).unwrap();

        let mut cursor = std::io::Cursor::new(bytes);
        let decoded: Option<Transaction> = read_one(&mut cursor).unwrap();

        assert_eq!(decoded, Some(tx));

        // Next read should return None (EOF)
        let eof: Option<Transaction> = read_one(&mut cursor).unwrap();
        assert!(eof.is_none());
    }

    #[test]
    fn test_streaming_multiple_records() {
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

        // Read back using iterator
        let txs: Vec<Transaction> =
            iter_reader(std::io::Cursor::new(buffer)).collect::<Result<Vec<_>>>().unwrap();

        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0], tx1);
        assert_eq!(txs[1], tx2);
    }

    #[test]
    fn test_iter_reader_empty() {
        let buffer: Vec<u8> = Vec::new();
        let txs: Vec<Transaction> =
            iter_reader(std::io::Cursor::new(buffer)).collect::<Result<Vec<_>>>().unwrap();

        assert!(txs.is_empty());
    }

    // ========================================================================
    // Recovery tests
    // ========================================================================

    #[test]
    fn test_skip_to_next_magic_basic() {
        let tx = sample_transaction();
        let valid_bytes = to_bytes(&tx).unwrap();

        // Create buffer: garbage + valid record
        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"GARBAGE_DATA_HERE");
        buffer.extend_from_slice(&valid_bytes);

        let mut de = StreamingBinaryDeserializer::new(std::io::Cursor::new(buffer));

        // First read_header should fail (reads "GARB" as magic)
        let result = de.read_header();
        assert!(matches!(result, Err(Error::InvalidMagic(_))));

        // Skip to next magic
        let skipped = de.skip_to_next_magic().unwrap();
        assert!(skipped.is_some());

        // Now read_header should succeed
        let size = de.read_header().unwrap();
        assert!(size.is_some());

        // And deserialization should work
        let decoded: Transaction = Transaction::deserialize(&mut de).unwrap();
        assert_eq!(decoded, tx);
    }

    #[test]
    fn test_skip_to_next_magic_at_start() {
        let tx = sample_transaction();
        let valid_bytes = to_bytes(&tx).unwrap();

        let mut de = StreamingBinaryDeserializer::new(std::io::Cursor::new(valid_bytes));

        // skip_to_next_magic at start should find MAGIC immediately (0 skipped)
        let skipped = de.skip_to_next_magic().unwrap();
        assert_eq!(skipped, Some(0));

        // read_header should work
        let size = de.read_header().unwrap();
        assert!(size.is_some());
    }

    #[test]
    fn test_skip_to_next_magic_eof() {
        let buffer = b"NO_MAGIC_HERE_JUST_GARBAGE";

        let mut de = StreamingBinaryDeserializer::new(std::io::Cursor::new(buffer.as_slice()));

        // skip_to_next_magic should return None (EOF)
        let result = de.skip_to_next_magic().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_recovery_after_corrupt_record() {
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

        let bytes1 = to_bytes(&tx1).unwrap();
        let bytes2 = to_bytes(&tx2).unwrap();

        // Create buffer: valid1 + garbage + valid2
        let mut buffer = Vec::new();
        buffer.extend_from_slice(&bytes1);
        buffer.extend_from_slice(b"CORRUPTED_DATA");
        buffer.extend_from_slice(&bytes2);

        // Regular iterator should fail after first record
        let regular_txs: Vec<_> =
            iter_reader::<_, Transaction>(std::io::Cursor::new(buffer.clone())).collect();
        assert_eq!(regular_txs.len(), 2); // First OK, second is error
        assert!(regular_txs[0].is_ok());
        assert!(regular_txs[1].is_err());

        // Recovery iterator should get both valid records
        let mut recovery_iter =
            iter_reader_with_recovery::<_, Transaction>(std::io::Cursor::new(buffer));
        let results: Vec<_> = recovery_iter.by_ref().collect();

        // Should have 2 successful results (tx1 and tx2)
        let successes: Vec<_> = results.iter().filter(|r| r.is_ok()).collect();
        assert_eq!(successes.len(), 2);

        // Check recovered values
        assert_eq!(results[0].as_ref().unwrap(), &tx1);
        // tx2 should be somewhere in results (after recovery)
        let found_tx2 = results.iter().any(|r| r.as_ref().ok() == Some(&tx2));
        assert!(found_tx2, "Should have recovered tx2");

        // Should have skipped at least 1 corrupt region
        assert!(recovery_iter.skipped_count() >= 1);
    }

    #[test]
    fn test_recovery_multiple_corruptions() {
        let tx = sample_transaction();
        let valid_bytes = to_bytes(&tx).unwrap();

        // Create buffer: garbage1 + valid + garbage2 + valid + garbage3 + valid
        let mut buffer = Vec::new();
        buffer.extend_from_slice(b"JUNK1");
        buffer.extend_from_slice(&valid_bytes);
        buffer.extend_from_slice(b"JUNK2JUNK2");
        buffer.extend_from_slice(&valid_bytes);
        buffer.extend_from_slice(b"JUNK3");
        buffer.extend_from_slice(&valid_bytes);

        let mut recovery_iter =
            iter_reader_with_recovery::<_, Transaction>(std::io::Cursor::new(buffer));
        let results: Vec<Result<Transaction>> = recovery_iter.by_ref().collect();

        // Should have recovered 3 valid transactions
        let successes: Vec<_> = results.into_iter().filter_map(|r| r.ok()).collect();
        assert_eq!(successes.len(), 3);

        // All should be equal to the sample
        for decoded in successes {
            assert_eq!(decoded, tx);
        }
    }

    #[test]
    fn test_recovery_stats() {
        let tx = sample_transaction();
        let valid_bytes = to_bytes(&tx).unwrap();

        let mut buffer = Vec::new();
        buffer.extend_from_slice(&valid_bytes);
        buffer.extend_from_slice(b"CORRUPT");
        buffer.extend_from_slice(&valid_bytes);

        let mut iter =
            iter_reader_with_recovery::<_, Transaction>(std::io::Cursor::new(buffer.clone()));

        // Consume iterator
        let _: Vec<_> = iter.by_ref().collect();

        assert_eq!(iter.records_read(), 2);
        assert!(iter.skipped_count() >= 1);
        assert!(iter.bytes_read() > 0);
    }
}
