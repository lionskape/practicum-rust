//! Common types and protocol constants shared between quote-server and quote-client.

use serde::{Deserialize, Serialize};

// ──────────────────────────────────────────────
// Protocol constants
// ──────────────────────────────────────────────

/// Command prefix for the streaming request.
pub const CMD_STREAM: &str = "STREAM";

/// Response prefix for a successful handshake.
pub const RESP_OK: &str = "OK";

/// Response prefix for an error during handshake.
pub const RESP_ERR: &str = "ERR";

/// PING payload sent from client to server over UDP.
pub const PING_PAYLOAD: &[u8; 4] = b"PING";

/// How often the client must send PING (seconds).
pub const PING_INTERVAL_SECS: u64 = 2;

/// Server drops the client after this many seconds without a PING.
pub const PING_TIMEOUT_SECS: u64 = 5;

/// Interval between quote generation batches (milliseconds).
pub const GENERATION_INTERVAL_MS: u64 = 100;

// ──────────────────────────────────────────────
// Data types
// ──────────────────────────────────────────────

/// A single stock quote sent over the wire as JSON via UDP.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StockQuote {
    pub ticker: String,
    pub price: f64,
    pub volume: u64,
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
}

// ──────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────

/// Protocol-level errors that can occur during handshake or data exchange.
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
