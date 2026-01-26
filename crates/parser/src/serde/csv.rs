//! CSV format serialization for YPBank transactions.
//!
//! This module provides streaming read/write operations for transactions
//! in standard CSV format with a header row.
//!
//! # Format
//!
//! ```csv
//! TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
//! 1234567890,DEPOSIT,0,9876543210,50000,1700000000000,SUCCESS,"Test deposit"
//! ```
//!
//! # Streaming Example
//!
//! ```ignore
//! use parser::serde::csv;
//! use std::fs::File;
//!
//! let file = File::open("transactions.csv")?;
//! for tx in csv::iter_reader(file) {
//!     let tx = tx?;
//!     println!("{:?}", tx);
//! }
//! ```

use std::io::{BufRead, BufReader, Read, Write};

use serde::{Deserialize, Serialize};

use super::{Error, Result};
use crate::transaction::Transaction;

/// CSV header line with all field names.
pub const HEADER: &str =
    "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION";

// ============================================================================
// Streaming API (recommended for files)
// ============================================================================

/// Reads a single transaction from a CSV reader (streaming).
///
/// **Important**: This function expects the header to be already skipped.
/// Use [`iter_reader`] for automatic header handling, or manually skip
/// the first line before calling this function.
///
/// Returns `Ok(Some(tx))` if a transaction was read, `Ok(None)` at EOF.
///
/// # Example
///
/// ```ignore
/// use parser::serde::csv;
/// use std::io::BufReader;
/// use std::fs::File;
///
/// let mut reader = BufReader::new(File::open("transactions.csv")?);
/// // Skip header manually
/// let mut header = String::new();
/// reader.read_line(&mut header)?;
///
/// while let Some(tx) = csv::read_one(&mut reader)? {
///     println!("{:?}", tx);
/// }
/// ```
pub fn read_one<R: BufRead>(reader: &mut R) -> Result<Option<Transaction>> {
    let mut line = String::new();

    // Skip empty lines
    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Ok(None); // EOF
        }
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            break;
        }
    }

    // Parse the CSV line
    let mut csv_reader =
        ::csv::ReaderBuilder::new().has_headers(false).flexible(true).from_reader(line.as_bytes());

    match csv_reader.deserialize().next() {
        Some(Ok(tx)) => Ok(Some(tx)),
        Some(Err(e)) => Err(Error::Csv(e)),
        None => Ok(None),
    }
}

/// Skips the CSV header line when reading.
///
/// Should be called once before reading the first transaction.
/// Returns Ok(()) even if the file is empty.
///
/// # Example
///
/// ```ignore
/// use parser::serde::csv;
/// use std::io::BufReader;
/// use std::fs::File;
///
/// let mut reader = BufReader::new(File::open("transactions.csv")?);
/// csv::skip_header(&mut reader)?;
/// while let Some(tx) = csv::read_one(&mut reader)? {
///     println!("{:?}", tx);
/// }
/// ```
pub fn skip_header<R: BufRead>(reader: &mut R) -> Result<()> {
    let mut header = String::new();
    reader.read_line(&mut header)?;
    // TODO: Header Validation
    // We don't validate the header content - just skip it
    Ok(())
}

/// Writes the CSV header line.
///
/// Should be called once before writing any transactions.
///
/// # Example
///
/// ```ignore
/// use parser::serde::csv;
/// use std::fs::File;
///
/// let mut file = File::create("output.csv")?;
/// csv::write_header(&mut file)?;
/// csv::write_one(&mut file, &tx)?;
/// ```
pub fn write_header<W: Write>(writer: &mut W) -> Result<()> {
    writeln!(writer, "{}", HEADER)?;
    Ok(())
}

