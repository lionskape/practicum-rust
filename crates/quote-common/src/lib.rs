//! Общие типы и константы протокола, разделяемые между quote-server и quote-client.

use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────
// Константы протокола
// ──────────────────────────────────────────────

/// Префикс команды подписки на поток котировок.
pub const CMD_STREAM: &str = "STREAM";

/// Префикс ответа при успешном рукопожатии.
pub const RESP_OK: &str = "OK";

/// Префикс ответа при ошибке рукопожатия.
pub const RESP_ERR: &str = "ERR";

/// PING-пакет, отправляемый клиентом серверу по UDP.
pub const PING_PAYLOAD: &[u8; 4] = b"PING";

/// Интервал отправки PING клиентом (секунды).
pub const PING_INTERVAL_SECS: u64 = 2;

/// Сервер отключает клиента после стольких секунд без PING.
pub const PING_TIMEOUT_SECS: u64 = 5;

/// Интервал между генерациями пакетов котировок (миллисекунды).
pub const GENERATION_INTERVAL_MS: u64 = 100;

/// Максимальный размер буфера одной UDP-датаграммы (байты).
pub const UDP_BUF_SIZE: usize = 4096;

// ──────────────────────────────────────────────
// Типы данных
// ──────────────────────────────────────────────

/// Одна котировка акции, передаваемая по сети в формате JSON через UDP.
///
/// # Примеры
///
/// ```
/// use quote_common::StockQuote;
///
/// let quote =
///     StockQuote { ticker: "AAPL".into(), price: 150.25, volume: 1200, timestamp: 1708617600000 };
///
/// let json = serde_json::to_string(&quote).unwrap();
/// let parsed: StockQuote = serde_json::from_str(&json).unwrap();
/// assert_eq!(parsed, quote);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StockQuote {
    pub ticker: String,
    pub price: f64,
    pub volume: u64,
    /// Временна́я метка Unix в миллисекундах.
    pub timestamp: u64,
}

// ──────────────────────────────────────────────
// Ошибки
// ──────────────────────────────────────────────

/// Ошибки уровня протокола, возникающие при рукопожатии или обмене данными.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("invalid command format: {0}")]
    InvalidCommand(String),

    #[error("unknown ticker: {0}")]
    UnknownTicker(String),

    #[error("invalid UDP address: {0}")]
    InvalidAddress(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_quote_serializes_to_json() {
        let quote = StockQuote {
            ticker: "AAPL".into(),
            price: 187.42,
            volume: 3421,
            timestamp: 1708617600000,
        };
        let json = serde_json::to_string(&quote).unwrap();
        assert!(json.contains("\"ticker\":\"AAPL\""));
        assert!(json.contains("\"price\":187.42"));
    }

    #[test]
    fn stock_quote_deserializes_from_json() {
        let json = r#"{"ticker":"TSLA","price":242.5,"volume":10000,"timestamp":1708617600000}"#;
        let quote: StockQuote = serde_json::from_str(json).unwrap();
        assert_eq!(quote.ticker, "TSLA");
        assert!((quote.price - 242.5).abs() < f64::EPSILON);
    }
}
