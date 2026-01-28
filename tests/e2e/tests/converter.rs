//! E2E тесты для CLI инструмента `converter`.
//!
//! Тестируем конвертацию между всеми форматами:
//! - binary (YPBankBin)
//! - text (YPBankText)
//! - csv (YPBankCsv)

use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

/// Получить путь к фикстуре.
fn fixture(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures").join(name)
}

/// Создать команду для запуска converter.
///
/// `cargo_bin` deprecated из-за edge case с custom build directories,
/// но это единственный способ для кросс-крейтовых бинарников.
#[expect(deprecated)]
fn converter() -> Command {
    Command::cargo_bin("converter").unwrap()
}

// ============================================================================
// Тесты конвертации: каждый формат → каждый формат
// ============================================================================

#[test]
fn test_binary_to_text() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("output.txt");

    converter()
        .args([
            "--input",
            fixture("records_example.bin").to_str().unwrap(),
            "--input-format",
            "binary",
            "--output-format",
            "text",
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Converted"));

    // Проверяем, что файл создан и содержит данные
    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("TX_ID:"));
    assert!(content.contains("TX_TYPE:"));
}

#[test]
fn test_binary_to_csv() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("output.csv");

    converter()
        .args([
            "--input",
            fixture("records_example.bin").to_str().unwrap(),
            "--input-format",
            "binary",
            "--output-format",
            "csv",
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&output).unwrap();
    // CSV должен начинаться с заголовка
    assert!(content.starts_with("TX_ID,TX_TYPE,"));
}

#[test]
fn test_text_to_binary() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("output.bin");

    converter()
        .args([
            "-i",
            fixture("records_example.txt").to_str().unwrap(),
            "--input-format",
            "text",
            "--output-format",
            "binary",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Binary файл должен начинаться с magic bytes "YPBN"
    let content = fs::read(&output).unwrap();
    assert!(content.starts_with(b"YPBN"));
}

#[test]
fn test_text_to_csv() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("output.csv");

    converter()
        .args([
            "--input",
            fixture("records_example.txt").to_str().unwrap(),
            "--input-format",
            "text",
            "--output-format",
            "csv",
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.starts_with("TX_ID,TX_TYPE,"));
}

#[test]
fn test_csv_to_binary() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("output.bin");

    converter()
        .args([
            "--input",
            fixture("records_example.csv").to_str().unwrap(),
            "--input-format",
            "csv",
            "--output-format",
            "binary",
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = fs::read(&output).unwrap();
    assert!(content.starts_with(b"YPBN"));
}

#[test]
fn test_csv_to_text() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("output.txt");

    converter()
        .args([
            "--input",
            fixture("records_example.csv").to_str().unwrap(),
            "--input-format",
            "csv",
            "--output-format",
            "text",
            "--output",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("TX_ID:"));
}

// ============================================================================
// Round-trip тесты: формат A → формат B → формат A
// Проверяем сохранение данных при конвертации
// ============================================================================

#[test]
fn test_roundtrip_binary_via_text() {
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

    // Сравниваем размеры файлов (должны быть идентичны)
    let original = fs::read(fixture("records_example.bin")).unwrap();
    let converted = fs::read(&final_output).unwrap();
    assert_eq!(
        original.len(),
        converted.len(),
        "Round-trip binary→text→binary должен сохранить размер файла"
    );
}

#[test]
fn test_roundtrip_csv_via_binary() {
    let dir = tempdir().unwrap();
    let intermediate = dir.path().join("intermediate.bin");
    let final_output = dir.path().join("final.csv");

    // csv → binary
    converter()
        .args([
            "--input",
            fixture("records_example.csv").to_str().unwrap(),
            "--input-format",
            "csv",
            "--output-format",
            "binary",
            "--output",
            intermediate.to_str().unwrap(),
        ])
        .assert()
        .success();

    // binary → csv
    converter()
        .args([
            "--input",
            intermediate.to_str().unwrap(),
            "--input-format",
            "binary",
            "--output-format",
            "csv",
            "--output",
            final_output.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Подсчитываем количество строк (должно совпадать)
    let original_lines =
        fs::read_to_string(fixture("records_example.csv")).unwrap().lines().count();
    let converted_lines = fs::read_to_string(&final_output).unwrap().lines().count();
    assert_eq!(
        original_lines, converted_lines,
        "Round-trip csv→binary→csv должен сохранить количество записей"
    );
}

// ============================================================================
// Тесты обработки ошибок
// ============================================================================

#[test]
fn test_missing_input_file() {
    converter()
        .args([
            "--input",
            "/nonexistent/path/to/file.bin",
            "--input-format",
            "binary",
            "--output-format",
            "text",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to open input file"));
}

#[test]
fn test_missing_required_args() {
    // Без --input-format
    converter()
        .args(["--output-format", "text"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--input-format"));
}

// ============================================================================
// Тесты stdin/stdout
// ============================================================================

#[test]
fn test_stdin_to_stdout() {
    let input_data = fs::read(fixture("records_example.bin")).unwrap();

    let output = converter()
        .args(["--input-format", "binary", "--output-format", "text"])
        .write_stdin(input_data)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8(output).unwrap();
    assert!(output_str.contains("TX_ID:"));
}
