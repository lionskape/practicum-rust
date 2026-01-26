//! CLI tool for comparing transaction files in different formats.
//!
//! # Usage
//!
//! ```bash
//! # Compare binary and CSV files
//! ypbank_compare --file1 transactions.bin --format1 binary --file2 transactions.csv --format2 csv
//!
//! # Compare text files
//! ypbank_compare --file1 v1.txt --format1 text --file2 v2.txt --format2 text
//! ```

use std::{collections::HashMap, fs::File, path::PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, ValueEnum};
use parser::prelude::*;

/// Compare transaction records between two files in any supported format.
///
/// Reads both files, parses transactions, and reports differences.
/// Files can be in different formats (Binary, Text, CSV).
#[derive(Parser, Debug)]
#[command(name = "ypbank_compare")]
#[command(version, about)]
struct Args {
    /// First file path.
    #[arg(long)]
    file1: PathBuf,

    /// Format of the first file.
    #[arg(long, value_enum)]
    format1: FormatArg,

    /// Second file path.
    #[arg(long)]
    file2: PathBuf,

    /// Format of the second file.
    #[arg(long, value_enum)]
    format2: FormatArg,
}

/// Supported transaction formats for CLI arguments.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    /// Binary YPBN format (compact, with magic bytes).
    Binary,
    /// Text KEY: VALUE format (human-readable).
    Text,
    /// CSV format with header row.
    Csv,
}

