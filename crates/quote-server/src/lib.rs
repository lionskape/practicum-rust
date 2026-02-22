//! Библиотека quote-server — генерация и потоковая передача котировок по UDP.

extern crate core;

use std::sync::LazyLock;

pub mod client_sender;
pub mod generator;
pub mod protocol;

/// Все известные тикеры, встроенные из `tickers.txt` на этапе компиляции.
const TICKERS_RAW: &str = include_str!("tickers.txt");

/// Распарсенный список тикеров, инициализируется один раз при первом обращении.
static ALL_TICKERS: LazyLock<Vec<String>> = LazyLock::new(|| {
    TICKERS_RAW.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect()
});

/// Возвращает срез всех известных тикеров (непустые строки из `tickers.txt`).
///
/// Список парсится из встроенного файла один раз и кешируется на всё время
/// работы процесса.
///
/// # Примеры
///
/// ```
/// let tickers = quote_server::all_tickers();
/// assert!(!tickers.is_empty());
/// assert!(tickers.contains(&"AAPL".to_string()));
/// ```
pub fn all_tickers() -> &'static [String] {
    &ALL_TICKERS
}
