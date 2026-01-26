//! CLI tool for converting transaction files between Binary, Text, and CSV formats.
//!
//! # Usage
//!
//! ```bash
//! # Convert text to binary
//! converter --input transactions.txt --input-format text --output-format binary --output transactions.bin
//!
//! # Read from stdin, write to stdout
//! cat transactions.txt | converter --input-format text --output-format csv > transactions.csv
//!
//! # Validate by round-trip conversion
//! converter -i data.bin --input-format binary --output-format binary -o validated.bin
//! ```

use std::fs::File;
use std::io::{Read, Write, stdin, stdout};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use parser::prelude::*;

/// Convert transaction files between Binary, Text, and CSV formats.
///
/// Reads transactions from input (file or stdin) and writes them
/// to output (file or stdout) in the specified format.
#[derive(Parser, Debug)]
#[command(name = "converter")]
#[command(version, about)]
struct Args {
    /// Input file path. If not specified, reads from stdin.
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Input format.
    #[arg(long, value_enum)]
    input_format: FormatArg,

    /// Output format.
    #[arg(long, value_enum)]
    output_format: FormatArg,

    /// Output file path. If not specified, writes to stdout.
    #[arg(short, long)]
    output: Option<PathBuf>,
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

    // Open input source
    let input: Box<dyn Read> = match &args.input {
        Some(path) => {
            let file = File::open(path)
                .with_context(|| format!("Failed to open input file: {}", path.display()))?;
            Box::new(file)
        }
        None => Box::new(stdin().lock()),
    };

    // Open output destination
    let output: Box<dyn Write> = match &args.output {
        Some(path) => {
            let file = File::create(path)
                .with_context(|| format!("Failed to create output file: {}", path.display()))?;
            Box::new(file)
        }
        None => Box::new(stdout().lock()),
    };

    // Perform conversion
    let count = convert(input, output, args.input_format.into(), args.output_format.into())?;

    // Report result to stderr (so it doesn't interfere with stdout output)
    eprintln!("Converted {count} transaction(s)");

    Ok(())
}

/// Converts transactions from input to output with runtime format selection.
///
/// Uses compile-time dispatch through marker types for optimal performance.
fn convert<R: Read, W: Write>(
    input: R,
    output: W,
    input_format: Format,
    output_format: Format,
) -> Result<usize> {
    match (input_format, output_format) {
        // Binary -> *
        (Format::Binary, Format::Binary) => convert_typed::<_, _, Binary, Binary>(input, output),
        (Format::Binary, Format::Text) => convert_typed::<_, _, Binary, Text>(input, output),
        (Format::Binary, Format::Csv) => convert_typed::<_, _, Binary, Csv>(input, output),
        // Text -> *
        (Format::Text, Format::Binary) => convert_typed::<_, _, Text, Binary>(input, output),
        (Format::Text, Format::Text) => convert_typed::<_, _, Text, Text>(input, output),
        (Format::Text, Format::Csv) => convert_typed::<_, _, Text, Csv>(input, output),
        // CSV -> *
        (Format::Csv, Format::Binary) => convert_typed::<_, _, Csv, Binary>(input, output),
        (Format::Csv, Format::Text) => convert_typed::<_, _, Csv, Text>(input, output),
        (Format::Csv, Format::Csv) => convert_typed::<_, _, Csv, Csv>(input, output),
    }
}

/// Type-safe streaming conversion using TransactionReader and TransactionWriter.
///
/// Reads transactions one by one from input and writes them to output,
/// ensuring minimal memory usage for large files.
fn convert_typed<R, W, IF, OF>(input: R, output: W) -> Result<usize>
where
    R: Read,
    W: Write,
    IF: SerdeFormat,
    OF: SerdeFormat,
{
    let reader = TransactionReader::<_, IF>::new(input);
    let mut writer = TransactionWriter::<_, OF>::new(output);

    // Write header if the output format requires one (e.g., CSV)
    writer.write_header().context("Failed to write output header")?;

    // Process transactions one by one (streaming)
    for (idx, result) in reader.enumerate() {
        let tx = result.with_context(|| format!("Failed to read transaction #{}", idx + 1))?;
        writer.write(&tx).with_context(|| format!("Failed to write transaction #{}", idx + 1))?;
    }

    // Ensure all buffered data is written
    writer.flush().context("Failed to flush output")?;

    Ok(writer.records_written())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn sample_text_data() -> &'static str {
        r#"TX_ID: 1234567890
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 9876543210
AMOUNT: 50000
TIMESTAMP: 1700000000000
STATUS: SUCCESS
DESCRIPTION: "Test deposit"
"#
    }

    #[test]
    fn test_text_to_csv_conversion() {
        let input = Cursor::new(sample_text_data());
        let mut output = Vec::new();

        let count = convert_typed::<_, _, Text, Csv>(input, &mut output).unwrap();

        assert_eq!(count, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.starts_with("TX_ID,TX_TYPE,"));
        assert!(output_str.contains("1234567890"));
        assert!(output_str.contains("DEPOSIT"));
    }

    #[test]
    fn test_text_to_text_roundtrip() {
        let input = Cursor::new(sample_text_data());
        let mut output = Vec::new();

        let count = convert_typed::<_, _, Text, Text>(input, &mut output).unwrap();

        assert_eq!(count, 1);
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("TX_ID: 1234567890"));
        assert!(output_str.contains("TX_TYPE: DEPOSIT"));
    }

    #[test]
    fn test_empty_input() {
        let input = Cursor::new("");
        let mut output = Vec::new();

        let count = convert_typed::<_, _, Text, Csv>(input, &mut output).unwrap();

        assert_eq!(count, 0);
        // CSV should still have header even for empty input
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.starts_with("TX_ID,TX_TYPE,"));
    }

    #[test]
    fn test_csv_roundtrip() {
        let csv_input =
            format!("{}\n1,DEPOSIT,0,42,100,1000,SUCCESS,Test\n", parser::serde::csv::HEADER);
        let input = Cursor::new(csv_input);
        let mut output = Vec::new();

        let count = convert_typed::<_, _, Csv, Csv>(input, &mut output).unwrap();

        assert_eq!(count, 1);
    }
}