/// Writes a single transaction as a CSV row (streaming).
///
/// # Example
///
/// ```ignore
/// use parser::serde::csv;
/// use std::fs::File;
///
/// let mut file = File::create("output.csv")?;
/// csv::write_header(&mut file)?;
/// csv::write_one(&mut file, &tx)?;
/// ```
pub fn write_one<W: Write>(writer: &mut W, tx: &Transaction) -> Result<()> {
    let mut csv_writer = ::csv::WriterBuilder::new().has_headers(false).from_writer(writer);

    csv_writer.serialize(tx)?;
    csv_writer.flush()?;

    Ok(())
}

/// Creates an iterator over transactions in a CSV file.
///
/// Automatically skips the header row on the first read.
///
/// # Example
///
/// ```ignore
/// use parser::serde::csv;
/// use std::fs::File;
///
/// let file = File::open("transactions.csv")?;
/// for result in csv::iter_reader(file) {
///     let tx = result?;
///     println!("{:?}", tx);
/// }
/// ```
pub fn iter_reader<R: Read>(reader: R) -> impl Iterator<Item = Result<Transaction>> {
    CsvReaderIterator::new(BufReader::new(reader))
}

/// Creates an iterator from a `BufRead` source (avoids double buffering).
pub fn iter_buf_reader<R: BufRead>(reader: R) -> impl Iterator<Item = Result<Transaction>> {
    CsvReaderIterator::new(reader)
}

/// Iterator adapter for streaming CSV reads.
struct CsvReaderIterator<R> {
    reader: R,
    header_skipped: bool,
    finished: bool,
}

impl<R: BufRead> CsvReaderIterator<R> {
    fn new(reader: R) -> Self {
        Self { reader, header_skipped: false, finished: false }
    }

    fn skip_header(&mut self) -> Result<bool> {
        let mut header = String::new();
        let bytes_read = self.reader.read_line(&mut header)?;
        Ok(bytes_read > 0)
    }
}

impl<R: BufRead> Iterator for CsvReaderIterator<R> {
    type Item = Result<Transaction>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // Skip header on first read
        if !self.header_skipped {
            self.header_skipped = true;
            match self.skip_header() {
                Ok(true) => {} // Header skipped, continue
                Ok(false) => {
                    // Empty file
                    self.finished = true;
                    return None;
                }
                Err(e) => {
                    self.finished = true;
                    return Some(Err(e));
                }
            }
        }

