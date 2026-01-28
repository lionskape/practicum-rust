//! E2E тесты для CLI инструмента `ypbank_compare`.
//!
//! Тестируем сравнение файлов транзакций:
//! - Идентичные файлы (одинаковый формат)
//! - Идентичные файлы (разные форматы)
//! - Файлы с различиями

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

/// Получить путь к фикстуре.
fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}

/// Создать команду для запуска ypbank_compare.
///
/// `cargo_bin` deprecated из-за edge case с custom build directories,
/// но это единственный способ для кросс-крейтовых бинарников.
#[expect(deprecated)]
fn comparer() -> Command {
    Command::cargo_bin("ypbank_compare").unwrap()
}

/// Создать команду для запуска converter.
#[expect(deprecated)]
fn converter() -> Command {
    Command::cargo_bin("converter").unwrap()
}

// ============================================================================
// Тесты сравнения идентичных файлов (один формат)
// ============================================================================

#[test]
fn test_compare_identical_binary() {
    comparer()
        .args([
            "--file1",
            fixture("records_example.bin").to_str().unwrap(),
            "--format1",
            "binary",
            "--file2",
            fixture("records_example.bin").to_str().unwrap(),
            "--format2",
            "binary",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("identical"));
}

#[test]
fn test_compare_identical_text() {
    comparer()
        .args([
            "--file1",
            fixture("records_example.txt").to_str().unwrap(),
            "--format1",
            "text",
            "--file2",
            fixture("records_example.txt").to_str().unwrap(),
            "--format2",
            "text",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("identical"));
}

#[test]
fn test_compare_identical_csv() {
    comparer()
        .args([
            "--file1",
            fixture("records_example.csv").to_str().unwrap(),
            "--format1",
            "csv",
            "--file2",
            fixture("records_example.csv").to_str().unwrap(),
            "--format2",
            "csv",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("identical"));
}

// ============================================================================
// Тесты сравнения идентичных файлов (разные форматы)
// Создаём файлы через converter для гарантии идентичности данных
// ============================================================================

#[test]
fn test_compare_binary_vs_text_via_conversion() {
    let dir = tempdir().unwrap();
    let text_converted = dir.path().join("converted.txt");

    // Конвертируем binary → text
    converter()
        .args([
            "--input",
            fixture("records_example.bin").to_str().unwrap(),
            "--input-format",
            "binary",
            "--output-format",
            "text",
            "--output",
            text_converted.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Сравниваем binary с конвертированным text
    comparer()
        .args([
            "--file1",
            fixture("records_example.bin").to_str().unwrap(),
            "--format1",
            "binary",
            "--file2",
            text_converted.to_str().unwrap(),
            "--format2",
            "text",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("identical"));
}

#[test]
fn test_compare_binary_vs_csv_via_conversion() {
    let dir = tempdir().unwrap();
    let csv_converted = dir.path().join("converted.csv");

    // Конвертируем binary → csv
    converter()
        .args([
            "--input",
            fixture("records_example.bin").to_str().unwrap(),
            "--input-format",
            "binary",
            "--output-format",
            "csv",
            "--output",
            csv_converted.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Сравниваем binary с конвертированным csv
    comparer()
        .args([
            "--file1",
            fixture("records_example.bin").to_str().unwrap(),
            "--format1",
            "binary",
            "--file2",
            csv_converted.to_str().unwrap(),
            "--format2",
            "csv",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("identical"));
}

#[test]
fn test_compare_text_vs_csv_via_conversion() {
    let dir = tempdir().unwrap();
    let csv_converted = dir.path().join("converted.csv");

    // Конвертируем text → csv
    converter()
        .args([
            "--input",
            fixture("records_example.txt").to_str().unwrap(),
            "--input-format",
            "text",
            "--output-format",
            "csv",
            "--output",
            csv_converted.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Сравниваем text с конвертированным csv
    comparer()
        .args([
            "--file1",
            fixture("records_example.txt").to_str().unwrap(),
            "--format1",
            "text",
            "--file2",
            csv_converted.to_str().unwrap(),
            "--format2",
            "csv",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("identical"));
}

// ============================================================================
// Тесты сравнения файлов с различиями
// ============================================================================

#[test]
fn test_compare_different_files() {
    let dir = tempdir().unwrap();

    // Создаём файл с одной транзакцией
    let file1 = dir.path().join("file1.txt");
    fs::write(
        &file1,
        r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 100
AMOUNT: 1000
TIMESTAMP: 1700000000000
STATUS: SUCCESS
DESCRIPTION: "Test"
"#,
    )
    .unwrap();

    // Создаём файл с другой транзакцией (другой TX_ID)
    let file2 = dir.path().join("file2.txt");
    fs::write(
        &file2,
        r#"TX_ID: 2
TX_TYPE: WITHDRAWAL
FROM_USER_ID: 100
TO_USER_ID: 0
AMOUNT: 500
TIMESTAMP: 1700000001000
STATUS: SUCCESS
DESCRIPTION: "Another test"
"#,
    )
    .unwrap();

    comparer()
        .args([
            "--file1",
            file1.to_str().unwrap(),
            "--format1",
            "text",
            "--file2",
            file2.to_str().unwrap(),
            "--format2",
            "text",
        ])
        .assert()
        .failure() // Выходит с ошибкой при различиях
        .stderr(predicate::str::contains("difference"));
}

#[test]
fn test_compare_same_id_different_amount() {
    let dir = tempdir().unwrap();

    // Транзакция с amount=1000
    let file1 = dir.path().join("file1.txt");
    fs::write(
        &file1,
        r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 100
AMOUNT: 1000
TIMESTAMP: 1700000000000
STATUS: SUCCESS
DESCRIPTION: "Test"
"#,
    )
    .unwrap();

    // Та же транзакция, но amount=2000
    let file2 = dir.path().join("file2.txt");
    fs::write(
        &file2,
        r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 100
AMOUNT: 2000
TIMESTAMP: 1700000000000
STATUS: SUCCESS
DESCRIPTION: "Test"
"#,
    )
    .unwrap();

    comparer()
        .args([
            "--file1",
            file1.to_str().unwrap(),
            "--format1",
            "text",
            "--file2",
            file2.to_str().unwrap(),
            "--format2",
            "text",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("AMOUNT"))
        .stderr(predicate::str::contains("1000"))
        .stderr(predicate::str::contains("2000"));
}

#[test]
fn test_compare_missing_transaction() {
    let dir = tempdir().unwrap();

    // Два транзакции
    let file1 = dir.path().join("file1.txt");
    fs::write(
        &file1,
        r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 100
AMOUNT: 1000
TIMESTAMP: 1700000000000
STATUS: SUCCESS
DESCRIPTION: "First"

TX_ID: 2
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 200
AMOUNT: 2000
TIMESTAMP: 1700000001000
STATUS: SUCCESS
DESCRIPTION: "Second"
"#,
    )
    .unwrap();

    // Только одна транзакция
    let file2 = dir.path().join("file2.txt");
    fs::write(
        &file2,
        r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 100
AMOUNT: 1000
TIMESTAMP: 1700000000000
STATUS: SUCCESS
DESCRIPTION: "First"
"#,
    )
    .unwrap();

    comparer()
        .args([
            "--file1",
            file1.to_str().unwrap(),
            "--format1",
            "text",
            "--file2",
            file2.to_str().unwrap(),
            "--format2",
            "text",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("TX_ID=2"))
        .stderr(predicate::str::contains("only in"));
}

// ============================================================================
// Тесты обработки ошибок
// ============================================================================

#[test]
fn test_missing_file() {
    comparer()
        .args([
            "--file1",
            "/nonexistent/path/to/file.bin",
            "--format1",
            "binary",
            "--file2",
            fixture("records_example.bin").to_str().unwrap(),
            "--format2",
            "binary",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to open file"));
}

#[test]
fn test_missing_required_args() {
    // Без --format1
    comparer()
        .args([
            "--file1",
            fixture("records_example.bin").to_str().unwrap(),
            "--file2",
            fixture("records_example.bin").to_str().unwrap(),
            "--format2",
            "binary",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--format1"));
}

// ============================================================================
// Тесты с конвертированными файлами
// Создаём файл через converter и сравниваем с оригиналом
// ============================================================================

#[test]
fn test_compare_converted_file_with_original() {
    let dir = tempdir().unwrap();
    let converted = dir.path().join("converted.csv");

    // Конвертируем binary → csv
    converter()
        .args([
            "--input",
            fixture("records_example.bin").to_str().unwrap(),
            "--input-format",
            "binary",
            "--output-format",
            "csv",
            "--output",
            converted.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Сравниваем оригинальный binary с конвертированным csv
    comparer()
        .args([
            "--file1",
            fixture("records_example.bin").to_str().unwrap(),
            "--format1",
            "binary",
            "--file2",
            converted.to_str().unwrap(),
            "--format2",
            "csv",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("identical"));
}

#[test]
fn test_compare_after_roundtrip() {
    let dir = tempdir().unwrap();
    let intermediate = dir.path().join("intermediate.txt");
    let final_output = dir.path().join("final.bin");

    // binary → text
    converter()
        .args([
            "--input",
            fixture("records_example.bin").to_str().unwrap(),
            "--input-format",
            "binary",
            "--output-format",
            "text",
            "--output",
            intermediate.to_str().unwrap(),
        ])
        .assert()
        .success();

    // text → binary
    converter()
        .args([
            "--input",
            intermediate.to_str().unwrap(),
            "--input-format",
            "text",
            "--output-format",
            "binary",
            "--output",
            final_output.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Сравниваем оригинал с результатом round-trip
    comparer()
        .args([
            "--file1",
            fixture("records_example.bin").to_str().unwrap(),
            "--format1",
            "binary",
            "--file2",
            final_output.to_str().unwrap(),
            "--format2",
            "binary",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("identical"));
}
