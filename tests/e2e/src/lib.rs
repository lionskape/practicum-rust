//! # e2e-tests - End-to-end тесты CLI инструментов
//!
//! Этот крейт содержит e2e тесты для CLI инструментов воркспейса:
//! - `converter` — конвертер форматов транзакций
//! - `ypbank_compare` — сравниватель файлов транзакций
//!
//! ## Фикстуры
//!
//! Тестовые файлы расположены в `fixtures/`:
//! - `records_example.bin` — бинарный формат
//! - `records_example.csv` — CSV формат
//! - `records_example.txt` — текстовый формат

use std::path::PathBuf;

/// Получить путь к директории фикстур.
pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// Получить путь к фикстуре по имени файла.
pub fn fixture(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}