        // Read next transaction
        match read_one(&mut self.reader) {
            Ok(Some(tx)) => Some(Ok(tx)),
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
// Buffered API (for in-memory operations)
// ============================================================================

/// Serializes a transaction to a CSV row string (without header).
///
/// # Example
///
/// ```ignore
/// let row = csv::to_string(&transaction)?;
/// ```
pub fn to_string<T: Serialize>(value: &T) -> Result<String> {
    let mut writer = ::csv::WriterBuilder::new().has_headers(false).from_writer(Vec::new());

    writer.serialize(value)?;
    writer.flush()?;

    let bytes = writer.into_inner().map_err(|e| Error::Io(e.into_error()))?;
    Ok(String::from_utf8(bytes)?)
}

/// Deserializes a transaction from a CSV row string (without header).
///
/// # Example
///
/// ```ignore
/// let row = "1234567890,DEPOSIT,0,9876543210,50000,1700000000000,SUCCESS,\"Test\"";
/// let tx: Transaction = csv::from_str(row)?;
/// ```
pub fn from_str<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T> {
    let mut reader = ::csv::ReaderBuilder::new().has_headers(false).from_reader(s.as_bytes());

    match reader.deserialize().next() {
        Some(Ok(value)) => Ok(value),
        Some(Err(e)) => Err(Error::Csv(e)),
        None => Err(Error::UnexpectedEof),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::transaction::{TransactionStatus, TransactionType};

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
    fn test_to_string() {
        let tx = sample_transaction();
        let row = to_string(&tx).unwrap();

        assert!(row.contains("1234567890"));
        assert!(row.contains("DEPOSIT"));
        assert!(row.contains("9876543210"));
        assert!(row.contains("50000"));
        assert!(row.contains("SUCCESS"));
        assert!(row.contains("Test deposit"));
    }

    #[test]
    fn test_roundtrip() {
        let original = sample_transaction();
        let row = to_string(&original).unwrap();
        let decoded: Transaction = from_str(&row).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_description_with_quotes() {
        let tx = Transaction {
            tx_id: 1,
            tx_type: TransactionType::Transfer,
            from_user_id: 10,
            to_user_id: 20,
            amount: 100,
            timestamp: 1000,
            status: TransactionStatus::Success,
            description: r#"Payment for "services""#.to_string(),
        };

        let row = to_string(&tx).unwrap();
        let decoded: Transaction = from_str(&row).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_description_with_comma() {
        let tx = Transaction {
            tx_id: 1,
            tx_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 42,
            amount: 100,
            timestamp: 1000,
            status: TransactionStatus::Success,
            description: "Hello, World!".to_string(),
        };

        let row = to_string(&tx).unwrap();
        let decoded: Transaction = from_str(&row).unwrap();
        assert_eq!(tx, decoded);
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

        let row = to_string(&tx).unwrap();
        let decoded: Transaction = from_str(&row).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_write_header() {
        let mut buffer = Vec::new();
        write_header(&mut buffer).unwrap();

        let header = String::from_utf8(buffer).unwrap();
        assert_eq!(header.trim(), HEADER);
    }

    #[test]
    fn test_iter_reader_with_header() {
        let csv_data = format!(
            "{}\n1234567890,DEPOSIT,0,9876543210,50000,1700000000000,SUCCESS,Test deposit\n",
            HEADER
        );

        let txs: Vec<Transaction> =
            iter_reader(Cursor::new(csv_data)).collect::<Result<Vec<_>>>().unwrap();

        assert_eq!(txs.len(), 1);
        assert_eq!(txs[0], sample_transaction());
    }

    #[test]
    fn test_iter_reader_multiple_records() {
        let csv_data = format!(
            "{}\n\
             1,DEPOSIT,0,42,100,1000,SUCCESS,First\n\
             2,TRANSFER,42,100,50,2000,PENDING,Second\n",
            HEADER
        );

        let txs: Vec<Transaction> =
            iter_reader(Cursor::new(csv_data)).collect::<Result<Vec<_>>>().unwrap();

        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0].tx_id, 1);
        assert_eq!(txs[0].tx_type, TransactionType::Deposit);
        assert_eq!(txs[1].tx_id, 2);
        assert_eq!(txs[1].tx_type, TransactionType::Transfer);
    }

    #[test]
    fn test_iter_reader_empty_file() {
        let txs: Vec<Transaction> =
            iter_reader(Cursor::new("")).collect::<Result<Vec<_>>>().unwrap();

        assert!(txs.is_empty());
    }

    #[test]
    fn test_iter_reader_header_only() {
        let csv_data = format!("{}\n", HEADER);

        let txs: Vec<Transaction> =
            iter_reader(Cursor::new(csv_data)).collect::<Result<Vec<_>>>().unwrap();

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

        // Write multiple records with header
        let mut buffer = Vec::new();
        write_header(&mut buffer).unwrap();
        write_one(&mut buffer, &tx1).unwrap();
        write_one(&mut buffer, &tx2).unwrap();

        // Read back
        let txs: Vec<Transaction> =
            iter_reader(Cursor::new(buffer)).collect::<Result<Vec<_>>>().unwrap();

        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0], tx1);
        assert_eq!(txs[1], tx2);
    }

    #[test]
    fn test_cyrillic_description() {
        let tx = Transaction {
            tx_id: 1,
            tx_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 42,
            amount: 10000,
            timestamp: 1633036800000,
            status: TransactionStatus::Success,
            description: "Пополнение через терминал".to_string(),
        };

        let row = to_string(&tx).unwrap();
        let decoded: Transaction = from_str(&row).unwrap();
        assert_eq!(tx, decoded);
    }
}