impl From<FormatArg> for Format {
    fn from(arg: FormatArg) -> Self {
        match arg {
            FormatArg::Binary => Format::Binary,
            FormatArg::Text => Format::Text,
            FormatArg::Csv => Format::Csv,
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    // Read transactions from both files
    let file1 = File::open(&args.file1)
        .with_context(|| format!("Failed to open file: {}", args.file1.display()))?;
    let file2 = File::open(&args.file2)
        .with_context(|| format!("Failed to open file: {}", args.file2.display()))?;

    let txs1 = read_transactions(file1, args.format1.into())
        .with_context(|| format!("Failed to read transactions from '{}'", args.file1.display()))?;
    let txs2 = read_transactions(file2, args.format2.into())
        .with_context(|| format!("Failed to read transactions from '{}'", args.file2.display()))?;

    // Compare
    let result = compare_transactions(&txs1, &txs2);

    match result {
        CompareResult::Identical => {
            println!(
                "The transaction records in '{}' and '{}' are identical.",
                args.file1.display(),
                args.file2.display()
            );
        }
        CompareResult::Different(differences) => {
            format_differences(&args, &differences)?;
            bail!(
                "Found {} difference(s) between '{}' and '{}'",
                differences.len(),
                args.file1.display(),
                args.file2.display()
            );
        }
    }

    Ok(())
}

/// Reads all transactions from a file with the given format.
fn read_transactions<R: std::io::Read>(reader: R, format: Format) -> Result<Vec<Transaction>> {
    match format {
        Format::Binary => read_typed::<_, Binary>(reader),
        Format::Text => read_typed::<_, Text>(reader),
        Format::Csv => read_typed::<_, Csv>(reader),
    }
}

/// Type-safe transaction reading using TransactionReader.
fn read_typed<R, F>(reader: R) -> Result<Vec<Transaction>>
where
    R: std::io::Read,
    F: SerdeFormat,
{
    let reader = TransactionReader::<_, F>::new(reader);
    let mut transactions = Vec::new();

    for (idx, result) in reader.enumerate() {
        let tx = result.with_context(|| format!("Failed to read transaction #{}", idx + 1))?;
        transactions.push(tx);
    }

    Ok(transactions)
}

/// Represents a single difference between two transaction lists.
#[derive(Debug)]
enum Difference<'a> {
    /// Transaction with this TX_ID exists only in the first file.
    OnlyInFirst { tx: &'a Transaction },
    /// Transaction with this TX_ID exists only in the second file.
    OnlyInSecond { tx: &'a Transaction },
    /// Transactions with the same TX_ID have different field values.
    Mismatch { tx1: &'a Transaction, tx2: &'a Transaction },
}

/// Result of comparing two transaction lists.
enum CompareResult<'a> {
    /// Both lists contain identical transactions (matched by TX_ID).
    Identical,
    /// Lists differ; contains the list of differences.
    Different(Vec<Difference<'a>>),
}

/// Compares two lists of transactions by TX_ID and returns the result.
///
/// Transactions are matched by their `tx_id` field, not by position in the list.
/// This allows comparing files where transactions may be in different order.
fn compare_transactions<'a>(txs1: &'a [Transaction], txs2: &'a [Transaction]) -> CompareResult<'a> {
    let mut differences = Vec::new();
    let txs1_map: HashMap<u64, &Transaction> = txs1.iter().map(|tx| (tx.tx_id, tx)).collect();
    let txs2_map: HashMap<u64, &Transaction> = txs2.iter().map(|tx| (tx.tx_id, tx)).collect();

    txs1_map.iter().for_each(|(tx_id, tx1)| {
        if let Some(tx2) = txs2_map.get(tx_id) {
            if tx1 != tx2 {
                differences.push(Difference::Mismatch { tx1, tx2 })
            }
        } else {
            differences.push(Difference::OnlyInFirst { tx: tx1 })
        }
    });
    txs2_map.iter().for_each(|(tx_id, tx2)| {
        if !txs1_map.contains_key(tx_id) {
            differences.push(Difference::OnlyInSecond { tx: tx2 })
        }
    });

    if differences.is_empty() {
        CompareResult::Identical
    } else {
        CompareResult::Different(differences)
    }
}

/// Formats and prints differences to stderr.
fn format_differences(args: &Args, differences: &[Difference<'_>]) -> Result<()> {
    eprintln!(
        "Comparing '{}' ({:?}) with '{}' ({:?}):",
        args.file1.display(),
        args.format1,
        args.file2.display(),
        args.format2
    );
    eprintln!();

    for diff in differences {
        match diff {
            Difference::OnlyInFirst { tx } => {
                eprintln!(
                    "Transaction TX_ID={} exists only in '{}':",
                    tx.tx_id,
                    args.file1.display()
                );
                eprintln!("  TX_TYPE: {}", tx.tx_type.as_str());
                eprintln!("  AMOUNT: {}", tx.amount);
                eprintln!();
            }
            Difference::OnlyInSecond { tx } => {
                eprintln!(
                    "Transaction TX_ID={} exists only in '{}':",
                    tx.tx_id,
                    args.file2.display()
                );
                eprintln!("  TX_TYPE: {}", tx.tx_type.as_str());
                eprintln!("  AMOUNT: {}", tx.amount);
                eprintln!();
            }
            Difference::Mismatch { tx1, tx2 } => {
                eprintln!("Transaction TX_ID={} differs:", tx1.tx_id);
                print_field_diff("TX_TYPE", &tx1.tx_type.as_str(), &tx2.tx_type.as_str());
                print_field_diff("FROM_USER_ID", &tx1.from_user_id, &tx2.from_user_id);
                print_field_diff("TO_USER_ID", &tx1.to_user_id, &tx2.to_user_id);
                print_field_diff("AMOUNT", &tx1.amount, &tx2.amount);
                print_field_diff("TIMESTAMP", &tx1.timestamp, &tx2.timestamp);
                print_field_diff("STATUS", &tx1.status.as_str(), &tx2.status.as_str());
                print_field_diff("DESCRIPTION", &tx1.description, &tx2.description);
                eprintln!();
            }
        }
    }

    Ok(())
}

/// Prints a field comparison, only showing if values differ.
fn print_field_diff<T: PartialEq + std::fmt::Display>(name: &str, val1: &T, val2: &T) {
    if val1 != val2 {
        eprintln!("  {}: '{}' vs '{}'", name, val1, val2);
    }
}

#[cfg(test)]
mod tests {
    use parser::transaction::{TransactionStatus, TransactionType};

    use super::*;

    fn sample_transaction(id: u64) -> Transaction {
        Transaction {
            tx_id: id,
            tx_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 12345,
            amount: 1000,
            timestamp: 1700000000000,
            status: TransactionStatus::Success,
            description: "Test".to_string(),
        }
    }

    fn sample_transaction_with_amount(id: u64, amount: i64) -> Transaction {
        Transaction {
            tx_id: id,
            tx_type: TransactionType::Deposit,
            from_user_id: 0,
            to_user_id: 12345,
            amount,
            timestamp: 1700000000000,
            status: TransactionStatus::Success,
            description: "Test".to_string(),
        }
    }

    #[test]
    fn test_identical_transactions() {
        let txs1 = vec![sample_transaction(1), sample_transaction(2)];
        let txs2 = vec![sample_transaction(1), sample_transaction(2)];

        match compare_transactions(&txs1, &txs2) {
            CompareResult::Identical => {}
            CompareResult::Different(_) => panic!("Expected identical"),
        }
    }

    #[test]
    fn test_identical_different_order() {
        // Same transactions but in different order - should be identical
        let txs1 = vec![sample_transaction(1), sample_transaction(2), sample_transaction(3)];
        let txs2 = vec![sample_transaction(3), sample_transaction(1), sample_transaction(2)];

        match compare_transactions(&txs1, &txs2) {
            CompareResult::Identical => {}
            CompareResult::Different(_) => panic!("Expected identical (order should not matter)"),
        }
    }

    #[test]
    fn test_only_in_first() {
        let txs1 = vec![sample_transaction(1), sample_transaction(2)];
        let txs2 = vec![sample_transaction(1)];

        match compare_transactions(&txs1, &txs2) {
            CompareResult::Identical => panic!("Expected different"),
            CompareResult::Different(diffs) => {
                assert_eq!(diffs.len(), 1);
                match &diffs[0] {
                    Difference::OnlyInFirst { tx } => assert_eq!(tx.tx_id, 2),
                    _ => panic!("Expected OnlyInFirst"),
                }
            }
        }
    }

    #[test]
    fn test_only_in_second() {
        let txs1 = vec![sample_transaction(1)];
        let txs2 = vec![sample_transaction(1), sample_transaction(99)];

        match compare_transactions(&txs1, &txs2) {
            CompareResult::Identical => panic!("Expected different"),
            CompareResult::Different(diffs) => {
                assert_eq!(diffs.len(), 1);
                match &diffs[0] {
                    Difference::OnlyInSecond { tx } => assert_eq!(tx.tx_id, 99),
                    _ => panic!("Expected OnlyInSecond"),
                }
            }
        }
    }

    #[test]
    fn test_mismatched_content() {
        // Same TX_ID but different amount
        let txs1 = vec![sample_transaction_with_amount(1, 1000)];
        let txs2 = vec![sample_transaction_with_amount(1, 2000)];

        match compare_transactions(&txs1, &txs2) {
            CompareResult::Identical => panic!("Expected different"),
            CompareResult::Different(diffs) => {
                assert_eq!(diffs.len(), 1);
                match &diffs[0] {
                    Difference::Mismatch { tx1, tx2 } => {
                        assert_eq!(tx1.tx_id, 1);
                        assert_eq!(tx1.amount, 1000);
                        assert_eq!(tx2.amount, 2000);
                    }
                    _ => panic!("Expected Mismatch"),
                }
            }
        }
    }

    #[test]
    fn test_empty_lists() {
        let txs1: Vec<Transaction> = vec![];
        let txs2: Vec<Transaction> = vec![];

        match compare_transactions(&txs1, &txs2) {
            CompareResult::Identical => {}
            CompareResult::Different(_) => panic!("Expected identical for empty lists"),
        }
    }
}
